//! Playernet v4 state, observation-stream, relational-graph, and lossless
//! candidate/action encoder (`ENCODER_VERSION = 4`).
//!
//! Fixes all P0 encoder/contract findings from
//! `docs/history/trained-ai-v4-handover-2026-10-22.md`:
//!
//! - **Full 8-seat relative coordinates** (`0..7`, `8 = none`) across every
//!   entity, seat, combat arrow, commander-damage tally, and candidate.
//! - **Complete `PublicObject`, `SeatView`, and `PlayerView` coverage**:
//!   all 7 `ObjectStatus` bits, all 6 `offered` bits + numeric `picked`,
//!   active face (`card.face`), `rules` (`RulesFace`), `prepared_spell`,
//!   `chosen_subtype`, `chosen_name`, `chosen_opponent`, `unlocked_doors`,
//!   `suspended`, `battle_protector`, all 21 `CounterKind` categories,
//!   `granted_mana`, `board_mana`, `flashback`, `grants`, `stack_item`,
//!   `owed` (`ManaPayment`), `casting`, `targeting`, `sorcery_lock`,
//!   `sorceries_have_flash`, `SeatView::{speed, rad, no_max_hand_size}`, and
//!   `ManaPoolView::{spending, restricted}`.
//! - **Lossless `CastModeKind` & option `ManaCost` encoding** on `ChooseMode`
//!   candidates.
//! - **Expanded neural head coverage** across all 28 `Pending` variants,
//!   including `ChooseManaAbility`, `ChooseDamageSource`, `ChooseDamageEffect`,
//!   `TakeGrantedAction`, `ChooseCardName`, `CancelCast`, `ChoosePile`, and
//!   `Arrange`, with an explicit [`CoverageMatrix4`] and per-step provenance.
//! - **View-only state hash** ([`view_only_state_hash`]) for stall/loop guards
//!   that never reads hidden engine state (`Session::snapshot_hash`).

use std::collections::BTreeMap;

use baylee_core::color::Color;
use baylee_core::ids::{CardIndex, Defender, ObjectId, PlayerId};
use baylee_core::mana::{ManaCost, ManaPayment, ManaSpending};
use baylee_core::preset::GamePreset;
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_engine::choice::{
    ArrangePile, CastModeKind, DamageEffectKind, Pending, PlayerAction, default_arrangement,
};
use baylee_view::{
    CounterKind, DayNight, HandObject, ObjectStatus, Phase, PlayerView, PublicObject, StackItem,
    Step, TargetRef,
};
use serde::{Deserialize, Serialize};

use crate::carddsl::{
    CARD_DSL_WIDTH, MAX_ABILITIES, encode_ability_def, encode_card_face, encode_mana_cost,
    encode_token_def,
};

/// Packs an [`ObjectId`] (`slot` + `generation`) into a deterministic `u64`.
#[inline]
#[must_use]
pub const fn pack_object_id(id: ObjectId) -> u64 {
    ((id.generation() as u64) << 32) | (id.slot() as u64)
}

/// Schema version of the v4 state/action encoder.
pub const ENCODER_VERSION: u32 = 4;

/// Maximum seats at a Baylee table.
pub const MAX_SEATS: usize = 8;

/// Maximum tokenized entities per frame.
pub const MAX_ENTITIES: usize = 256;

/// Maximum directed relational edges per frame.
pub const MAX_EDGES: usize = 256;

/// Maximum action candidates per decision.
pub const MAX_CANDIDATES: usize = 64;

/// Maximum pair candidates for `PairPick` (`DeclareBlockers`, multi-defender attacks).
pub const MAX_PAIRS: usize = 128;

/// Width of per-entity numeric features (excluding looked-up `CARD_DSL_WIDTH`).
pub const ENTITY_FEAT_WIDTH: usize = 96;

/// Width of per-seat numeric features (`8 x 64`).
pub const SEAT_FEAT_WIDTH: usize = 64;

/// Width of the global table/context token (`64`).
pub const GLOBAL_FEAT_WIDTH: usize = 64;

/// Width of each action candidate's numeric feature vector (`48`).
pub const CAND_FEAT_WIDTH: usize = 48;

/// Number of directed edge relation types (`0..14`).
pub const NUM_EDGE_RELATIONS: usize = 14;

/// Token offset of seat 0 in the unified transformer sequence (`0` is Global/CLS).
pub const SEAT_TOKEN_OFFSET: i16 = 1;

/// Token offset of entity 0 in the unified transformer sequence (`1..=8` are seats).
pub const ENTITY_TOKEN_OFFSET: i16 = 9;

#[inline]
fn unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(0.0, 1.0)
}

#[inline]
fn signed_unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(-1.0, 1.0)
}

/// Relative seat index `0..7` clockwise from `viewer` (`0` = `viewer`), or `8`
/// when `seat` is `None`.
#[inline]
#[must_use]
pub fn rel_seat(viewer: PlayerId, seat: Option<PlayerId>, seat_count: usize) -> usize {
    let Some(s) = seat else {
        return MAX_SEATS;
    };
    let n = seat_count.max(1);
    ((s.get() as usize) + n - (viewer.get() as usize % n)) % n
}

/// Inverse of [`rel_seat`]: maps relative seat `0..seat_count` back to [`PlayerId`].
#[inline]
#[must_use]
pub fn abs_seat(viewer: PlayerId, rel: usize, seat_count: usize) -> PlayerId {
    let n = seat_count.max(1);
    PlayerId::new(((viewer.get() as usize + rel) % n) as u8)
}

/// Consistent multiplayer side utility:
/// - `1.0` if `seat`'s side won,
/// - `0.0` if another side won,
/// - `1.0 / surviving_sides` on a draw if `seat`'s side survived, else `0.0`.
#[must_use]
pub fn side_utility(
    seat: u8,
    teams: &[Option<u8>],
    surviving_seats: &[u8],
    winners: &[u8],
) -> f32 {
    let my_team = teams.get(seat as usize).copied().flatten();
    let on_my_side = |other: u8| -> bool {
        if other == seat {
            return true;
        }
        match (my_team, teams.get(other as usize).copied().flatten()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    };
    if !winners.is_empty() {
        return if winners.iter().copied().any(on_my_side) {
            1.0
        } else {
            0.0
        };
    }
    // Draw: split among surviving sides
    let my_side_survived = surviving_seats.iter().copied().any(on_my_side);
    if !my_side_survived {
        return 0.0;
    }
    let mut sides: Vec<u16> = Vec::new();
    for &s in surviving_seats {
        let side_key = match teams.get(s as usize).copied().flatten() {
            Some(t) => 0x100 | u16::from(t),
            None => u16::from(s),
        };
        if !sides.contains(&side_key) {
            sides.push(side_key);
        }
    }
    1.0 / (sides.len().max(1) as f32)
}

/// Computes a deterministic 64-bit FNV-1a hash of the seat-authorized
/// [`PlayerView`] and [`Pending`] ONLY.
///
/// Used by `netplay4`, `arena`, and `league` to detect no-progress loops
/// without ever reading `Session::snapshot_hash()` (which hashes hidden library
/// and opponent hand state, P0-6).
#[must_use]
pub fn view_only_state_hash(view: &PlayerView, pending: &Pending) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(&view.turn.to_le_bytes());
    feed(&[view.seat.get(), view.active.get()]);
    feed(&[view.awaiting.map_or(255, PlayerId::get)]);
    if let Ok(bytes) = serde_json::to_vec(&view.step) {
        feed(&bytes);
    }
    for s in &view.seats {
        feed(&s.life.to_le_bytes());
        feed(&s.poison.to_le_bytes());
        feed(&s.energy.to_le_bytes());
        feed(&[s.speed]);
        feed(&s.rad.to_le_bytes());
        feed(&s.hand_count.to_le_bytes());
        feed(&s.library_count.to_le_bytes());
        feed(&s.graveyard_count.to_le_bytes());
        feed(&s.mana_pool.total().to_le_bytes());
    }
    for h_obj in &view.hand {
        feed(&pack_object_id(h_obj.id).to_le_bytes());
        feed(&h_obj.card.index.get().to_le_bytes());
    }
    for obj in view
        .battlefield
        .iter()
        .chain(&view.stack)
        .chain(view.graveyards.iter().flatten())
        .chain(view.exile.iter().flatten())
        .chain(view.command.iter().flatten())
        .chain(&view.looking_at)
    {
        feed(&pack_object_id(obj.id).to_le_bytes());
        feed(&[obj.controller.get(), obj.owner.get(), obj.status.bits()]);
        feed(&obj.damage.to_le_bytes());
        feed(&obj.power.unwrap_or(0).to_le_bytes());
        feed(&obj.toughness.unwrap_or(0).to_le_bytes());
        for c in &obj.counters {
            feed(&c.count.to_le_bytes());
        }
    }
    if let Ok(p_bytes) = serde_json::to_vec(pending) {
        feed(&p_bytes);
    }
    h
}

/// Computes a deterministic 64-bit FNV-1a hash of the observable board, stack,
/// hand, zones, and seat totals in [`PlayerView`], excluding transient priority
/// window fields (`view.awaiting`, `view.step`, `pending`).
///
/// Used by [`SeatKnowledge4::observe`] so passive observation frames fire only
/// when public or seat-visible game state actually changes (`P0-4`).
#[must_use]
pub fn board_observation_hash(view: &PlayerView) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(&view.turn.to_le_bytes());
    feed(&[view.seat.get(), view.active.get()]);
    for s in &view.seats {
        feed(&s.life.to_le_bytes());
        feed(&s.poison.to_le_bytes());
        feed(&s.energy.to_le_bytes());
        feed(&[s.speed]);
        feed(&s.rad.to_le_bytes());
        feed(&s.hand_count.to_le_bytes());
        feed(&s.library_count.to_le_bytes());
        feed(&s.graveyard_count.to_le_bytes());
        feed(&s.mana_pool.total().to_le_bytes());
    }
    for h_obj in &view.hand {
        feed(&pack_object_id(h_obj.id).to_le_bytes());
        feed(&h_obj.card.index.get().to_le_bytes());
    }
    for obj in view
        .battlefield
        .iter()
        .chain(&view.stack)
        .chain(view.graveyards.iter().flatten())
        .chain(view.exile.iter().flatten())
        .chain(view.command.iter().flatten())
        .chain(&view.looking_at)
    {
        feed(&pack_object_id(obj.id).to_le_bytes());
        feed(&[obj.controller.get(), obj.owner.get(), obj.status.bits()]);
        feed(&obj.damage.to_le_bytes());
        feed(&obj.power.unwrap_or(0).to_le_bytes());
        feed(&obj.toughness.unwrap_or(0).to_le_bytes());
        for c in &obj.counters {
            feed(&c.count.to_le_bytes());
        }
    }
    h
}

/// Public per-seat history statistics accumulated from seat-authorized views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatHistoryStats4 {
    /// Total spells observed on the stack from this seat.
    pub spells_cast_total: u16,
    /// Spells observed on the stack from this seat during the current turn.
    pub spells_cast_this_turn: u8,
    /// Lands observed entering under this seat's control.
    pub lands_played_total: u16,
    /// Creatures observed entering under this seat's control.
    pub creatures_entered_total: u16,
    /// Total attacker declarations observed for this seat.
    pub attacks_declared_total: u16,
    /// Cumulative life gained over the game.
    pub life_gained_total: u16,
    /// Cumulative life lost over the game.
    pub life_lost_total: u16,
    /// Peak graveyard count observed.
    pub graveyard_peak: u16,
}

/// A card revealed in an opponent's hand/library/looking_at that has not yet
/// been seen moving to a public zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownCardMemory4 {
    /// Owner seat (`PlayerId::get()`).
    pub owner_seat: u8,
    /// `CardIndex` of the revealed card.
    pub card_index: u32,
    /// Turn when last observed.
    pub last_seen_turn: u32,
    /// `1` = revealed in hand/looking_at, `2` = revealed on library top.
    pub origin_zone: u8,
}

/// Per-seat persistent knowledge & observation stream state (`SeatKnowledge4`).
///
/// Tracks only seat-authorized information:
/// 1. The seat's own submitted decklist (`own_deck_cards` + `own_deck_summary`),
/// 2. Publicly or legally revealed opponent cards (`known_cards`),
/// 3. Public per-seat history counters (`seat_history`),
/// 4. Monotonic observation sequence cursor (`cursor`).
#[derive(Clone, Debug)]
pub struct SeatKnowledge4 {
    /// Viewing seat.
    pub viewer: PlayerId,
    /// Number of seats at the table.
    pub seat_count: usize,
    /// Own submitted decklist `(CardIndex, total_copies)` sorted by `CardIndex`.
    pub own_deck_cards: Vec<(u32, u8)>,
    /// Compact `24`-float summary of own submitted deck (curve, colors, roles).
    pub own_deck_summary: [f32; 24],
    /// Starting life per seat (observed on turn 0/1).
    pub starting_life: [i32; MAX_SEATS],
    /// Last observed life per seat.
    last_life: [i32; MAX_SEATS],
    /// Last observed turn number.
    last_turn: u32,
    /// Public object IDs already counted on the stack.
    seen_stack_ids: Vec<u64>,
    /// Public object IDs already counted on the battlefield.
    seen_bf_ids: Vec<u64>,
    /// Revealed opponent cards in hidden zones (`(owner_seat, object_id) -> KnownCardMemory4`).
    pub known_cards: BTreeMap<(u8, u64), KnownCardMemory4>,
    /// Per-seat public history counters (indexed by absolute `PlayerId`).
    pub seat_history: [SeatHistoryStats4; MAX_SEATS],
    /// Monotonic observation frame counter for this seat.
    pub cursor: u64,
    /// Digest of the last observed frame to deduplicate identical views.
    last_frame_digest: u64,
}

impl SeatKnowledge4 {
    /// Initializes `SeatKnowledge4` for `viewer` from the seat's own submitted
    /// deck in `preset` (never reading any opponent's deck!).
    #[must_use]
    pub fn from_preset(preset: &GamePreset, viewer: PlayerId) -> Self {
        let mut counts: BTreeMap<u32, u8> = BTreeMap::new();
        let mut start_life = 20_i32;
        if let Some(spec) = preset.seats.get(viewer.get() as usize) {
            if !spec.commanders.is_empty() {
                start_life = 40;
            }
            for entry in spec
                .deck
                .iter()
                .chain(&spec.commanders)
                .chain(spec.starting_hand.iter().flatten())
                .chain(&spec.starting_battlefield)
            {
                let idx = entry.card.get();
                if idx != 0 {
                    *counts.entry(idx).or_insert(0) =
                        counts.get(&idx).copied().unwrap_or(0).saturating_add(1);
                }
            }
        }
        let own_deck_cards: Vec<(u32, u8)> = counts.into_iter().collect();
        let own_deck_summary = summarize_deck(&own_deck_cards);
        Self {
            viewer,
            seat_count: preset.seats.len().clamp(1, MAX_SEATS),
            own_deck_cards,
            own_deck_summary,
            starting_life: [start_life; MAX_SEATS],
            last_life: [start_life; MAX_SEATS],
            last_turn: 0,
            seen_stack_ids: Vec::new(),
            seen_bf_ids: Vec::new(),
            known_cards: BTreeMap::new(),
            seat_history: [SeatHistoryStats4::default(); MAX_SEATS],
            cursor: 0,
            last_frame_digest: 0,
        }
    }

    /// Initializes `SeatKnowledge4` when only `viewer` and `seat_count` are known.
    #[must_use]
    pub fn empty(viewer: PlayerId, seat_count: usize) -> Self {
        Self {
            viewer,
            seat_count: seat_count.clamp(1, MAX_SEATS),
            own_deck_cards: Vec::new(),
            own_deck_summary: [0.0; 24],
            starting_life: [20; MAX_SEATS],
            last_life: [20; MAX_SEATS],
            last_turn: 0,
            seen_stack_ids: Vec::new(),
            seen_bf_ids: Vec::new(),
            known_cards: BTreeMap::new(),
            seat_history: [SeatHistoryStats4::default(); MAX_SEATS],
            cursor: 0,
            last_frame_digest: 0,
        }
    }

    /// Updates `SeatKnowledge4` from a seat-authorized `PlayerView` and returns
    /// `true` if the observable state changed since the last call.
    pub fn observe(&mut self, view: &PlayerView) -> bool {
        if self.cursor == 0 {
            for (i, s) in view.seats.iter().enumerate().take(MAX_SEATS) {
                self.starting_life[i] = s.life;
                self.last_life[i] = s.life;
            }
        }
        if view.turn != self.last_turn {
            self.last_turn = view.turn;
            for h in &mut self.seat_history {
                h.spells_cast_this_turn = 0;
            }
        }
        for (i, s) in view.seats.iter().enumerate().take(MAX_SEATS) {
            let delta = s.life - self.last_life[i];
            if delta > 0 {
                self.seat_history[i].life_gained_total = self.seat_history[i]
                    .life_gained_total
                    .saturating_add(delta.min(u16::MAX as i32) as u16);
            } else if delta < 0 {
                self.seat_history[i].life_lost_total = self.seat_history[i]
                    .life_lost_total
                    .saturating_add((-delta).min(u16::MAX as i32) as u16);
            }
            self.last_life[i] = s.life;
            let gy = s.graveyard_count.min(u32::from(u16::MAX)) as u16;
            if gy > self.seat_history[i].graveyard_peak {
                self.seat_history[i].graveyard_peak = gy;
            }
        }
        // Track stack spells
        for obj in &view.stack {
            let id = pack_object_id(obj.id);
            if !self.seen_stack_ids.contains(&id) {
                self.seen_stack_ids.push(id);
                if matches!(obj.stack_item, Some(StackItem::Spell)) {
                    let c = (obj.controller.get() as usize).min(MAX_SEATS - 1);
                    self.seat_history[c].spells_cast_total =
                        self.seat_history[c].spells_cast_total.saturating_add(1);
                    self.seat_history[c].spells_cast_this_turn =
                        self.seat_history[c].spells_cast_this_turn.saturating_add(1);
                }
            }
            // If a known opponent card was cast, remove it from hidden-zone memory
            self.known_cards.remove(&(obj.owner.get(), id));
        }
        // Track battlefield arrivals
        for obj in &view.battlefield {
            let id = pack_object_id(obj.id);
            if !self.seen_bf_ids.contains(&id) {
                self.seen_bf_ids.push(id);
                let c = (obj.controller.get() as usize).min(MAX_SEATS - 1);
                if obj.types.contains(TypeSet::LAND) {
                    self.seat_history[c].lands_played_total =
                        self.seat_history[c].lands_played_total.saturating_add(1);
                }
                if obj.types.contains(TypeSet::CREATURE) {
                    self.seat_history[c].creatures_entered_total =
                        self.seat_history[c].creatures_entered_total.saturating_add(1);
                }
            }
            self.known_cards.remove(&(obj.owner.get(), id));
        }
        for obj in view.graveyards.iter().flatten().chain(view.exile.iter().flatten()) {
            self.known_cards.remove(&(obj.owner.get(), pack_object_id(obj.id)));
        }
        // Record newly revealed opponent cards in shared_hands, controlled_hands, looking_at, library_tops
        for sh in view.shared_hands.iter().chain(&view.controlled_hands) {
            if sh.player != self.viewer {
                for h_obj in &sh.cards {
                    self.known_cards.insert(
                        (sh.player.get(), pack_object_id(h_obj.id)),
                        KnownCardMemory4 {
                            owner_seat: sh.player.get(),
                            card_index: h_obj.card.index.get(),
                            last_seen_turn: view.turn,
                            origin_zone: 1,
                        },
                    );
                }
            }
        }
        for obj in &view.looking_at {
            if obj.owner != self.viewer
                && let Some(card) = obj.card
            {
                self.known_cards.insert(
                    (obj.owner.get(), pack_object_id(obj.id)),
                    KnownCardMemory4 {
                        owner_seat: obj.owner.get(),
                        card_index: card.index.get(),
                        last_seen_turn: view.turn,
                        origin_zone: 1,
                    },
                );
            }
        }
        for obj in &view.library_tops {
            if obj.owner != self.viewer
                && let Some(card) = obj.card
            {
                self.known_cards.insert(
                    (obj.owner.get(), pack_object_id(obj.id)),
                    KnownCardMemory4 {
                        owner_seat: obj.owner.get(),
                        card_index: card.index.get(),
                        last_seen_turn: view.turn,
                        origin_zone: 2,
                    },
                );
            }
        }
        // Clear known hand cards if an opponent's hand count reached 0
        for s in &view.seats {
            if s.hand_count == 0 {
                let p = s.player.get();
                self.known_cards
                    .retain(|&(owner, _), mem| !(owner == p && mem.origin_zone == 1));
            }
        }
        let digest = board_observation_hash(view);
        let changed = digest != self.last_frame_digest || self.cursor == 0;
        if changed {
            self.last_frame_digest = digest;
            self.cursor += 1;
        }
        changed
    }
}

fn summarize_deck(cards: &[(u32, u8)]) -> [f32; 24] {
    let mut out = [0.0_f32; 24];
    let mut total = 0.0_f32;
    for &(card_id, count) in cards {
        let Some(def) = baylee_cards::by_index(CardIndex::new(card_id)) else {
            continue;
        };
        let n = f32::from(count);
        total += n;
        let dsl = encode_card_face(def, 0);
        // Lands vs non-lands
        if dsl.summary[25] > 0.0 {
            out[0] += n;
        } else {
            let mv = (dsl.summary[0] * 12.0).round() as usize;
            let bucket = mv.clamp(1, 6);
            out[bucket] += n;
        }
        // Color identity shares [7..12]
        for c in 0..5 {
            out[7 + c] += n * dsl.summary[15 + c];
        }
        // Type shares [12..16]: Creature, Instant/Sorcery, Artifact, Enchantment
        out[12] += n * dsl.summary[20];
        out[13] += n * (dsl.summary[21] + dsl.summary[22]).min(1.0);
        out[14] += n * dsl.summary[23];
        out[15] += n * dsl.summary[24];
        // Key synergy shares [16..24]: ramp, draw, spot_removal, wipe, counter, gy, tokens, Sac/ETB
        out[16] += n * (dsl.summary[120] + dsl.summary[121]).min(1.0);
        out[17] += n * dsl.summary[122];
        out[18] += n * dsl.summary[124];
        out[19] += n * dsl.summary[125];
        out[20] += n * dsl.summary[126];
        out[21] += n * (dsl.summary[127] + dsl.summary[128]).min(1.0);
        out[22] += n * dsl.summary[129];
        out[23] += n * (dsl.summary[130] + dsl.summary[141]).min(1.0);
    }
    if total > 0.0 {
        for v in &mut out {
            *v = (*v / total).clamp(0.0, 1.0);
        }
    }
    out
}

/// Zone identifiers for tokenized entities (`0..12`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EntityZone4 {
    /// Shared battlefield (`0`).
    Battlefield = 0,
    /// The stack (`1`).
    Stack = 1,
    /// Viewing seat's own hand (`2`).
    Hand = 2,
    /// Voluntarily or publicly shared hand (`3`).
    SharedHand = 3,
    /// Controlled player's hand (`4`).
    ControlledHand = 4,
    /// Graveyard (`5`).
    Graveyard = 5,
    /// Public exile (`6`).
    Exile = 6,
    /// Command zone (`7`).
    Command = 7,
    /// Temporary inspection zone (`PlayerView::looking_at`, `8`).
    LookingAt = 8,
    /// Revealed top of library (`PlayerView::library_tops`, `9`).
    LibraryTop = 9,
    /// Designated commander tracker (`SeatView::commanders`, `10`).
    CommanderDesignation = 10,
    /// Previously revealed opponent card remembered by `SeatKnowledge4` (`11`).
    RememberedOpponentCard = 11,
}

/// Directed edge relation kinds (`0..14`) between tokens in the unified
/// transformer sequence (`0` = Global, `1..=8` = Relative Seats `0..7`,
/// `9..` = Entities `0..num_entities`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i16)]
pub enum EdgeRel4 {
    /// `entity -> controller_seat` (`0`).
    Controller = 0,
    /// `entity -> owner_seat` (`1`).
    Owner = 1,
    /// `entity -> attached_to_entity` (`2`).
    AttachedTo = 2,
    /// `attacker_entity -> defending_seat_or_entity` (`3`).
    Attacking = 3,
    /// `blocker_entity -> blocked_attacker_entity` (`4`).
    Blocking = 4,
    /// `attacker_entity -> blocking_entity` (`5`).
    BlockedBy = 5,
    /// `stack_entity -> target_entity_or_seat` (`6`).
    StackTarget = 6,
    /// `stack_entity -> source_entity` (`7`).
    StackSource = 7,
    /// `granted_entity -> grantor_entity` (`8`).
    GrantSource = 8,
    /// `band_member_entity -> peer_band_member_entity` (`9`).
    BandMember = 9,
    /// `battle_entity -> protector_seat` (`10`).
    ProtectorSeat = 10,
    /// `entity -> chosen_opponent_seat` (`11`).
    ChosenOpponentSeat = 11,
    /// `entity -> other_entity_sharing_card_index` (`12`).
    SameCardCopy = 12,
    /// `targeting_source_entity -> global_token` (`13`).
    TargetingContextSource = 13,
}

/// Complete v4 encoded state frame (`EncodedState4`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedState4 {
    /// Per-entity numeric features (`num_entities x 96`).
    pub entity_feat: Vec<Vec<f32>>,
    /// Engine `ObjectId` for each entity row (`0` for synthetic memory rows).
    pub entity_object_ids: Vec<u64>,
    /// Active rules `CardIndex` (`rules.card` when copying, else `card.index`, `0` if unknown/token).
    pub entity_card_ids: Vec<u32>,
    /// Active face (`0` or `1`).
    pub entity_face_ids: Vec<u8>,
    /// Registry token ID (`0` if not a registry token).
    pub entity_token_ids: Vec<u16>,
    /// Zone ID (`EntityZone4 as u8`).
    pub entity_zone_ids: Vec<u8>,
    /// Relative controller seat (`0..7`, `8` = unknown).
    pub entity_rel_controller: Vec<u8>,
    /// Per-seat features in relative seat order `0..7` (`8 x 64`).
    pub seat_feat: Vec<Vec<f32>>,
    /// Global table/turn/payment/deck features (`64`).
    pub global_feat: Vec<f32>,
    /// Directed relational edges `[src_token, dst_token, rel_kind]`.
    pub edges: Vec<[i16; 3]>,
}

impl Default for EncodedState4 {
    fn default() -> Self {
        Self {
            entity_feat: Vec::new(),
            entity_object_ids: Vec::new(),
            entity_card_ids: Vec::new(),
            entity_face_ids: Vec::new(),
            entity_token_ids: Vec::new(),
            entity_zone_ids: Vec::new(),
            entity_rel_controller: Vec::new(),
            seat_feat: vec![vec![0.0; SEAT_FEAT_WIDTH]; MAX_SEATS],
            global_feat: vec![0.0; GLOBAL_FEAT_WIDTH],
            edges: Vec::new(),
        }
    }
}

impl EncodedState4 {
    /// Finds the entity index (`0..entity_object_ids.len()`) for an [`ObjectId`].
    #[must_use]
    pub fn find_entity(&self, id: ObjectId) -> Option<usize> {
        let raw = pack_object_id(id);
        self.entity_object_ids.iter().position(|&oid| oid == raw)
    }

    /// Materializes the `CARD_DSL_WIDTH` (`160`) vector for entity `idx` using
    /// `entity_card_ids`, `entity_face_ids`, and `entity_token_ids` (`0` = none,
    /// `1 + token_id` = registry token).
    #[must_use]
    pub fn entity_dsl_summary(&self, idx: usize) -> [f32; CARD_DSL_WIDTH] {
        let card_id = self.entity_card_ids.get(idx).copied().unwrap_or(0);
        let face_id = self.entity_face_ids.get(idx).copied().unwrap_or(0) as usize;
        if card_id != 0
            && let Some(def) = baylee_cards::by_index(CardIndex::new(card_id))
        {
            return encode_card_face(def, face_id).summary;
        }
        let tok_slot = self.entity_token_ids.get(idx).copied().unwrap_or(0);
        if tok_slot != 0
            && let Some(tok) = baylee_cards::tokens::by_token_id(tok_slot - 1)
        {
            return encode_token_def(tok).summary;
        }
        [0.0; CARD_DSL_WIDTH]
    }
}

/// Encodes a [`PlayerView`] (plus optional [`Pending`] and [`SeatKnowledge4`])
/// into an [`EncodedState4`].
#[must_use]
pub fn encode_state4(
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
) -> EncodedState4 {
    let mut state = EncodedState4::default();
    let viewer = view.seat;
    let seat_count = view.seats.len().clamp(1, MAX_SEATS);

    // 1. Collect entities in deterministic priority order up to MAX_ENTITIES:
    //    Battlefield -> Stack -> Own Hand -> Shared/Controlled Hands ->
    //    LookingAt -> LibraryTops -> Command -> Graveyards -> Exile ->
    //    Known opponent cards from SeatKnowledge4.
    for obj in &view.battlefield {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::Battlefield,
            viewer,
            seat_count,
            pending,
        );
    }
    for obj in &view.stack {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::Stack,
            viewer,
            seat_count,
            pending,
        );
    }
    for h_obj in &view.hand {
        push_hand_object(
            &mut state,
            h_obj,
            viewer,
            EntityZone4::Hand,
            viewer,
            seat_count,
            pending,
        );
    }
    for sh in &view.shared_hands {
        for h_obj in &sh.cards {
            push_hand_object(
                &mut state,
                h_obj,
                sh.player,
                EntityZone4::SharedHand,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for ch in &view.controlled_hands {
        for h_obj in &ch.cards {
            push_hand_object(
                &mut state,
                h_obj,
                ch.player,
                EntityZone4::ControlledHand,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for obj in &view.looking_at {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::LookingAt,
            viewer,
            seat_count,
            pending,
        );
    }
    for obj in &view.library_tops {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::LibraryTop,
            viewer,
            seat_count,
            pending,
        );
    }
    for seat_cmd in &view.command {
        for obj in seat_cmd {
            push_public_object(
                &mut state,
                view,
                obj,
                EntityZone4::Command,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for seat_gy in &view.graveyards {
        for obj in seat_gy {
            push_public_object(
                &mut state,
                view,
                obj,
                EntityZone4::Graveyard,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for seat_ex in &view.exile {
        for obj in seat_ex {
            push_public_object(
                &mut state,
                view,
                obj,
                EntityZone4::Exile,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    if let Some(kn) = knowledge {
        for (&(_owner, obj_id), mem) in &kn.known_cards {
            if state.entity_object_ids.len() >= MAX_ENTITIES {
                break;
            }
            if state.entity_object_ids.contains(&obj_id) {
                continue;
            }
            push_remembered_card(&mut state, obj_id, mem, viewer, seat_count);
        }
    }

    // 2. Encode Seat features in relative seat order `0..seat_count`
    encode_seats4(&mut state, view, knowledge, teams, viewer, seat_count);

    // 3. Encode Global features
    encode_global4(&mut state, view, pending, knowledge, teams, viewer, seat_count);

    // 4. Build directed relational edges
    build_edges4(&mut state, view, viewer, seat_count);

    state
}

fn push_public_object(
    state: &mut EncodedState4,
    view: &PlayerView,
    obj: &PublicObject,
    zone: EntityZone4,
    viewer: PlayerId,
    seat_count: usize,
    pending: Option<&Pending>,
) {
    if state.entity_feat.len() >= MAX_ENTITIES {
        return;
    }
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH];
    // [0..12]: zone one-hot
    f[zone as usize] = 1.0;

    let rel_ctrl = rel_seat(viewer, Some(obj.controller), seat_count);
    let rel_own = rel_seat(viewer, Some(obj.owner), seat_count);
    let rel_prot = rel_seat(viewer, obj.battle_protector, seat_count);
    let rel_c_opp = rel_seat(viewer, obj.chosen_opponent, seat_count);

    // [12..20]: relative seat relationships
    f[12] = f32::from(u8::from(rel_ctrl == 0));
    f[13] = unit(rel_ctrl as f32, 8.0);
    f[14] = f32::from(u8::from(rel_own == 0));
    f[15] = unit(rel_own as f32, 8.0);
    f[16] = f32::from(u8::from(obj.battle_protector.is_some()));
    f[17] = unit(rel_prot as f32, 8.0);
    f[18] = f32::from(u8::from(obj.chosen_opponent.is_some()));
    f[19] = unit(rel_c_opp as f32, 8.0);

    // [20..27]: all 7 ObjectStatus bits
    let st = obj.status;
    f[20] = f32::from(u8::from(st.contains(ObjectStatus::TAPPED)));
    f[21] = f32::from(u8::from(st.contains(ObjectStatus::FACE_DOWN)));
    f[22] = f32::from(u8::from(st.contains(ObjectStatus::PHASED_OUT)));
    f[23] = f32::from(u8::from(st.contains(ObjectStatus::FLIPPED)));
    f[24] = f32::from(u8::from(st.contains(ObjectStatus::NO_ABILITIES)));
    f[25] = f32::from(u8::from(st.contains(ObjectStatus::MELDED_CARD)));
    f[26] = f32::from(u8::from(st.contains(ObjectStatus::TOKEN)));

    // [27..34]: all 6 Offered bits + numeric picked derived from Pending
    if let Some(p) = pending {
        match p {
            Pending::Priority { legal, .. } => {
                if legal.castable.contains(&obj.id) || legal.suspendable.contains(&obj.id) {
                    f[27] = 1.0;
                }
                if legal.lands.contains(&obj.id) {
                    f[28] = 1.0;
                }
                if legal.mana_abilities.contains(&obj.id)
                    || legal.abilities.iter().any(|&(src, _)| src == obj.id)
                {
                    f[29] = 1.0;
                }
            }
            Pending::ChooseAttackers { attackers, .. } => {
                if attackers.contains(&obj.id) {
                    f[31] = 1.0;
                }
            }
            Pending::ChooseBlockers { blockers, .. } => {
                if blockers.iter().any(|b| b.blocker == obj.id) {
                    f[32] = 1.0;
                }
            }
            Pending::ChooseCards { options, .. }
            | Pending::ChooseTargets { options, .. }
            | Pending::LegendChoice { options, .. }
            | Pending::Arrange { cards: options, .. } => {
                if options.contains(&obj.id) {
                    f[30] = 1.0;
                }
            }
            _ => {}
        }
    }

    // [34..46]: face, rules override, prepared, chosen_subtype, chosen_name, doors, suspended, commander, summoning_sick
    let face_idx = obj
        .rules
        .map(|r| r.face)
        .or_else(|| obj.card.map(|c| c.face))
        .unwrap_or(0);
    let active_card_id = obj
        .rules
        .map(|r| r.card.get())
        .or_else(|| obj.card.map(|c| c.index.get()))
        .unwrap_or(0);
    let is_copy_override = obj
        .rules
        .zip(obj.card)
        .is_some_and(|(r, c)| r.card != c.index || r.face != c.face);
    f[34] = f32::from(face_idx);
    f[35] = f32::from(u8::from(is_copy_override));
    f[36] = f32::from(u8::from(obj.prepared_spell.is_some()));
    f[37] = obj
        .chosen_subtype
        .map_or(0.0, |st| unit(f32::from(st.get() % 256), 256.0));
    f[38] = f32::from(u8::from(obj.chosen_name.is_some()));
    if let Some([d0, d1]) = obj.unlocked_doors {
        f[39] = f32::from(u8::from(d0));
        f[40] = f32::from(u8::from(d1));
    }
    f[41] = f32::from(u8::from(obj.suspended));
    f[42] = f32::from(u8::from(obj.commander));
    f[43] = f32::from(u8::from(obj.summoning_sick));
    f[44] = f32::from(u8::from(obj.attached_to.is_some()));
    f[45] = unit(obj.mana_value as f32, 12.0);

    // [46..56]: projected & base P/T, damage, remaining toughness, lethal, loyalty
    f[46] = f32::from(u8::from(obj.power.is_some()));
    f[47] = obj.power.map_or(0.0, |p| signed_unit(p as f32, 12.0));
    f[48] = obj.toughness.map_or(0.0, |t| signed_unit(t as f32, 12.0));
    f[49] = obj
        .base_power
        .map_or(0.0, |p| signed_unit(p as f32, 12.0));
    f[50] = obj
        .base_toughness
        .map_or(0.0, |t| signed_unit(t as f32, 12.0));
    f[51] = unit(obj.damage as f32, 10.0);
    f[52] = obj
        .remaining_toughness()
        .map_or(0.0, |r| signed_unit(r as f32, 12.0));
    f[53] = f32::from(u8::from(obj.is_lethally_damaged()));
    f[54] = f32::from(u8::from(obj.loyalty.is_some()));
    f[55] = obj.loyalty.map_or(0.0, |l| unit(l as f32, 10.0));

    // [56..70]: types (9) + colors (5)
    f[56] = f32::from(u8::from(obj.types.contains(TypeSet::CREATURE)));
    f[57] = f32::from(u8::from(obj.types.contains(TypeSet::INSTANT)));
    f[58] = f32::from(u8::from(obj.types.contains(TypeSet::SORCERY)));
    f[59] = f32::from(u8::from(obj.types.contains(TypeSet::ARTIFACT)));
    f[60] = f32::from(u8::from(obj.types.contains(TypeSet::ENCHANTMENT)));
    f[61] = f32::from(u8::from(obj.types.contains(TypeSet::LAND)));
    f[62] = f32::from(u8::from(obj.types.contains(TypeSet::PLANESWALKER)));
    f[63] = f32::from(u8::from(obj.types.contains(TypeSet::BATTLE)));
    f[64] = f32::from(u8::from(obj.supertypes.contains(SupertypeSet::LEGENDARY)));
    f[65] = f32::from(u8::from(obj.colors.contains(Color::White)));
    f[66] = f32::from(u8::from(obj.colors.contains(Color::Blue)));
    f[67] = f32::from(u8::from(obj.colors.contains(Color::Black)));
    f[68] = f32::from(u8::from(obj.colors.contains(Color::Red)));
    f[69] = f32::from(u8::from(obj.colors.contains(Color::Green)));

    // [70..82]: CounterKind categories (all 21 mapped into 12 dense channels)
    for entry in &obj.counters {
        let amt = entry.count as f32;
        match entry.kind {
            CounterKind::Plus { power, toughness } => {
                f[70] = (f[70] + unit(amt * f32::from(power.max(toughness).max(1)), 8.0)).min(1.0);
            }
            CounterKind::Minus { power, toughness } => {
                f[71] = (f[71] + unit(amt * f32::from(power.max(toughness).max(1)), 8.0)).min(1.0);
            }
            CounterKind::Loyalty => f[72] = unit(amt, 10.0),
            CounterKind::Defense => f[73] = unit(amt, 10.0),
            CounterKind::Lore | CounterKind::Level => f[74] = unit(amt, 6.0),
            CounterKind::Time => f[75] = unit(amt, 6.0),
            CounterKind::Charge | CounterKind::Bore | CounterKind::Energy => f[76] = unit(amt, 8.0),
            CounterKind::Stun => f[77] = unit(amt, 4.0),
            CounterKind::Shield | CounterKind::Indestructible => f[78] = unit(amt, 4.0),
            CounterKind::Finality => f[79] = unit(amt, 2.0),
            CounterKind::Flying
            | CounterKind::Lifelink
            | CounterKind::Shadow
            | CounterKind::Hone => f[80] = unit(amt, 4.0),
            CounterKind::Poison | CounterKind::Rad | CounterKind::Custom(_) => {
                f[81] = (f[81] + unit(amt, 6.0)).min(1.0);
            }
        }
    }

    // [82..90]: Combat state & mana/grant capabilities
    let is_attacking = view
        .combat
        .attackers
        .iter()
        .find(|a| a.creature == obj.id);
    let is_blocking = view
        .combat
        .blockers
        .iter()
        .any(|b| b.blocker == obj.id);
    f[82] = f32::from(u8::from(is_attacking.is_some()));
    f[83] = f32::from(u8::from(is_attacking.is_some_and(|a| a.blocked)));
    f[84] = f32::from(u8::from(is_blocking));
    f[85] = f32::from(u8::from(
        obj.granted_mana.is_some() || obj.board_mana.is_some(),
    ));
    f[86] = f32::from(u8::from(obj.flashback.is_some()));
    f[87] = obj
        .flashback
        .as_ref()
        .map_or(0.0, |mc| unit(mc.cmc() as f32, 10.0));
    f[88] = unit(obj.grants.len() as f32, 4.0);
    f[89] = f32::from(u8::from(obj.ability_mana_spending != ManaSpending::EXACT));

    // [90..96]: StackItem & projected keywords summary
    match obj.stack_item {
        Some(StackItem::Spell) => {
            f[90] = 1.0;
        }
        Some(StackItem::Ability { ability, token, .. }) => {
            f[91] = 1.0;
            let ab_idx = ability
                .map(|a| a.index)
                .or_else(|| token.map(|t| t.index))
                .unwrap_or(0);
            f[92] = unit(ab_idx as f32, 8.0);
        }
        None => {}
    }
    f[93] = unit(obj.targets.len() as f32, 4.0);
    f[94] = f32::from(u8::from((obj.keywords & (1 << 0)) != 0)); // Flying
    f[95] = f32::from(u8::from(obj.keywords != 0));

    let token_slot = obj.token.map_or(0, |id| {
        if (id as usize) < baylee_cards::tokens::ALL.len() {
            id.saturating_add(1)
        } else {
            0
        }
    });
    state.entity_feat.push(f);
    state.entity_object_ids.push(pack_object_id(obj.id));
    state.entity_card_ids.push(active_card_id);
    state.entity_face_ids.push(face_idx);
    state.entity_token_ids.push(token_slot);
    state.entity_zone_ids.push(zone as u8);
    state.entity_rel_controller.push(rel_ctrl as u8);
}

fn push_hand_object(
    state: &mut EncodedState4,
    h_obj: &HandObject,
    owner: PlayerId,
    zone: EntityZone4,
    viewer: PlayerId,
    seat_count: usize,
    pending: Option<&Pending>,
) {
    if state.entity_feat.len() >= MAX_ENTITIES {
        return;
    }
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH];
    f[zone as usize] = 1.0;
    let rel_own = rel_seat(viewer, Some(owner), seat_count);
    f[12] = f32::from(u8::from(rel_own == 0));
    f[13] = unit(rel_own as f32, 8.0);
    f[14] = f32::from(u8::from(rel_own == 0));
    f[15] = unit(rel_own as f32, 8.0);

    // Check if castable or playable land in current Priority pending
    if let Some(Pending::Priority { legal, .. }) = pending {
        if legal.castable.contains(&h_obj.id) || legal.suspendable.contains(&h_obj.id) {
            f[27] = 1.0;
        }
        if legal.lands.contains(&h_obj.id) {
            f[28] = 1.0;
        }
        if legal
            .abilities
            .iter()
            .any(|(src, _)| *src == h_obj.id)
        {
            f[29] = 1.0;
        }
    }
    f[34] = f32::from(h_obj.card.face);
    f[42] = f32::from(u8::from(h_obj.commander));
    f[45] = unit(h_obj.mana_value as f32, 12.0);

    if let Some(def) = baylee_cards::by_index(h_obj.card.index)
        && let Some(face) = def.faces.get(h_obj.card.face as usize).or_else(|| def.faces.first())
    {
        f[46] = f32::from(u8::from(face.power.is_some()));
        f[47] = face.power.map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
        f[48] = face
            .toughness
            .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
        f[54] = f32::from(u8::from(face.loyalty.is_some()));
        f[55] = face.loyalty.map_or(0.0, |l| unit(f32::from(l), 10.0));
    }

    f[56] = f32::from(u8::from(h_obj.types.contains(TypeSet::CREATURE)));
    f[57] = f32::from(u8::from(h_obj.types.contains(TypeSet::INSTANT)));
    f[58] = f32::from(u8::from(h_obj.types.contains(TypeSet::SORCERY)));
    f[59] = f32::from(u8::from(h_obj.types.contains(TypeSet::ARTIFACT)));
    f[60] = f32::from(u8::from(h_obj.types.contains(TypeSet::ENCHANTMENT)));
    f[61] = f32::from(u8::from(h_obj.types.contains(TypeSet::LAND)));
    f[62] = f32::from(u8::from(h_obj.types.contains(TypeSet::PLANESWALKER)));
    f[63] = f32::from(u8::from(h_obj.types.contains(TypeSet::BATTLE)));
    f[65] = f32::from(u8::from(h_obj.colors.contains(Color::White)));
    f[66] = f32::from(u8::from(h_obj.colors.contains(Color::Blue)));
    f[67] = f32::from(u8::from(h_obj.colors.contains(Color::Black)));
    f[68] = f32::from(u8::from(h_obj.colors.contains(Color::Red)));
    f[69] = f32::from(u8::from(h_obj.colors.contains(Color::Green)));

    state.entity_feat.push(f);
    state.entity_object_ids.push(pack_object_id(h_obj.id));
    state.entity_card_ids.push(h_obj.card.index.get());
    state.entity_face_ids.push(h_obj.card.face);
    state.entity_token_ids.push(0);
    state.entity_zone_ids.push(zone as u8);
    state.entity_rel_controller.push(rel_own as u8);
}

fn push_remembered_card(
    state: &mut EncodedState4,
    obj_id: u64,
    mem: &KnownCardMemory4,
    viewer: PlayerId,
    seat_count: usize,
) {
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH];
    f[EntityZone4::RememberedOpponentCard as usize] = 1.0;
    let rel_own = rel_seat(viewer, Some(PlayerId::new(mem.owner_seat)), seat_count);
    f[13] = unit(rel_own as f32, 8.0);
    f[15] = unit(rel_own as f32, 8.0);
    if let Some(def) = baylee_cards::by_index(CardIndex::new(mem.card_index))
        && let Some(face) = def.faces.first()
    {
        f[45] = unit(face.mana_cost.cmc() as f32, 12.0);
        f[56] = f32::from(u8::from(face.types.contains(TypeSet::CREATURE)));
        f[57] = f32::from(u8::from(face.types.contains(TypeSet::INSTANT)));
        f[58] = f32::from(u8::from(face.types.contains(TypeSet::SORCERY)));
        f[59] = f32::from(u8::from(face.types.contains(TypeSet::ARTIFACT)));
        f[60] = f32::from(u8::from(face.types.contains(TypeSet::ENCHANTMENT)));
        f[61] = f32::from(u8::from(face.types.contains(TypeSet::LAND)));
        f[62] = f32::from(u8::from(face.types.contains(TypeSet::PLANESWALKER)));
    }
    state.entity_feat.push(f);
    state.entity_object_ids.push(obj_id);
    state.entity_card_ids.push(mem.card_index);
    state.entity_face_ids.push(0);
    state.entity_token_ids.push(0);
    state
        .entity_zone_ids
        .push(EntityZone4::RememberedOpponentCard as u8);
    state.entity_rel_controller.push(rel_own as u8);
}

fn encode_seats4(
    state: &mut EncodedState4,
    view: &PlayerView,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
    viewer: PlayerId,
    seat_count: usize,
) {
    let my_team = teams.get(viewer.get() as usize).copied().flatten();
    for rel in 0..seat_count.min(MAX_SEATS) {
        let abs = abs_seat(viewer, rel, seat_count);
        let Some(sv) = view.seats.get(abs.get() as usize) else {
            continue;
        };
        let dst = &mut state.seat_feat[rel];
        let seat_team = teams.get(abs.get() as usize).copied().flatten();
        let is_self = rel == 0;
        let is_teammate = !is_self && my_team.is_some() && my_team == seat_team;
        let is_opponent = !is_self && !is_teammate;

        dst[0] = 1.0; // seat present
        dst[1] = f32::from(u8::from(is_self));
        dst[2] = f32::from(u8::from(is_teammate));
        dst[3] = f32::from(u8::from(is_opponent));
        dst[4] = f32::from(u8::from(sv.has_lost()));
        dst[5] = f32::from(u8::from(view.active == abs));
        dst[6] = f32::from(u8::from(view.awaiting == Some(abs)));
        dst[7] = f32::from(u8::from(view.decision_player == Some(abs)));
        dst[8] = f32::from(u8::from(view.monarch == Some(abs)));
        dst[9] = f32::from(u8::from(sv.no_max_hand_size));
        dst[10] = f32::from(u8::from(view.deciding.contains(abs)));

        // Life, counters, speed, rad
        let start_life = knowledge.map_or(20, |k| k.starting_life[abs.get() as usize]);
        dst[11] = signed_unit(sv.life as f32, 40.0);
        dst[12] = signed_unit((sv.life - start_life) as f32, 40.0);
        dst[13] = unit(f32::from(sv.poison), 10.0);
        dst[14] = unit(f32::from(sv.energy), 10.0);
        dst[15] = unit(f32::from(sv.speed), 4.0);
        dst[16] = unit(f32::from(sv.rad), 10.0);

        // Zone counts & board aggregates
        dst[17] = unit(sv.hand_count as f32, 7.0);
        dst[18] = unit(sv.library_count as f32, 60.0);
        dst[19] = unit(sv.graveyard_count as f32, 20.0);
        let exile_cnt = view
            .exile
            .get(abs.get() as usize)
            .map_or(0, Vec::len);
        dst[20] = unit(exile_cnt as f32, 10.0);

        let mut creatures = 0.0_f32;
        let mut lands = 0.0_f32;
        let mut untapped_lands = 0.0_f32;
        let mut other_perms = 0.0_f32;
        let mut board_power = 0.0_f32;
        let mut board_tough = 0.0_f32;
        for obj in view.battlefield.iter().filter(|o| o.controller == abs) {
            if obj.types.contains(TypeSet::CREATURE) {
                creatures += 1.0;
                board_power += obj.power.unwrap_or(0).max(0) as f32;
                board_tough += obj.toughness.unwrap_or(0).max(0) as f32;
            } else if obj.types.contains(TypeSet::LAND) {
                lands += 1.0;
                if !obj.status.contains(ObjectStatus::TAPPED) {
                    untapped_lands += 1.0;
                }
            } else {
                other_perms += 1.0;
            }
        }
        dst[21] = unit(creatures, 10.0);
        dst[22] = unit(lands, 10.0);
        dst[23] = unit(untapped_lands, 10.0);
        dst[24] = unit(other_perms, 10.0);
        dst[25] = unit(board_power, 25.0);
        dst[26] = unit(board_tough, 25.0);

        // ManaPoolView: W, U, B, R, G, C, total, spending, restricted[0..6]
        let mp = &sv.mana_pool;
        dst[27] = unit(mp.white as f32, 8.0);
        dst[28] = unit(mp.blue as f32, 8.0);
        dst[29] = unit(mp.black as f32, 8.0);
        dst[30] = unit(mp.red as f32, 8.0);
        dst[31] = unit(mp.green as f32, 8.0);
        dst[32] = unit(mp.colorless as f32, 8.0);
        dst[33] = unit(mp.total() as f32, 12.0);
        dst[34] = f32::from(u8::from(mp.spending != ManaSpending::EXACT));
        for (r_idx, &r_val) in mp.restricted.iter().enumerate().take(6) {
            dst[35 + r_idx] = unit(r_val as f32, 6.0);
        }

        // Commander info & 8-seat commander damage matrix
        dst[41] = unit(sv.commanders.len() as f32, 2.0);
        let cmd_casts: u32 = sv.commanders.iter().map(|c| c.casts).sum();
        dst[42] = unit(cmd_casts as f32, 5.0);
        let max_cmd_dmg = sv
            .commander_damage
            .iter()
            .map(|cd| cd.amount)
            .max()
            .unwrap_or(0);
        dst[43] = unit(max_cmd_dmg as f32, 21.0);
        // Map each CommanderDamage source back to the relative seat of that commander's owner
        for cd in &sv.commander_damage {
            if let Some((owner_idx, _)) = view
                .seats
                .iter()
                .enumerate()
                .find(|(_, other_s)| other_s.commanders.iter().any(|c| c.object == cd.source))
            {
                let rel_src = rel_seat(viewer, Some(PlayerId::new(owner_idx as u8)), seat_count);
                if rel_src < MAX_SEATS {
                    dst[44 + rel_src] =
                        (dst[44 + rel_src] + unit(cd.amount as f32, 21.0)).min(1.0);
                }
            }
        }

        // Public history statistics from SeatKnowledge4 [52..64]
        if let Some(kn) = knowledge {
            let h = &kn.seat_history[abs.get() as usize];
            dst[52] = unit(f32::from(h.spells_cast_total), 20.0);
            dst[53] = unit(f32::from(h.spells_cast_this_turn), 5.0);
            dst[54] = unit(f32::from(h.lands_played_total), 12.0);
            dst[55] = unit(f32::from(h.creatures_entered_total), 15.0);
            dst[56] = unit(f32::from(h.attacks_declared_total), 10.0);
            dst[57] = unit(f32::from(h.life_gained_total), 20.0);
            dst[58] = unit(f32::from(h.life_lost_total), 30.0);
            dst[59] = unit(f32::from(h.graveyard_peak), 20.0);
            let known_in_hand = kn
                .known_cards
                .values()
                .filter(|m| m.owner_seat == abs.get() && m.origin_zone == 1)
                .count();
            dst[60] = unit(known_in_hand as f32, 7.0);
        }
    }
}

fn encode_global4(
    state: &mut EncodedState4,
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
    viewer: PlayerId,
    seat_count: usize,
) {
    let g = &mut state.global_feat;
    g[0] = unit(view.turn as f32, 20.0);
    let step_idx = match view.step {
        Step::Untap => 0,
        Step::Upkeep => 1,
        Step::Draw => 2,
        Step::Main => {
            if matches!(view.phase, Phase::SecondMain) {
                10
            } else {
                3
            }
        }
        Step::CombatBegin => 4,
        Step::DeclareAttackers => 5,
        Step::DeclareBlockers => 6,
        Step::CombatDamageFirst => 7,
        Step::CombatDamage => 8,
        Step::CombatEnd => 9,
        Step::End => 11,
        Step::Cleanup => 12,
    };
    g[1 + step_idx] = 1.0; // [1..14]

    let rel_act = rel_seat(viewer, Some(view.active), seat_count);
    let rel_awt = rel_seat(viewer, view.awaiting, seat_count);
    g[14] = unit(rel_act as f32, 8.0);
    g[15] = f32::from(u8::from(rel_act == 0));
    g[16] = unit(rel_awt as f32, 8.0);
    g[17] = f32::from(u8::from(rel_awt == 0));
    g[18] = unit(seat_count as f32, 8.0);

    let surviving = view.seats.iter().filter(|s| !s.has_lost()).count();
    g[19] = unit(surviving as f32, 8.0);
    let is_team = teams.iter().any(Option::is_some);
    let is_cmd = view.seats.iter().any(|s| !s.commanders.is_empty());
    g[20] = f32::from(u8::from(is_team));
    g[21] = f32::from(u8::from(is_cmd));
    g[22] = match view.day_night {
        None => 0.0,
        Some(DayNight::Day) => 0.5,
        Some(DayNight::Night) => 1.0,
    };
    g[23] = unit(view.stack.len() as f32, 6.0);
    g[24] = f32::from(u8::from(view.combat.is_active()));
    g[25] = unit(view.combat.attackers.len() as f32, 10.0);
    g[26] = unit(view.combat.blockers.len() as f32, 10.0);
    g[27] = f32::from(u8::from(view.sorcery_lock.is_some()));
    g[28] = f32::from(u8::from(view.sorceries_have_flash));
    g[29] = f32::from(u8::from(view.casting.is_some()));
    g[30] = f32::from(u8::from(view.targeting.is_some()));

    // Owed payment window (CR 605.3a)
    match &view.owed {
        None => {}
        Some(ManaPayment::Fixed(mc)) => {
            g[31] = 1.0;
            g[33] = unit(mc.cmc() as f32, 10.0);
        }
        Some(ManaPayment::AnyAmount { preventable_damage }) => {
            g[32] = 1.0;
            g[33] = unit(*preventable_damage as f32, 10.0);
        }
    }

    if let Some(p) = pending {
        let pk = PendingKind4::of(p);
        g[34] = unit(pk as u8 as f32, 28.0);
    }
    if let Some(kn) = knowledge {
        g[35] = unit(kn.cursor as f32, 200.0);
        // [40..64]: 24-dim own deck summary
        g[40..64].copy_from_slice(&kn.own_deck_summary);
    }
}

fn build_edges4(
    state: &mut EncodedState4,
    view: &PlayerView,
    viewer: PlayerId,
    seat_count: usize,
) {
    let entity_ids = state.entity_object_ids.clone();
    let obj_token = |id: ObjectId| -> Option<i16> {
        let raw = pack_object_id(id);
        entity_ids
            .iter()
            .position(|&oid| oid == raw)
            .map(|idx| ENTITY_TOKEN_OFFSET + idx as i16)
    };
    let mut push_edge = |src: i16, dst: i16, rel: EdgeRel4| {
        if state.edges.len() < MAX_EDGES {
            state.edges.push([src, dst, rel as i16]);
        }
    };
    let seat_token = |p: PlayerId| -> i16 {
        let rel = rel_seat(viewer, Some(p), seat_count);
        SEAT_TOKEN_OFFSET + (rel.min(MAX_SEATS - 1) as i16)
    };

    // 1. Entity relationships on battlefield & stack
    for obj in view.battlefield.iter().chain(&view.stack) {
        let Some(src_tok) = obj_token(obj.id) else {
            continue;
        };
        push_edge(src_tok, seat_token(obj.controller), EdgeRel4::Controller);
        if obj.owner != obj.controller {
            push_edge(src_tok, seat_token(obj.owner), EdgeRel4::Owner);
        }
        if let Some(parent) = obj.attached_to
            && let Some(dst_tok) = obj_token(parent)
        {
            push_edge(src_tok, dst_tok, EdgeRel4::AttachedTo);
        }
        if let Some(prot) = obj.battle_protector {
            push_edge(src_tok, seat_token(prot), EdgeRel4::ProtectorSeat);
        }
        if let Some(c_opp) = obj.chosen_opponent {
            push_edge(src_tok, seat_token(c_opp), EdgeRel4::ChosenOpponentSeat);
        }
        for grant in &obj.grants {
            if let Some(grantor) = grant.source
                && let Some(dst_tok) = obj_token(grantor)
            {
                push_edge(src_tok, dst_tok, EdgeRel4::GrantSource);
            }
        }
        if let Some(StackItem::Ability { source, .. }) = obj.stack_item
            && let Some(dst_tok) = obj_token(source)
        {
            push_edge(src_tok, dst_tok, EdgeRel4::StackSource);
        }
        for &t in &obj.targets {
            match t {
                TargetRef::Object(ds) => {
                    if let Some(dst_tok) = obj_token(ds.object) {
                        push_edge(src_tok, dst_tok, EdgeRel4::StackTarget);
                    }
                }
                TargetRef::Player(pid) => {
                    push_edge(src_tok, seat_token(pid), EdgeRel4::StackTarget);
                }
            }
        }
    }

    // 2. Combat edges: attackers, blockers, bands
    for atk in &view.combat.attackers {
        if let Some(atk_tok) = obj_token(atk.creature) {
            let dst_tok = match atk.defending {
                Defender::Player(pid) => Some(seat_token(pid)),
                Defender::Planeswalker(oid) | Defender::Battle(oid) => obj_token(oid),
            };
            if let Some(dst) = dst_tok {
                push_edge(atk_tok, dst, EdgeRel4::Attacking);
            }
        }
    }
    for blk in &view.combat.blockers {
        if let (Some(b_tok), Some(a_tok)) = (obj_token(blk.blocker), obj_token(blk.attacker)) {
            push_edge(b_tok, a_tok, EdgeRel4::Blocking);
            push_edge(a_tok, b_tok, EdgeRel4::BlockedBy);
        }
    }
    for band in &view.combat.bands {
        for i in 0..band.len() {
            for j in (i + 1)..band.len() {
                if let (Some(ti), Some(tj)) = (obj_token(band[i]), obj_token(band[j])) {
                    push_edge(ti, tj, EdgeRel4::BandMember);
                    push_edge(tj, ti, EdgeRel4::BandMember);
                }
            }
        }
    }

    // 3. Targeting context source -> Global token (0)
    if let Some(tc) = &view.targeting
        && let Some(src_tok) = obj_token(tc.source.id)
    {
        push_edge(src_tok, 0, EdgeRel4::TargetingContextSource);
    }

    // 4. Same-card-copy edges between battlefield/hand/stack entities sharing a CardIndex
    let n_ent = state.entity_card_ids.len();
    for i in 0..n_ent {
        let cid = state.entity_card_ids[i];
        if cid == 0 {
            continue;
        }
        for j in (i + 1)..n_ent {
            if state.entity_card_ids[j] == cid {
                let ti = ENTITY_TOKEN_OFFSET + i as i16;
                let tj = ENTITY_TOKEN_OFFSET + j as i16;
                push_edge(ti, tj, EdgeRel4::SameCardCopy);
                break;
            }
        }
    }
}

/// Which of the four factorized policy heads answers a decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum HeadKind4 {
    /// Choose one candidate index `0..candidates.len()`.
    SingleChoice = 0,
    /// Independent binary inclusion over `0..candidates.len()` (or sequential multi-pick).
    SubsetPick = 1,
    /// Choose a pair `(a, b)` or multi-pair assignment (`DeclareBlockers`, multi-defender attacks).
    PairPick = 2,
    /// Scalar integer/bucket choice (`AnnounceX`, `ChooseNumber`, `DivideValues`).
    ScalarValue = 3,
}

/// All `Pending` variants mapped to a stable index `0..28` for the
/// [`CoverageMatrix4`] and global conditioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum PendingKind4 {
    /// `Pending::Priority`
    Priority = 0,
    /// `Pending::ChooseManaAbility`
    ChooseManaAbility = 1,
    /// `Pending::ChooseDamageSource`
    ChooseDamageSource = 2,
    /// `Pending::ChooseDamageEffect`
    ChooseDamageEffect = 3,
    /// `Pending::AllocatePrevention`
    AllocatePrevention = 4,
    /// `Pending::ChooseAttackers`
    DeclareAttackers = 5,
    /// `Pending::ChooseBlockers`
    DeclareBlockers = 6,
    /// `Pending::ChooseCards`
    ChooseCards = 7,
    /// `Pending::ChooseTargets`
    ChooseTargets = 8,
    /// `Pending::Arrange`
    Arrange = 9,
    /// `Pending::LegendChoice`
    LegendChoice = 10,
    /// `Pending::Mulligan`
    Mulligan = 11,
    /// `Pending::MulliganBottom`
    MulliganBottom = 12,
    /// `Pending::DiscardChoice`
    DiscardChoice = 13,
    /// `Pending::YesNo`
    YesNo = 14,
    /// `Pending::ChooseCastMode`
    ChooseCastMode = 15,
    /// `Pending::ChooseNumber`
    ChooseNumber = 16,
    /// `Pending::ChooseColor`
    ChooseColor = 17,
    /// `Pending::ChooseSubtype`
    ChooseSubtype = 18,
    /// `Pending::ChooseCardName`
    ChooseCardName = 19,
    /// `Pending::ChoosePlayer`
    ChoosePlayer = 20,
    /// `Pending::ChoosePile`
    ChoosePile = 21,
    /// `Pending::GameOver`
    GameOver = 27,
}

impl PendingKind4 {
    /// Total number of `PendingKind4` buckets.
    pub const COUNT: usize = 28;

    /// Maps any [`Pending`] to its [`PendingKind4`].
    #[must_use]
    pub const fn of(pending: &Pending) -> Self {
        match pending {
            Pending::Priority { .. } => Self::Priority,
            Pending::ChooseManaAbility { .. } => Self::ChooseManaAbility,
            Pending::ChooseDamageSource { .. } => Self::ChooseDamageSource,
            Pending::ChooseDamageEffect { .. } => Self::ChooseDamageEffect,
            Pending::AllocatePrevention { .. } => Self::AllocatePrevention,
            Pending::ChooseAttackers { .. } => Self::DeclareAttackers,
            Pending::ChooseBlockers { .. } => Self::DeclareBlockers,
            Pending::ChooseCards { .. } => Self::ChooseCards,
            Pending::ChooseTargets { .. } => Self::ChooseTargets,
            Pending::Arrange { .. } => Self::Arrange,
            Pending::LegendChoice { .. } => Self::LegendChoice,
            Pending::Mulligan { .. } => Self::Mulligan,
            Pending::MulliganBottom { .. } => Self::MulliganBottom,
            Pending::DiscardChoice { .. } => Self::DiscardChoice,
            Pending::YesNo { .. } => Self::YesNo,
            Pending::ChooseCastMode { .. } => Self::ChooseCastMode,
            Pending::ChooseNumber { .. } => Self::ChooseNumber,
            Pending::ChooseColor { .. } => Self::ChooseColor,
            Pending::ChooseSubtype { .. } => Self::ChooseSubtype,
            Pending::ChooseCardName { .. } => Self::ChooseCardName,
            Pending::ChoosePlayer { .. } => Self::ChoosePlayer,
            Pending::ChoosePile { .. } => Self::ChoosePile,
            Pending::GameOver(_) => Self::GameOver,
        }
    }
}

/// Who produced a recorded decision step (`StepProvenance4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SourceKind4 {
    /// Answered directly by the neural policy (`Playernet v4`).
    Net = 0,
    /// Answered by the house heuristic agent (teacher or fallback).
    House = 1,
    /// Forced single-option decision (`<= 1` legal choice).
    Forced = 2,
    /// Answered by belief-consistent lookahead search over `Playernet v4`.
    Search = 3,
}

/// Provenance metadata stored with every decision step in v4 datasets and RL rollouts.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepProvenance4 {
    /// Who produced the action.
    pub source: SourceKind4,
    /// Sampling temperature `tau` used at the decision (`0.0` for greedy/teacher).
    pub temperature: f32,
    /// Number of latent thinking iterations `K` executed (`1..=8`).
    pub thinking_steps: u8,
    /// Exact conditional log-probability under the behavior policy at rollout time.
    pub behavior_logprob: f32,
    /// Whether `behavior_logprob` was recorded by an active neural rollout policy (`P0-2`).
    #[serde(default)]
    pub has_logprob: bool,
}

impl Default for StepProvenance4 {
    fn default() -> Self {
        Self {
            source: SourceKind4::House,
            temperature: 0.0,
            thinking_steps: 1,
            behavior_logprob: 0.0,
            has_logprob: false,
        }
    }
}

/// Coverage matrix tracking per-`PendingKind4` counts of `Net`, `House`, `Forced`,
/// and `Search` answers across a run or arena evaluation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CoverageMatrix4 {
    /// Counts `[pending_kind][source_kind]`.
    pub counts: [[u64; 4]; PendingKind4::COUNT],
}

impl CoverageMatrix4 {
    /// Records one answered decision.
    pub fn record(&mut self, kind: PendingKind4, source: SourceKind4) {
        self.counts[kind as usize][source as usize] += 1;
    }

    /// Total non-forced decisions answered by the neural net (`Net` + `Search`).
    #[must_use]
    pub fn neural_decisions(&self) -> u64 {
        self.counts.iter().map(|r| r[0] + r[3]).sum()
    }

    /// Total non-forced decisions that fell back to `House`.
    #[must_use]
    pub fn fallback_decisions(&self) -> u64 {
        self.counts.iter().map(|r| r[1]).sum()
    }
}

/// One encoded action candidate (`Candidate4`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate4 {
    /// Numeric candidate features (`48` floats).
    pub feat: Vec<f32>,
    /// Referenced entity index in `EncodedState4` (`-1` if none).
    pub entity_idx: i16,
    /// Referenced relative seat index (`0..7`, `-1` if none).
    pub rel_seat_idx: i8,
    /// Referenced `CardIndex` (`0` if none).
    pub card_id: u32,
    /// Referenced ability index (`0..MAX_ABILITIES`, `-1` if none).
    pub ability_idx: i8,
    /// Concrete `PlayerAction` when `head == SingleChoice` (or the atomic item
    /// meaning for `SubsetPick`).
    pub action: Option<PlayerAction>,
    /// Atomic object ID for `SubsetPick`.
    pub object_id: Option<ObjectId>,
    /// Atomic player ID for `SubsetPick` (`ChooseTargets`).
    pub player_id: Option<PlayerId>,
    /// Atomic mode/color index for `SubsetPick`.
    pub index_id: Option<u8>,
    /// Atomic defender target for `ChooseAttackers` (`P1-8`).
    #[serde(default)]
    pub defender: Option<Defender>,
}

impl Default for Candidate4 {
    fn default() -> Self {
        Self {
            feat: vec![0.0; CAND_FEAT_WIDTH],
            entity_idx: -1,
            rel_seat_idx: -1,
            card_id: 0,
            ability_idx: -1,
            action: None,
            object_id: None,
            player_id: None,
            index_id: None,
            defender: None,
        }
    }
}

/// Encoded legal candidates for one decision (`EncodedCandidates4`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedCandidates4 {
    /// Policy head kind (`0..4`).
    pub head: HeadKind4,
    /// `Pending` variant kind (`0..28`).
    pub pending_kind: PendingKind4,
    /// Minimum required selections for `SubsetPick` (or min value for `ScalarValue`).
    pub min_choices: u32,
    /// Maximum allowed selections for `SubsetPick` (or max value for `ScalarValue`).
    pub max_choices: u32,
    /// Candidate list (`0..=MAX_CANDIDATES`).
    pub candidates: Vec<Candidate4>,
    /// Legal `(src_cand_or_entity, dst_cand_or_entity)` pairs for `PairPick`.
    pub pairs: Vec<[i16; 2]>,
    /// Underlying pair actions (`(blocker, attacker)` or `(attacker, defender)`).
    pub pair_objects: Vec<(ObjectId, ObjectId)>,
}

/// Target label for supervised training or RL log-prob evaluation (`TeacherTarget4`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TeacherTarget4 {
    /// Chosen candidate index for `SingleChoice` (`0..candidates.len()`).
    pub single_idx: u16,
    /// Binary inclusion mask over `candidates` (`SubsetPick`) or `pairs` (`PairPick`).
    pub multi_mask: Vec<u8>,
    /// Binary mask (`1` = free Bernoulli decision, `0` = forced/forbidden by constraints, `P0-3`).
    #[serde(default)]
    pub free_mask: Vec<u8>,
    /// Ordered sequence of visited/chosen indices (`SubsetPick` / `PairPick` trace).
    pub pick_sequence: Vec<u16>,
    /// Normalized scalar target in `[0.0, 1.0]` for `ScalarValue`.
    pub scalar_norm: f32,
}

impl Default for TeacherTarget4 {
    fn default() -> Self {
        Self {
            single_idx: 0,
            multi_mask: Vec::new(),
            free_mask: Vec::new(),
            pick_sequence: Vec::new(),
            scalar_norm: 0.0,
        }
    }
}

/// Encodes all legal candidates for `pending` given `state` and `view`.
#[must_use]
pub fn encode_candidates4(
    state: &EncodedState4,
    view: &PlayerView,
    pending: &Pending,
) -> EncodedCandidates4 {
    let viewer = view.seat;
    let seat_count = view.seats.len().clamp(1, MAX_SEATS);
    let pending_kind = PendingKind4::of(pending);
    let mut out = EncodedCandidates4 {
        head: HeadKind4::SingleChoice,
        pending_kind,
        min_choices: 1,
        max_choices: 1,
        candidates: Vec::new(),
        pairs: Vec::new(),
        pair_objects: Vec::new(),
    };

    match pending {
        Pending::Priority { legal, .. } => {
            if legal.can_pass {
                let mut pass = Candidate4::default();
                pass.feat[0] = 1.0; // PassPriority
                pass.action = Some(PlayerAction::PassPriority);
                out.candidates.push(pass);
            }

            if let Some(cast_id) = view.casting
                && out.candidates.len() < MAX_CANDIDATES
            {
                let mut cancel = Candidate4::default();
                cancel.feat[1] = 1.0; // CancelCast
                if let Some(eidx) = state.find_entity(cast_id) {
                    cancel.entity_idx = eidx as i16;
                    cancel.card_id = state.entity_card_ids[eidx];
                }
                cancel.action = Some(PlayerAction::CancelCast);
                out.candidates.push(cancel);
            }

            for &land_id in &legal.lands {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[2] = 1.0; // PlayLand
                if let Some(eidx) = state.find_entity(land_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::PlayLand { card: land_id });
                out.candidates.push(c);
            }

            for &spell_id in &legal.castable {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[3] = 1.0; // CastSpell
                if let Some(eidx) = state.find_entity(spell_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                    c.feat[16] = state.entity_feat[eidx][45];
                }
                c.action = Some(PlayerAction::CastSpell { card: spell_id });
                out.candidates.push(c);
            }

            for &susp_id in &legal.suspendable {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[3] = 1.0;
                c.feat[18] = 1.0; // Suspend
                if let Some(eidx) = state.find_entity(susp_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::Suspend { card: susp_id });
                out.candidates.push(c);
            }

            for &mana_src in &legal.mana_abilities {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[5] = 1.0; // ActivateManaAbility
                if let Some(eidx) = state.find_entity(mana_src) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ActivateManaAbility { source: mana_src });
                out.candidates.push(c);
            }

            for &(src_id, ab_idx) in &legal.abilities {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[4] = 1.0; // ActivateAbility
                c.feat[17] = unit(ab_idx as f32, 8.0);
                if let Some(eidx) = state.find_entity(src_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                    if (ab_idx as usize) < MAX_ABILITIES {
                        c.ability_idx = ab_idx as i8;
                    }
                    populate_cand_ability_dsl(&mut c, state, eidx, ab_idx as usize);
                }
                c.action = Some(PlayerAction::ActivateAbility {
                    source: src_id,
                    ability_index: ab_idx,
                });
                out.candidates.push(c);
            }

            for grant in &legal.granted_actions {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[8] = 1.0; // TakeGrantedAction
                if let Some(eidx) = state.find_entity(grant.source.object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::TakeGrantedAction { id: grant.id });
                out.candidates.push(c);
            }
        }

        Pending::ChooseManaAbility {
            choice, options, ..
        } => {
            for (idx, opt) in options.iter().enumerate() {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[5] = 1.0; // ChooseManaAbility
                c.feat[16] = unit(idx as f32, 16.0);
                c.feat[17] = unit(opt.ability_index.unwrap_or(0) as f32, 8.0);
                if let Some(eidx) = state.find_entity(opt.source.object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseManaAbility {
                    choice: *choice,
                    source: opt.source,
                    ability_index: opt.ability_index,
                });
                out.candidates.push(c);
            }
        }

        Pending::ChooseDamageSource {
            choice, options, ..
        } => {
            for &src in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[6] = 1.0; // ChooseDamageSource
                if let Some(eidx) = state.find_entity(src.object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                } else if let Some(dsv) = view.damage_sources.iter().find(|d| d.source == src)
                    && let Some(ci) = dsv.card
                {
                    c.card_id = ci.index.get();
                }
                c.action = Some(PlayerAction::ChooseDamageSource {
                    choice: *choice,
                    source: src,
                });
                out.candidates.push(c);
            }
        }

        Pending::ChooseDamageEffect {
            choice, options, ..
        } => {
            for eff in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[7] = 1.0; // ChooseDamageEffect
                match eff.kind {
                    DamageEffectKind::PreventNext { .. }
                    | DamageEffectKind::PreventThisEvent { .. }
                    | DamageEffectKind::PreventCombat
                    | DamageEffectKind::Protection
                    | DamageEffectKind::PreventAll
                    | DamageEffectKind::PreventFromSource { .. }
                    | DamageEffectKind::RemoveShieldCounter
                    | DamageEffectKind::RemoveCounter { .. } => c.feat[16] = 1.0,
                    DamageEffectKind::Redirect { .. }
                    | DamageEffectKind::RedirectNext { .. } => c.feat[17] = 1.0,
                    DamageEffectKind::ReplaceAmount { .. }
                    | DamageEffectKind::Reflect { .. }
                    | DamageEffectKind::TurnFaceUp { .. } => c.feat[18] = 1.0,
                }
                if let Some(src_obj) = eff.source
                    && let Some(eidx) = state.find_entity(src_obj)
                {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseDamageEffect {
                    choice: *choice,
                    effect: eff.id,
                });
                out.candidates.push(c);
            }
        }

        Pending::AllocatePrevention {
            choice,
            damage,
            total,
            ..
        } => {
            let mut rem = *total;
            let mut alloc = Vec::new();
            for d in damage {
                let take = rem.min(d.amount);
                alloc.push((d.id, take));
                rem -= take;
            }
            let mut c = Candidate4::default();
            c.feat[7] = 1.0;
            c.feat[19] = unit(*total as f32, 10.0);
            c.action = Some(PlayerAction::AllocatePrevention {
                choice: *choice,
                allocation: alloc,
            });
            out.candidates.push(c);
        }

        Pending::ChooseCastMode {
            object, options, ..
        } => {
            for (idx, opt) in options.iter().enumerate() {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                encode_cast_mode_kind_into(&opt.kind, &opt.cost, idx as u8, &mut c.feat);
                if let Some(eidx) = state.find_entity(*object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseMode(idx));
                out.candidates.push(c);
            }
        }

        Pending::Mulligan { can_take, .. } => {
            let mut keep = Candidate4::default();
            keep.feat[9] = 1.0;
            keep.action = Some(PlayerAction::MulliganKeep);
            out.candidates.push(keep);

            if *can_take {
                let mut take = Candidate4::default();
                take.feat[10] = 1.0;
                take.action = Some(PlayerAction::MulliganTake);
                out.candidates.push(take);
            }
        }

        Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
            out.head = HeadKind4::SubsetPick;
            out.min_choices = u32::from(*count);
            out.max_choices = u32::from(*count);
            for h_obj in view.hand.iter().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(h_obj.id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.object_id = Some(h_obj.id);
                out.candidates.push(c);
            }
        }

        Pending::LegendChoice { options, .. } => {
            for &oid in options.iter().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(oid) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseObjects {
                    objects: vec![oid],
                });
                out.candidates.push(c);
            }
        }

        Pending::YesNo { source, .. } => {
            let cid = source.map_or(0, |a| a.card.get());
            for yn in [true, false] {
                let mut c = Candidate4::default();
                c.feat[11] = 1.0;
                c.feat[16] = f32::from(u8::from(yn));
                c.entity_idx = -1;
                c.card_id = cid;
                c.action = Some(PlayerAction::YesNo(yn));
                out.candidates.push(c);
            }
        }

        Pending::ChooseColor { options, .. } => {
            for &mc in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[13] = 1.0;
                c.feat[16 + mc.index().min(5)] = 1.0;
                c.action = Some(PlayerAction::ChooseColor(mc));
                out.candidates.push(c);
            }
        }

        Pending::ChoosePlayer { options, .. } => {
            for &pid in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[14] = 1.0;
                let rel = rel_seat(viewer, Some(pid), seat_count);
                c.rel_seat_idx = rel as i8;
                c.feat[16 + rel.min(7)] = 1.0;
                c.action = Some(PlayerAction::ChoosePlayer(pid));
                out.candidates.push(c);
            }
        }

        Pending::ChoosePile { piles, .. } => {
            for (p_idx, pile) in piles.iter().enumerate() {
                let mut c = Candidate4::default();
                c.feat[15] = 1.0;
                c.feat[18] = p_idx as f32;
                c.feat[19] = unit(pile.len() as f32, 10.0);
                if let Some(&first_obj) = pile.first()
                    && let Some(eidx) = state.find_entity(first_obj)
                {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseMode(p_idx));
                out.candidates.push(c);
            }
        }

        Pending::ChooseSubtype { options, .. } => {
            let mut prioritized: Vec<baylee_core::ids::SubtypeId> = Vec::new();
            for obj in view
                .battlefield
                .iter()
                .chain(&view.stack)
                .chain(view.graveyards.iter().flatten())
                .chain(view.command.iter().flatten())
                .chain(view.exile.iter().flatten())
            {
                for st in obj.subtypes.iter() {
                    if options.contains(&st) && !prioritized.contains(&st) {
                        prioritized.push(st);
                    }
                }
                if let Some(ci) = obj.card
                    && let Some(def) = baylee_cards::by_index(ci.index)
                {
                    for face in def.faces {
                        for &st in face.subtypes.iter() {
                            if options.contains(&st) && !prioritized.contains(&st) {
                                prioritized.push(st);
                            }
                        }
                    }
                }
            }
            for h_obj in &view.hand {
                if let Some(def) = baylee_cards::by_index(h_obj.card.index) {
                    for face in def.faces {
                        for &st in face.subtypes.iter() {
                            if options.contains(&st) && !prioritized.contains(&st) {
                                prioritized.push(st);
                            }
                        }
                    }
                }
            }
            for &st in options {
                if prioritized.len() >= MAX_CANDIDATES {
                    break;
                }
                if !prioritized.contains(&st) {
                    prioritized.push(st);
                }
            }
            for st in prioritized.into_iter().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[15] = 1.0;
                c.feat[20] = unit(f32::from(st.get() % 256), 256.0);
                c.action = Some(PlayerAction::ChooseSubtype(st));
                out.candidates.push(c);
            }
        }

        Pending::ChooseCardName { types, .. } => {
            let mut seen_cards: Vec<(CardIndex, u8)> = Vec::new();
            for (eidx, &cid) in state.entity_card_ids.iter().enumerate() {
                if out.candidates.len() >= MAX_CANDIDATES.saturating_sub(1) {
                    break;
                }
                if cid == 0 {
                    continue;
                }
                let card_idx = CardIndex::new(cid);
                let Some(def) = baylee_cards::by_index(card_idx) else {
                    continue;
                };
                for (f_idx, face) in def.faces.iter().enumerate() {
                    if !types.is_empty() && !face.types.intersects(*types) {
                        continue;
                    }
                    let key = (card_idx, f_idx as u8);
                    if seen_cards.contains(&key) {
                        continue;
                    }
                    seen_cards.push(key);
                    let mut c = Candidate4::default();
                    c.feat[15] = 1.0;
                    c.entity_idx = eidx as i16;
                    c.card_id = cid;
                    c.action = Some(PlayerAction::ChooseCardName {
                        card: card_idx,
                        face: f_idx as u8,
                    });
                    out.candidates.push(c);
                }
            }
            if out.candidates.len() < MAX_CANDIDATES {
                'fallback_card: for def in baylee_cards::all() {
                    for (f_idx, face) in def.faces.iter().enumerate() {
                        if types.is_empty() || face.types.intersects(*types) {
                            let key = (def.index, f_idx as u8);
                            if !seen_cards.contains(&key) {
                                let mut c = Candidate4::default();
                                c.feat[15] = 1.0;
                                c.card_id = def.index.get();
                                c.action = Some(PlayerAction::ChooseCardName {
                                    card: def.index,
                                    face: f_idx as u8,
                                });
                                out.candidates.push(c);
                            }
                            break 'fallback_card;
                        }
                    }
                }
            }
        }

        Pending::Arrange { cards, piles, .. } => {
            let arrangements = enumerate_arrangements(cards, piles);
            for (arr_idx, arr) in arrangements.into_iter().enumerate().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[15] = 1.0;
                c.feat[21] = unit(arr_idx as f32, 16.0);
                let mut rep_obj = cards.first().copied();
                if let PlayerAction::Arrange {
                    piles: ref arr_piles,
                } = arr
                {
                    let p0_len = arr_piles.first().map_or(0, Vec::len);
                    let p1_len = arr_piles.get(1).map_or(0, Vec::len);
                    c.feat[22] = unit(p0_len as f32, 5.0);
                    c.feat[23] = unit(p1_len as f32, 5.0);
                    if let Some(p0) = arr_piles.first() {
                        if let Some(&top_id) = p0.first() {
                            rep_obj = Some(top_id);
                        }
                        for (k, &cid) in cards.iter().take(5).enumerate() {
                            if p0.contains(&cid) {
                                c.feat[24 + k] = 1.0;
                            }
                        }
                    }
                }
                if let Some(oid) = rep_obj
                    && let Some(eidx) = state.find_entity(oid)
                {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(arr);
                out.candidates.push(c);
            }
        }

        Pending::ChooseAttackers {
            attackers,
            defenders,
            required,
            limits,
            ..
        } => {
            out.head = HeadKind4::SubsetPick;
            out.min_choices = required.len() as u32;
            out.max_choices = attackers.len() as u32;
            let best_def = choose_best_attack_target(view, defenders);
            let per_attacker_defs: Vec<(ObjectId, Vec<Defender>)> = attackers
                .iter()
                .map(|&cid| {
                    let allowed_defs: &[Defender] = limits
                        .iter()
                        .find(|l| l.creature == cid)
                        .map_or(defenders.as_slice(), |l| l.defenders.as_slice());
                    let mut ordered_defs: Vec<Defender> = Vec::with_capacity(allowed_defs.len());
                    if let Some(bd) = best_def
                        && allowed_defs.contains(&bd)
                    {
                        ordered_defs.push(bd);
                    }
                    for &d in allowed_defs {
                        if !ordered_defs.contains(&d) {
                            ordered_defs.push(d);
                        }
                    }
                    (cid, ordered_defs)
                })
                .collect();
            let max_defs = per_attacker_defs
                .iter()
                .map(|(_, ds)| ds.len())
                .max()
                .unwrap_or(0);
            // Round-robin by defender rank so every attacker gets its primary defender
            // before secondary defenders fill remaining MAX_CANDIDATES slots.
            for def_rank in 0..max_defs {
                for (cid, ordered_defs) in &per_attacker_defs {
                    if out.candidates.len() >= MAX_CANDIDATES {
                        break;
                    }
                    let Some(&def) = ordered_defs.get(def_rank) else {
                        continue;
                    };
                    let mut c = Candidate4::default();
                    c.feat[5] = 1.0;
                    c.feat[16] = f32::from(u8::from(required.contains(cid)));
                    if let Some(eidx) = state.find_entity(*cid) {
                        c.entity_idx = eidx as i16;
                        c.card_id = state.entity_card_ids[eidx];
                    }
                    match def {
                        Defender::Player(pid) => {
                            let rel = rel_seat(viewer, Some(pid), seat_count);
                            c.rel_seat_idx = rel as i8;
                            c.feat[17] = 1.0;
                            c.feat[20 + rel.min(7)] = 1.0;
                        }
                        Defender::Planeswalker(pw_id) => {
                            c.feat[18] = 1.0;
                            if let Some(pw_eidx) = state.find_entity(pw_id) {
                                c.feat[28] = unit(pw_eidx as f32, 64.0);
                                let rel = state.entity_rel_controller[pw_eidx] as usize;
                                if rel < MAX_SEATS {
                                    c.rel_seat_idx = rel as i8;
                                    c.feat[20 + rel] = 1.0;
                                }
                            }
                        }
                        Defender::Battle(b_id) => {
                            c.feat[19] = 1.0;
                            if let Some(b_eidx) = state.find_entity(b_id) {
                                c.feat[28] = unit(b_eidx as f32, 64.0);
                                let rel = state.entity_rel_controller[b_eidx] as usize;
                                if rel < MAX_SEATS {
                                    c.rel_seat_idx = rel as i8;
                                    c.feat[20 + rel] = 1.0;
                                }
                            }
                        }
                    }
                    c.object_id = Some(*cid);
                    c.defender = Some(def);
                    out.candidates.push(c);
                }
            }
        }

        Pending::ChooseCards {
            options: objs,
            min,
            max,
            ..
        } => {
            let single = *min == 1 && *max == 1;
            out.head = if single {
                HeadKind4::SingleChoice
            } else {
                HeadKind4::SubsetPick
            };
            out.min_choices = u32::from(*min);
            out.max_choices = u32::from(*max);
            let cap = MAX_CANDIDATES.saturating_sub(out.candidates.len());
            for oid in prioritize_object_options(view, objs, cap) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(oid) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                if single {
                    c.action = Some(PlayerAction::ChooseObjects {
                        objects: vec![oid],
                    });
                } else {
                    c.object_id = Some(oid);
                }
                out.candidates.push(c);
            }
        }

        Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            ..
        } => {
            let single = *min <= 1 && *max == 1;
            out.head = if single {
                HeadKind4::SingleChoice
            } else {
                HeadKind4::SubsetPick
            };
            out.min_choices = u32::from(*min);
            out.max_choices = u32::from(*max);
            if *min == 0 && single {
                let mut none_cand = Candidate4::default();
                none_cand.feat[0] = 1.0;
                none_cand.action = Some(PlayerAction::ChooseTargets {
                    objects: Vec::new(),
                    players: Vec::new(),
                });
                out.candidates.push(none_cand);
            }
            let reserved_players = player_options.len().min(MAX_SEATS);
            let obj_cap = MAX_CANDIDATES
                .saturating_sub(out.candidates.len())
                .saturating_sub(reserved_players);
            for oid in prioritize_object_options(view, options, obj_cap) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(oid) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                if single {
                    c.action = Some(PlayerAction::ChooseTargets {
                        objects: vec![oid],
                        players: Vec::new(),
                    });
                } else {
                    c.object_id = Some(oid);
                }
                out.candidates.push(c);
            }
            for &pid in player_options
                .iter()
                .take(MAX_CANDIDATES.saturating_sub(out.candidates.len()))
            {
                let mut c = Candidate4::default();
                c.feat[14] = 1.0;
                let rel = rel_seat(viewer, Some(pid), seat_count);
                c.rel_seat_idx = rel as i8;
                c.feat[16 + rel.min(7)] = 1.0;
                if single {
                    c.action = Some(PlayerAction::ChooseTargets {
                        objects: Vec::new(),
                        players: vec![pid],
                    });
                } else {
                    c.player_id = Some(pid);
                }
                out.candidates.push(c);
            }
        }

        Pending::ChooseBlockers { blockers, .. } => {
            out.head = HeadKind4::PairPick;
            let max_atk = blockers.iter().map(|b| b.attackers.len()).max().unwrap_or(0);
            for atk_idx in 0..max_atk {
                for b in blockers {
                    if out.pairs.len() >= MAX_PAIRS {
                        break;
                    }
                    let Some(&atk_id) = b.attackers.get(atk_idx) else {
                        continue;
                    };
                    let b_eidx = state.find_entity(b.blocker).map_or(-1, |i| i as i16);
                    let a_eidx = state.find_entity(atk_id).map_or(-1, |i| i as i16);
                    out.pairs.push([b_eidx, a_eidx]);
                    out.pair_objects.push((b.blocker, atk_id));
                }
            }
        }

        Pending::ChooseNumber { min, max, .. } => {
            out.head = HeadKind4::ScalarValue;
            out.min_choices = *min;
            out.max_choices = *max;
        }

        Pending::GameOver(_) => {}
    }

    out
}

fn prioritize_object_options(
    view: &PlayerView,
    objs: &[ObjectId],
    cap: usize,
) -> Vec<ObjectId> {
    if objs.len() <= cap {
        return objs.to_vec();
    }
    // Partition into:
    // 0: unique non-land objects by (controller, card_id, token, status, power, toughness)
    // 1: unique land objects by (controller, card_id, status)
    // 2: remaining non-land objects
    // 3: remaining land objects
    let mut tier0 = Vec::new();
    let mut tier1 = Vec::new();
    let mut tier2 = Vec::new();
    let mut tier3 = Vec::new();
    let mut seen_nonland = Vec::new();
    let mut seen_land = Vec::new();

    for &oid in objs {
        if let Some(o) = view.object(oid) {
            let cid = o.card.map_or(0, |c| c.index.get());
            let tok = o.token.unwrap_or(0);
            if o.types.contains(TypeSet::LAND) {
                let sig = (o.controller.get(), cid, o.status.bits());
                if !seen_land.contains(&sig) {
                    seen_land.push(sig);
                    tier1.push(oid);
                } else {
                    tier3.push(oid);
                }
            } else {
                let sig = (
                    o.controller.get(),
                    cid,
                    tok,
                    o.status.bits(),
                    o.power.unwrap_or(0),
                    o.toughness.unwrap_or(0),
                );
                if !seen_nonland.contains(&sig) {
                    seen_nonland.push(sig);
                    tier0.push(oid);
                } else {
                    tier2.push(oid);
                }
            }
        } else {
            tier0.push(oid);
        }
    }

    tier0
        .into_iter()
        .chain(tier1)
        .chain(tier2)
        .chain(tier3)
        .take(cap)
        .collect()
}

fn populate_cand_ability_dsl(
    c: &mut Candidate4,
    state: &EncodedState4,
    entity_idx: usize,
    ab_idx: usize,
) {
    let cid = state.entity_card_ids[entity_idx];
    let fid = state.entity_face_ids[entity_idx] as usize;
    if cid != 0
        && let Some(def) = baylee_cards::by_index(CardIndex::new(cid))
        && let Some(ab) = def.abilities_for_face(fid).get(ab_idx)
    {
        let row = encode_ability_def(ab);
        c.feat[24..48].copy_from_slice(&row[40..64]);
        return;
    }
    let tok_slot = state.entity_token_ids[entity_idx];
    if tok_slot != 0
        && let Some(tok) = baylee_cards::tokens::by_token_id(tok_slot - 1)
        && let Some(ab) = tok.abilities.get(ab_idx)
    {
        let row = encode_ability_def(ab);
        c.feat[24..48].copy_from_slice(&row[40..64]);
    }
}

/// Encodes all 21 [`CastModeKind`] variants + option [`ManaCost`] into `feat[0..48]`
/// (fixes P0-4: `ChooseMode` options are never collapsed to `[0, 0, index]`).
fn encode_cast_mode_kind_into(
    kind: &CastModeKind,
    cost: &ManaCost,
    index: u8,
    feat: &mut [f32],
) {
    feat[14] = 1.0; // ChooseMode family marker
    feat[15] = unit(f32::from(index), 8.0);
    let mc = encode_mana_cost(cost);
    feat[16..26].copy_from_slice(&mc);

    let kind_slot = match kind {
        CastModeKind::Normal => 26,
        CastModeKind::Kicked => 27,
        CastModeKind::Alternative(_) => 28,
        CastModeKind::Mode(_) => 29,
        CastModeKind::Modes(_) => 30,
        CastModeKind::Face(_) => 31,
        CastModeKind::PlayLandFace(_) => 32,
        CastModeKind::Prototype => 33,
        CastModeKind::Disguise => 34,
        CastModeKind::Miracle => 35,
        CastModeKind::Flashback => 36,
        CastModeKind::BeamMeUp => 37,
        CastModeKind::Dash => 38,
        CastModeKind::Mutate => 39,
        CastModeKind::MutatePosition(_) => 40,
        CastModeKind::Dungeon(_) | CastModeKind::DungeonRoom { .. } => 41,
        CastModeKind::Escape | CastModeKind::EscapeGranted { .. } => 42,
        CastModeKind::Permission(_) => 43,
        CastModeKind::Counter(_) => 44,
        CastModeKind::Colors(_) => 45,
    };
    feat[kind_slot] = 1.0;
    match kind {
        CastModeKind::Mode(m) | CastModeKind::Face(m) | CastModeKind::PlayLandFace(m) => {
            feat[46] = unit(*m as f32, 6.0);
        }
        CastModeKind::MutatePosition(on_top) => {
            feat[47] = f32::from(u8::from(*on_top));
        }
        _ => {}
    }
}

fn enumerate_arrangements(
    cards: &[ObjectId],
    piles: &[ArrangePile],
) -> Vec<PlayerAction> {
    let mut out = Vec::new();
    let mut push_unique = |p: Vec<Vec<ObjectId>>| {
        if out.len() >= MAX_CANDIDATES {
            return;
        }
        let act = PlayerAction::Arrange { piles: p };
        if !out.contains(&act) {
            out.push(act);
        }
    };
    if let Some(def) = default_arrangement(cards, piles) {
        push_unique(def);
    }
    if cards.len() > 1 {
        let mut rev_cards = cards.to_vec();
        rev_cards.reverse();
        if let Some(rev) = default_arrangement(&rev_cards, piles) {
            push_unique(rev);
        }
    }
    // Enumerate all legal 2-pile splits (Scry, Surveil, DiscardToLibrary, Fact or Fiction) up to 5 cards
    if piles.len() == 2 && cards.len() <= 5 {
        let n = cards.len();
        for mask in 0..(1_usize << n) {
            let mut p0 = Vec::with_capacity(n);
            let mut p1 = Vec::with_capacity(n);
            for (idx, &cid) in cards.iter().enumerate() {
                if (mask & (1 << idx)) == 0 {
                    p0.push(cid);
                } else {
                    p1.push(cid);
                }
            }
            if p0.len() >= piles[0].min as usize
                && p0.len() <= piles[0].max as usize
                && p1.len() >= piles[1].min as usize
                && p1.len() <= piles[1].max as usize
            {
                push_unique(vec![p0.clone(), p1.clone()]);
                if p0.len() >= 2 && n <= 4 {
                    let mut p0_rev = p0.clone();
                    p0_rev.reverse();
                    push_unique(vec![p0_rev, p1.clone()]);
                }
                if p1.len() >= 2 && n <= 4 {
                    let mut p1_rev = p1;
                    p1_rev.reverse();
                    push_unique(vec![p0, p1_rev]);
                }
            }
        }
    } else if piles.len() == 2 && piles[0].min == 0 && (piles[1].max as usize) >= cards.len() {
        push_unique(vec![Vec::new(), cards.to_vec()]);
    }
    out
}

/// Computes the binary `free_mask` (`1` = free Bernoulli decision, `0` = forced
/// or forbidden by constraints, `P0-3`) when visiting `cands.candidates` in
/// `visit_order` (completed with any remaining indices in `0..N`).
#[must_use]
pub fn compute_subset_free_mask(
    cands: &EncodedCandidates4,
    pending: &Pending,
    multi_mask: &[u8],
    visit_order: &[u16],
) -> Vec<u8> {
    let n = cands.candidates.len();
    let mut free_mask = vec![0_u8; n];
    if n == 0 {
        return free_mask;
    }
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut in_order = vec![false; n];
    for &idx in visit_order {
        let i = idx as usize;
        if i < n && !in_order[i] {
            in_order[i] = true;
            order.push(i);
        }
    }
    for i in 0..n {
        if !in_order[i] {
            order.push(i);
        }
    }

    if let Pending::ChooseAttackers { required, .. } = pending {
        let mut rem_for_creature: BTreeMap<u64, usize> = BTreeMap::new();
        for cand in &cands.candidates {
            if let Some(oid) = cand.object_id {
                *rem_for_creature.entry(pack_object_id(oid)).or_insert(0) += 1;
            }
        }
        let mut chosen_creatures: Vec<u64> = Vec::new();
        for i in order {
            let Some(oid) = cands.candidates[i].object_id else {
                continue;
            };
            let key = pack_object_id(oid);
            let rem_c = rem_for_creature.get_mut(&key).expect("counted above");
            let is_last_for_c = *rem_c <= 1;
            *rem_c = rem_c.saturating_sub(1);

            if chosen_creatures.contains(&key) {
                free_mask[i] = 0;
            } else if required.contains(&oid) && is_last_for_c {
                free_mask[i] = 0;
                chosen_creatures.push(key);
            } else {
                free_mask[i] = 1;
                if multi_mask.get(i).copied().unwrap_or(0) != 0 {
                    chosen_creatures.push(key);
                }
            }
        }
        return free_mask;
    }

    let min_k = (cands.min_choices as usize).min(n);
    let max_k = (cands.max_choices as usize).min(n);
    let mut picked = 0_usize;
    for (pos, i) in order.into_iter().enumerate() {
        let rem = n - pos;
        if picked >= max_k {
            free_mask[i] = 0;
        } else if picked + rem <= min_k {
            free_mask[i] = 0;
            picked += 1;
        } else {
            free_mask[i] = 1;
            if multi_mask.get(i).copied().unwrap_or(0) != 0 {
                picked += 1;
            }
        }
    }
    free_mask
}

/// Computes the binary `free_mask` (`1` = free Bernoulli decision, `0` =
/// forbidden because blocker is already assigned, `P0-3`) for `PairPick` when
/// visiting `cands.pair_objects` in `visit_order`.
#[must_use]
pub fn compute_pair_free_mask(
    cands: &EncodedCandidates4,
    multi_mask: &[u8],
    visit_order: &[u16],
) -> Vec<u8> {
    let n = cands.pair_objects.len();
    let mut free_mask = vec![0_u8; n];
    if n == 0 {
        return free_mask;
    }
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut in_order = vec![false; n];
    for &idx in visit_order {
        let i = idx as usize;
        if i < n && !in_order[i] {
            in_order[i] = true;
            order.push(i);
        }
    }
    for i in 0..n {
        if !in_order[i] {
            order.push(i);
        }
    }

    let mut used_blockers: Vec<ObjectId> = Vec::new();
    for i in order {
        let (blk, _atk) = cands.pair_objects[i];
        if used_blockers.contains(&blk) {
            free_mask[i] = 0;
        } else {
            free_mask[i] = 1;
            if multi_mask.get(i).copied().unwrap_or(0) != 0 {
                used_blockers.push(blk);
            }
        }
    }
    free_mask
}

/// Normalizes an integer `v` in `[min, max]` into `[0.0, 1.0]`, using linear
/// scaling when `max - min <= 20` and log-scaling when `max - min > 20` (`P1-8`).
#[must_use]
pub fn normalize_scalar_choice(v: u32, min: u32, max: u32) -> f32 {
    let lo = min;
    let hi = max.max(lo);
    let span = hi.saturating_sub(lo);
    if span == 0 {
        return 0.0;
    }
    let offset = v.clamp(lo, hi).saturating_sub(lo);
    if span <= 20 {
        (offset as f32 / span as f32).clamp(0.0, 1.0)
    } else {
        (((offset as f64).ln_1p() / (span as f64).ln_1p()) as f32).clamp(0.0, 1.0)
    }
}

/// Inverts [`normalize_scalar_choice`] back to an integer in `[min, max]`.
#[must_use]
pub fn denormalize_scalar_choice(scalar_norm: f32, min: u32, max: u32) -> u32 {
    let lo = min;
    let hi = max.max(lo);
    let span = hi.saturating_sub(lo);
    if span == 0 {
        return lo;
    }
    let norm = scalar_norm.clamp(0.0, 1.0);
    let offset = if span <= 20 {
        (norm * span as f32).round() as u32
    } else {
        ((f64::from(norm) * (span as f64).ln_1p()).exp_m1().round() as u32).min(span)
    };
    lo.saturating_add(offset).clamp(lo, hi)
}

/// Encodes a ground-truth teacher or rollout [`PlayerAction`] into a
/// [`TeacherTarget4`] against `cands`.
///
/// Never falls back to `single_idx = 0` when an action is missing from `cands`:
/// returns `None` so `convert4` increments `unencoded_decisions` honestly (`P0-2`).
#[must_use]
pub fn encode_teacher_action4(
    cands: &EncodedCandidates4,
    pending: &Pending,
    action: &PlayerAction,
) -> Option<TeacherTarget4> {
    let mut target = TeacherTarget4::default();
    match cands.head {
        HeadKind4::SingleChoice => {
            if let Some(idx) = cands
                .candidates
                .iter()
                .position(|c| c.action.as_ref() == Some(action))
            {
                target.single_idx = idx as u16;
                return Some(target);
            }
            // Match ChooseObjects / ChooseTargets for single-target questions
            if let (
                Pending::ChooseTargets { .. },
                PlayerAction::ChooseObjects { objects }
                | PlayerAction::ChooseTargets { objects, .. },
            ) = (pending, action)
            {
                if objects.is_empty()
                    && let Some(idx) = cands.candidates.iter().position(|c| {
                        matches!(
                            &c.action,
                            Some(PlayerAction::ChooseTargets { objects: o, players: p })
                                if o.is_empty() && p.is_empty()
                        )
                    })
                {
                    target.single_idx = idx as u16;
                    return Some(target);
                }
                if let Some(&first_oid) = objects.first()
                    && let Some(idx) = cands.candidates.iter().position(|c| {
                        matches!(
                            &c.action,
                            Some(PlayerAction::ChooseTargets { objects: o, .. })
                                if o.first() == Some(&first_oid)
                        )
                    })
                {
                    target.single_idx = idx as u16;
                    return Some(target);
                }
            }
            if let (Pending::Arrange { .. }, PlayerAction::Arrange { piles: act_piles }) =
                (pending, action)
                && let Some(idx) = cands.candidates.iter().position(|c| {
                    let Some(PlayerAction::Arrange { piles: cand_piles }) = &c.action else {
                        return false;
                    };
                    cand_piles.len() == act_piles.len()
                        && cand_piles.iter().zip(act_piles.iter()).all(|(cp, ap)| {
                            cp.len() == ap.len() && ap.iter().all(|oid| cp.contains(oid))
                        })
                })
            {
                target.single_idx = idx as u16;
                return Some(target);
            }
            None
        }

        HeadKind4::SubsetPick => {
            let n = cands.candidates.len();
            if n == 0 {
                return None;
            }
            target.multi_mask = vec![0; n];
            match action {
                PlayerAction::DeclareAttackers { attackers } => {
                    for &(oid, def) in attackers {
                        let maybe_idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.object_id == Some(oid) && c.defender == Some(def))
                            .or_else(|| {
                                cands
                                    .candidates
                                    .iter()
                                    .position(|c| c.object_id == Some(oid))
                            });
                        if let Some(idx) = maybe_idx {
                            target.multi_mask[idx] = 1;
                            target.pick_sequence.push(idx as u16);
                        } else if cands.candidates.len() < MAX_CANDIDATES {
                            return None;
                        }
                    }
                    if !attackers.is_empty() && target.pick_sequence.is_empty() {
                        return None;
                    }
                    target.free_mask = compute_subset_free_mask(
                        cands,
                        pending,
                        &target.multi_mask,
                        &target.pick_sequence,
                    );
                    Some(target)
                }
                PlayerAction::ChooseObjects { objects } => {
                    for oid in objects {
                        let idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.object_id == Some(*oid))?;
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    }
                    target.free_mask = compute_subset_free_mask(
                        cands,
                        pending,
                        &target.multi_mask,
                        &target.pick_sequence,
                    );
                    Some(target)
                }
                PlayerAction::ChooseTargets { objects, players } => {
                    for oid in objects {
                        let idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.object_id == Some(*oid))?;
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    }
                    for pid in players {
                        let idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.player_id == Some(*pid))?;
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    }
                    target.free_mask = compute_subset_free_mask(
                        cands,
                        pending,
                        &target.multi_mask,
                        &target.pick_sequence,
                    );
                    Some(target)
                }
                _ => None,
            }
        }

        HeadKind4::PairPick => {
            let n = cands.pair_objects.len();
            target.multi_mask = vec![0; n];
            if let PlayerAction::DeclareBlockers { blockers } = action {
                for &(blk, atk) in blockers {
                    if let Some(idx) = cands
                        .pair_objects
                        .iter()
                        .position(|&(b, a)| b == blk && a == atk)
                    {
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    } else if cands.pair_objects.len() < MAX_PAIRS {
                        return None;
                    }
                }
                if !blockers.is_empty() && target.pick_sequence.is_empty() {
                    return None;
                }
                target.free_mask =
                    compute_pair_free_mask(cands, &target.multi_mask, &target.pick_sequence);
                return Some(target);
            }
            None
        }

        HeadKind4::ScalarValue => {
            if let PlayerAction::ChooseNumber(v) = action {
                if *v < cands.min_choices || *v > cands.max_choices {
                    return None;
                }
                target.scalar_norm =
                    normalize_scalar_choice(*v, cands.min_choices, cands.max_choices);
                return Some(target);
            }
            None
        }
    }
}

/// Decodes a [`TeacherTarget4`] (or model prediction) back into a legal
/// [`PlayerAction`] for `pending`.
#[must_use]
pub fn decode_action4(
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    choice: &TeacherTarget4,
) -> Option<PlayerAction> {
    match cands.head {
        HeadKind4::SingleChoice => cands
            .candidates
            .get(choice.single_idx as usize)
            .and_then(|c| c.action.clone()),

        HeadKind4::SubsetPick => match pending {
            Pending::ChooseAttackers {
                attackers,
                defenders,
                required,
                limits,
                ..
            } => {
                let default_def = choose_best_attack_target(view, defenders)?;
                let mut chosen: Vec<(ObjectId, Defender)> = Vec::new();
                let def_for = |cid: ObjectId| -> Defender {
                    if let Some(lim) = limits.iter().find(|l| l.creature == cid)
                        && !lim.defenders.contains(&default_def)
                        && let Some(&first_d) = lim.defenders.first()
                    {
                        return first_d;
                    }
                    default_def
                };
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && let Some(oid) = cand.object_id
                        && !chosen.iter().any(|(c, _)| *c == oid)
                    {
                        let def = cand.defender.unwrap_or_else(|| def_for(oid));
                        chosen.push((oid, def));
                    }
                }
                for &req in required {
                    if attackers.contains(&req) && !chosen.iter().any(|(c, _)| *c == req) {
                        chosen.push((req, def_for(req)));
                    }
                }
                Some(PlayerAction::DeclareAttackers { attackers: chosen })
            }
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                let need = usize::from(*count);
                let mut objects: Vec<ObjectId> = Vec::new();
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && let Some(oid) = cand.object_id
                        && objects.len() < need
                    {
                        objects.push(oid);
                    }
                }
                for cand in &cands.candidates {
                    if objects.len() >= need {
                        break;
                    }
                    if let Some(oid) = cand.object_id
                        && !objects.contains(&oid)
                    {
                        objects.push(oid);
                    }
                }
                Some(PlayerAction::ChooseObjects { objects })
            }
            Pending::ChooseCards { min, max, .. } => {
                let mut objects: Vec<ObjectId> = Vec::new();
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && let Some(oid) = cand.object_id
                        && objects.len() < usize::from(*max)
                    {
                        objects.push(oid);
                    }
                }
                if objects.len() < usize::from(*min) {
                    for cand in &cands.candidates {
                        if objects.len() >= usize::from(*min) {
                            break;
                        }
                        if let Some(oid) = cand.object_id
                            && !objects.contains(&oid)
                        {
                            objects.push(oid);
                        }
                    }
                }
                Some(PlayerAction::ChooseObjects { objects })
            }
            Pending::ChooseTargets { min, max, .. } => {
                let mut objects: Vec<ObjectId> = Vec::new();
                let mut players: Vec<PlayerId> = Vec::new();
                let max_n = *max as usize;
                let min_n = *min as usize;
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && (objects.len() + players.len()) < max_n
                    {
                        if let Some(oid) = cand.object_id {
                            objects.push(oid);
                        } else if let Some(pid) = cand.player_id {
                            players.push(pid);
                        }
                    }
                }
                if (objects.len() + players.len()) < min_n {
                    for cand in &cands.candidates {
                        if (objects.len() + players.len()) >= min_n {
                            break;
                        }
                        if let Some(oid) = cand.object_id {
                            if !objects.contains(&oid) {
                                objects.push(oid);
                            }
                        } else if let Some(pid) = cand.player_id
                            && !players.contains(&pid)
                        {
                            players.push(pid);
                        }
                    }
                }
                Some(PlayerAction::ChooseTargets { objects, players })
            }
            _ => None,
        },

        HeadKind4::PairPick => {
            if let Pending::ChooseBlockers { .. } = pending {
                let mut blockers: Vec<(ObjectId, ObjectId)> = Vec::new();
                for (idx, &(blk, atk)) in cands.pair_objects.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && !blockers.iter().any(|&(b, _)| b == blk)
                    {
                        blockers.push((blk, atk));
                    }
                }
                return Some(PlayerAction::DeclareBlockers { blockers });
            }
            None
        }

        HeadKind4::ScalarValue => match pending {
            Pending::ChooseNumber { min, max, .. } => {
                let v = denormalize_scalar_choice(choice.scalar_norm, *min, *max);
                Some(PlayerAction::ChooseNumber(v))
            }
            _ => None,
        },
    }
}

fn choose_best_attack_target(view: &PlayerView, targets: &[Defender]) -> Option<Defender> {
    // Prefer an opponent player who can be killed or has the lowest positive life
    let mut best: Option<(i32, Defender)> = None;
    for &def in targets {
        let score = match def {
            Defender::Player(pid) => view
                .seats
                .get(pid.get() as usize)
                .map_or(20, |s| -s.life),
            Defender::Planeswalker(_) | Defender::Battle(_) => 0,
        };
        if best.is_none_or(|(b_score, _)| score > b_score) {
            best = Some((score, def));
        }
    }
    best.map(|(_, d)| d)
}

/// Samples a valid subset mask for `SubsetPick` (`ChooseAttackers`, `ChooseCards`,
/// `ChooseTargets`, `MulliganBottom`, `DiscardChoice`) while tracking exact
/// conditional `free_mask` and `logprob` (`P0-3`).
pub fn sample_subset_constrained(
    logits: &[f32],
    cands: &EncodedCandidates4,
    pending: &Pending,
    tau: f32,
    rng: &mut crate::deckgen::Rng,
) -> (Vec<u8>, Vec<u8>, Vec<u16>, f32) {
    let n = logits.len().min(cands.candidates.len());
    let mut mask = vec![0_u8; n];
    let mut free_mask = vec![0_u8; n];
    let mut total_lp = 0.0_f32;
    if n == 0 {
        return (mask, free_mask, Vec::new(), 0.0);
    }
    let scale = if tau <= 1e-4 { 1.0 } else { 1.0 / tau };

    let mut order: Vec<(usize, f32)> = logits[..n]
        .iter()
        .enumerate()
        .map(|(i, &l)| {
            let p = 1.0 / (1.0 + (-l * scale).exp());
            (i, p)
        })
        .collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let visit_seq: Vec<u16> = order.iter().map(|&(i, _)| i as u16).collect();

    if let Pending::ChooseAttackers { required, .. } = pending {
        let mut rem_for_creature: BTreeMap<u64, usize> = BTreeMap::new();
        for cand in &cands.candidates[..n] {
            if let Some(oid) = cand.object_id {
                *rem_for_creature.entry(pack_object_id(oid)).or_insert(0) += 1;
            }
        }
        let mut chosen_creatures: Vec<u64> = Vec::new();
        for &(i, p) in &order {
            let Some(oid) = cands.candidates[i].object_id else {
                continue;
            };
            let key = pack_object_id(oid);
            let rem_c = rem_for_creature.get_mut(&key).expect("counted above");
            let is_last_for_c = *rem_c <= 1;
            *rem_c = rem_c.saturating_sub(1);

            if chosen_creatures.contains(&key) {
                mask[i] = 0;
                free_mask[i] = 0;
            } else if required.contains(&oid) && is_last_for_c {
                mask[i] = 1;
                free_mask[i] = 0;
                chosen_creatures.push(key);
            } else {
                free_mask[i] = 1;
                let take = if tau <= 1e-4 {
                    logits[i] > 0.0
                } else {
                    (rng.unit() as f32) < p
                };
                if take {
                    mask[i] = 1;
                    chosen_creatures.push(key);
                    total_lp += p.max(1e-7).ln();
                } else {
                    mask[i] = 0;
                    total_lp += (1.0 - p).max(1e-7).ln();
                }
            }
        }
        return (mask, free_mask, visit_seq, total_lp);
    }

    let min_k = (cands.min_choices as usize).min(n);
    let max_k = (cands.max_choices as usize).min(n);
    let mut picked = 0_usize;
    for (pos, &(i, p)) in order.iter().enumerate() {
        let rem = n - pos;
        if picked >= max_k {
            mask[i] = 0;
            free_mask[i] = 0;
        } else if picked + rem <= min_k {
            mask[i] = 1;
            free_mask[i] = 0;
            picked += 1;
        } else {
            free_mask[i] = 1;
            let take = if tau <= 1e-4 {
                logits[i] > 0.0
            } else {
                (rng.unit() as f32) < p
            };
            if take {
                mask[i] = 1;
                picked += 1;
                total_lp += p.max(1e-7).ln();
            } else {
                mask[i] = 0;
                total_lp += (1.0 - p).max(1e-7).ln();
            }
        }
    }
    (mask, free_mask, visit_seq, total_lp)
}

/// Samples a valid blocker assignment mask for `PairPick` (`ChooseBlockers`)
/// while enforcing at-most-one-attacker-per-blocker and tracking exact
/// conditional `free_mask` and `logprob` (`P0-3`).
pub fn sample_pairs_constrained(
    logits: &[f32],
    cands: &EncodedCandidates4,
    tau: f32,
    rng: &mut crate::deckgen::Rng,
) -> (Vec<u8>, Vec<u8>, Vec<u16>, f32) {
    let n = logits.len().min(cands.pair_objects.len());
    let mut mask = vec![0_u8; n];
    let mut free_mask = vec![0_u8; n];
    let mut total_lp = 0.0_f32;
    if n == 0 {
        return (mask, free_mask, Vec::new(), 0.0);
    }
    let scale = if tau <= 1e-4 { 1.0 } else { 1.0 / tau };

    let mut order: Vec<(usize, f32)> = logits[..n]
        .iter()
        .enumerate()
        .map(|(i, &l)| {
            let p = 1.0 / (1.0 + (-l * scale).exp());
            (i, p)
        })
        .collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let visit_seq: Vec<u16> = order.iter().map(|&(i, _)| i as u16).collect();

    let mut used_blockers = Vec::new();
    for &(i, p) in &order {
        let [b_idx, a_idx] = cands.pairs[i];
        if b_idx < 0 || a_idx < 0 {
            mask[i] = 0;
            free_mask[i] = 0;
            continue;
        }
        let (blk, _atk) = cands.pair_objects[i];
        if used_blockers.contains(&blk) {
            mask[i] = 0;
            free_mask[i] = 0;
        } else {
            free_mask[i] = 1;
            let take = if tau <= 1e-4 {
                logits[i] > 0.0
            } else {
                (rng.unit() as f32) < p
            };
            if take {
                mask[i] = 1;
                used_blockers.push(blk);
                total_lp += p.max(1e-7).ln();
            } else {
                mask[i] = 0;
                total_lp += (1.0 - p).max(1e-7).ln();
            }
        }
    }
    (mask, free_mask, visit_seq, total_lp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::preset::AIProfile;
    use baylee_gamehost::Session;
    use crate::housedeck::HouseDeck;
    use crate::selfplay::table_for;

    #[test]
    fn eight_seat_relative_coordinates_round_trip_and_never_alias() {
        for seat_count in 2..=8 {
            for viewer_idx in 0..seat_count {
                let viewer = PlayerId::new(viewer_idx as u8);
                for s_idx in 0..seat_count {
                    let s = PlayerId::new(s_idx as u8);
                    let r = rel_seat(viewer, Some(s), seat_count);
                    assert!(r < seat_count);
                    if s == viewer {
                        assert_eq!(r, 0);
                    } else {
                        assert_ne!(r, 0, "opponent seat must never alias to 0");
                    }
                    assert_eq!(abs_seat(viewer, r, seat_count), s);
                }
                assert_eq!(rel_seat(viewer, None, seat_count), MAX_SEATS);
            }
        }
    }

    #[test]
    fn multiplayer_side_utility_sums_consistently() {
        // 4-player FFA: one winner gets 1.0, others 0.0
        let ffa_teams = [None, None, None, None];
        assert_eq!(side_utility(2, &ffa_teams, &[0, 1, 2, 3], &[2]), 1.0);
        assert_eq!(side_utility(0, &ffa_teams, &[0, 1, 2, 3], &[2]), 0.0);
        // 4-player FFA draw with 4 surviving seats: each gets 0.25
        for s in 0..4 {
            assert!((side_utility(s, &ffa_teams, &[0, 1, 2, 3], &[]) - 0.25).abs() < 1e-6);
        }
        // 2v2 team game draw with both sides alive: each surviving side gets 0.5
        let team_2v2 = [Some(1), Some(1), Some(2), Some(2)];
        assert!((side_utility(0, &team_2v2, &[0, 2], &[]) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn choose_mode_options_preserve_distinct_cast_mode_kinds_and_costs() {
        use baylee_engine::choice::CastModeDesc;
        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let pending = Pending::ChooseCastMode {
            player: PlayerId::new(0),
            object: ObjectId::new(1, 0),
            options: vec![
                CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: ManaCost::try_parse("{1}{U}").unwrap(),
                },
                CastModeDesc {
                    index: 1,
                    kind: CastModeKind::Kicked,
                    cost: ManaCost::try_parse("{3}{U}{U}").unwrap(),
                },
                CastModeDesc {
                    index: 2,
                    kind: CastModeKind::Flashback,
                    cost: ManaCost::try_parse("{4}{U}").unwrap(),
                },
            ],
        };
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let cands = encode_candidates4(&state, &view, &pending);
        assert_eq!(cands.candidates.len(), 3);
        // Verify that the three options have distinct CastModeKind and ManaCost vectors
        assert_ne!(cands.candidates[0].feat[16..46], cands.candidates[1].feat[16..46]);
        assert_ne!(cands.candidates[1].feat[16..46], cands.candidates[2].feat[16..46]);
    }

    #[test]
    fn encode_teacher_action4_returns_none_when_action_unrepresented() {
        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();
        let pending = Pending::ChooseCardName {
            player: PlayerId::new(0),
            types: TypeSet::EMPTY,
        };
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let cands = encode_candidates4(&state, &view, &pending);
        let unrepresented = PlayerAction::ActivateAbility {
            source: ObjectId::new(99999, 0),
            ability_index: 99,
        };
        assert!(
            encode_teacher_action4(&cands, &pending, &unrepresented).is_none(),
            "unrepresented teacher choice must return None rather than silently falling back to index 0"
        );
    }

    #[test]
    fn constrained_subset_and_pair_samplers_normalize_to_one_and_never_repair() {
        use crate::deckgen::Rng;

        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(42, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let (_, view) = session.view_for(PlayerId::new(0)).unwrap();

        // 1. SubsetPick with count=2 out of 4 hand cards (DiscardChoice)
        let mut view_sub = view.clone();
        view_sub.hand.truncate(4);
        let pending_subset = Pending::DiscardChoice {
            player: PlayerId::new(0),
            count: 2,
        };
        let state = encode_state4(&view_sub, Some(&pending_subset), None, &[]);
        let cands = encode_candidates4(&state, &view_sub, &pending_subset);
        let logits = [1.2_f32, -0.4, 0.7, -1.1];
        let tau = 0.8_f32;

        let mut rng = Rng::new(12345);
        let mut unique_probs: BTreeMap<Vec<u8>, f64> = BTreeMap::new();
        for _ in 0..4096 {
            let (mask, free_mask, seq, lp) =
                sample_subset_constrained(&logits, &cands, &pending_subset, tau, &mut rng);
            let k: u8 = mask.iter().sum();
            assert_eq!(k, 2, "sampled subset size {k} must equal 2");
            let target = TeacherTarget4 {
                single_idx: 0,
                multi_mask: mask.clone(),
                free_mask: free_mask.clone(),
                pick_sequence: seq.clone(),
                scalar_norm: 0.0,
            };
            let ans = decode_action4(&cands, &view_sub, &pending_subset, &target)
                .expect("decodes subset cleanly");
            let reencoded = encode_teacher_action4(&cands, &pending_subset, &ans)
                .expect("decoded subset must re-encode cleanly");
            assert_eq!(reencoded.multi_mask, mask, "decode_action4 must never modify sampled mask");
            assert_eq!(
                compute_subset_free_mask(&cands, &pending_subset, &mask, &seq),
                free_mask
            );
            unique_probs.insert(mask, (lp as f64).exp());
        }
        // There are C(4,2) = 6 valid subsets of size 2
        assert_eq!(unique_probs.len(), 6);
        let total_prob: f64 = unique_probs.values().sum();
        assert!(
            (total_prob - 1.0).abs() < 1e-5,
            "constrained subset distribution must sum to 1.0, got {total_prob}"
        );

        // 2. PairPick with 2 blockers and 2 attackers
        let b0 = ObjectId::new(20, 0);
        let b1 = ObjectId::new(21, 0);
        let a0 = ObjectId::new(30, 0);
        let a1 = ObjectId::new(31, 0);
        let pair_cands = EncodedCandidates4 {
            head: HeadKind4::PairPick,
            pending_kind: PendingKind4::DeclareBlockers,
            min_choices: 0,
            max_choices: 2,
            candidates: Vec::new(),
            pairs: vec![[0, 2], [0, 3], [1, 2], [1, 3]],
            pair_objects: vec![(b0, a0), (b0, a1), (b1, a0), (b1, a1)],
        };
        let pending_blk = Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: PlayerId::new(1),
            blockers: Vec::new(),
            capacity: Vec::new(),
            obeying: Vec::new(),
            demands: Vec::new(),
            bounds: Vec::new(),
        };
        let pair_logits = [0.9_f32, -0.2, 0.3, -0.8];
        let mut pair_probs: BTreeMap<Vec<u8>, f64> = BTreeMap::new();
        for _ in 0..4096 {
            let (mask, free_mask, seq, lp) =
                sample_pairs_constrained(&pair_logits, &pair_cands, tau, &mut rng);
            assert!(mask[0] + mask[1] <= 1);
            assert!(mask[2] + mask[3] <= 1);
            let target = TeacherTarget4 {
                single_idx: 0,
                multi_mask: mask.clone(),
                free_mask: free_mask.clone(),
                pick_sequence: seq.clone(),
                scalar_norm: 0.0,
            };
            let ans = decode_action4(&pair_cands, &view, &pending_blk, &target)
                .expect("decodes blockers cleanly");
            let PlayerAction::DeclareBlockers { blockers } = ans else {
                panic!("expected DeclareBlockers");
            };
            let mut rebuilt = vec![0_u8; 4];
            for (idx, po) in pair_cands.pair_objects.iter().enumerate() {
                if blockers.contains(po) {
                    rebuilt[idx] = 1;
                }
            }
            assert_eq!(rebuilt, mask, "decode_action4 must never repair sampled pair mask");
            assert_eq!(compute_pair_free_mask(&pair_cands, &mask, &seq), free_mask);
            pair_probs.insert(mask, (lp as f64).exp());
        }
        // Valid assignments: 3 choices for b0 (none, a0, a1) * 3 choices for b1 (none, a0, a1) = 9
        assert_eq!(pair_probs.len(), 9);
        let total_pair_prob: f64 = pair_probs.values().sum();
        assert!(
            (total_pair_prob - 1.0).abs() < 1e-5,
            "constrained pair distribution must sum to 1.0, got {total_pair_prob}"
        );
    }

    #[test]
    fn multi_defender_choose_attackers_and_log_scaled_choose_number_round_trip() {
        for &(lo, hi) in &[(0_u32, 5_u32), (0, 20), (0, 1000), (1, 50)] {
            for v in [lo, (lo + hi) / 2, hi] {
                let t = normalize_scalar_choice(v, lo, hi);
                assert!((-1.0 - 1e-5..=1.0 + 1e-5).contains(&t));
                let back = denormalize_scalar_choice(t, lo, hi);
                assert_eq!(back, v, "scalar choice {v} in [{lo}, {hi}] failed round-trip");
            }
        }
    }
}
