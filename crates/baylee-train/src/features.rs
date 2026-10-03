//! What the trained AI is shown of one decision, as numbers: encoder v1.
//!
//! The actor half is built from the seat's [`PlayerView`] and the question it
//! was asked, nothing else — the same view the house AI answers from — so a
//! net trained on it can be run live from what a seat's socket carries. The
//! omniscient half ([`Omni`]) is what the view hides (every other hand, the
//! top of every library); it exists for the training-only critic and the
//! auxiliary heads and is written to its own files, never beside the actor's
//! inputs.
//!
//! A decision is a set of entity rows (one per object the seat can see, see
//! [`ENT_COLS`]) and one row of globals ([`GLOB_COLS`]). Numbers are kept raw
//! (`i16`, bit sets as their bits); the trainer normalises them. Where a row
//! points at another object (what an Aura enchants, what an attacker attacks,
//! what a spell targets) it names that object's *row* in the same decision,
//! never an `ObjectId`, which means nothing outside its game.
//!
//! Bump [`ENCODER_VERSION`] whenever a column changes meaning.

use std::collections::BTreeMap;

use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_engine::choice::Pending;
use baylee_engine::engine::Engine;
use baylee_engine::zone::ZoneLocation;
use baylee_gamehost::RegistryLookup;
use baylee_gamehost::view::{SeatContext, awaiting_for, deciding, owed_payment, player_view};
use baylee_view::{CounterKind, PlayerView, PublicObject, StackItem, TargetRef};

/// The encoder's version; a dataset names the one that wrote it.
pub const ENCODER_VERSION: u32 = 2;

/// Rows in the `CardIndex` ledger (`baylee_cards_index::ROWS`). A card's id
/// is its index plus one, so id 0 is padding.
pub const LEDGER_ROWS: u32 = 33_694;
/// The id of an object the seat may not identify: a face-down permanent,
/// an emblem, a copy-token of no registry token.
pub const UNKNOWN_ID: u32 = LEDGER_ROWS + 2;
/// A registry token's id is this plus its index in `baylee_cards::tokens::ALL`.
pub const TOKEN_BASE: u32 = LEDGER_ROWS + 3;
/// Ids the embedding table must hold: the ledger, the unknown id, and room
/// for 4096 registry tokens.
pub const ID_SPACE: u32 = TOKEN_BASE + 4096;

/// Entities kept per decision, the rest dropped from the back of
/// [`ZONE_ORDER`]. A board of hundreds of tokens is the game the cap is for.
pub const MAX_ENTITIES: usize = 256;

/// Zones, as column `zone` numbers them. 0 is padding.
pub mod zone {
    /// The seat's own hand.
    pub const HAND: i16 = 1;
    /// The stack.
    pub const STACK: i16 = 2;
    /// The battlefield.
    pub const BATTLEFIELD: i16 = 3;
    /// A command zone.
    pub const COMMAND: i16 = 4;
    /// Cards the seat is looking at (a search, a scry).
    pub const LOOKING: i16 = 5;
    /// A revealed top of a library.
    pub const LIBRARY_TOP: i16 = 6;
    /// A graveyard.
    pub const GRAVEYARD: i16 = 7;
    /// Exile.
    pub const EXILE: i16 = 8;
}

/// Which zones are kept first when a decision has more than
/// [`MAX_ENTITIES`] objects.
pub const ZONE_ORDER: [i16; 8] = [
    zone::HAND,
    zone::STACK,
    zone::BATTLEFIELD,
    zone::COMMAND,
    zone::LOOKING,
    zone::LIBRARY_TOP,
    zone::GRAVEYARD,
    zone::EXILE,
];

/// Entity columns, in order.
pub const ENT_COLS: [&str; 38] = [
    "zone",
    "controller_rel",
    "owner_rel",
    "status",
    "types",
    "supertypes",
    "colors",
    "mana_value",
    "power",
    "toughness",
    "base_power",
    "base_toughness",
    "flags",
    "loyalty",
    "damage",
    "plus_counters",
    "minus_counters",
    "lore_counters",
    "time_counters",
    "charge_counters",
    "other_counters",
    "attached_to",
    "attacks_player_rel",
    "attacks_row",
    "blocks_row",
    "stack_pos",
    "stack_kind",
    "target_row",
    "target_player_rel",
    "offered",
    "keywords0",
    "keywords1",
    "keywords2",
    "keywords3",
    "keywords4",
    "keywords5",
    "keywords6",
    "keywords7",
];

/// Bits of the `flags` column.
pub mod flag {
    /// The object has a power and toughness.
    pub const HAS_PT: i16 = 1;
    /// Summoning sick.
    pub const SICK: i16 = 1 << 1;
    /// A token.
    pub const TOKEN: i16 = 1 << 2;
    /// A commander.
    pub const COMMANDER: i16 = 1 << 3;
    /// Suspended.
    pub const SUSPENDED: i16 = 1 << 4;
    /// Attacking.
    pub const ATTACKING: i16 = 1 << 5;
    /// Blocking.
    pub const BLOCKING: i16 = 1 << 6;
    /// An attacker that has been blocked.
    pub const BLOCKED: i16 = 1 << 7;
    /// The seat knows which card this is.
    pub const KNOWN: i16 = 1 << 8;
}

/// Bits of the `offered` column: what the question lets the seat do with
/// this object right now.
pub mod offered {
    /// Play it as a land.
    pub const LAND: i16 = 1;
    /// Cast it.
    pub const CAST: i16 = 1 << 1;
    /// Activate one of its abilities.
    pub const ABILITY: i16 = 1 << 2;
    /// Activate one of its mana abilities.
    pub const MANA: i16 = 1 << 3;
    /// Choose it (a target, a card, an attacker, a blocker, a keep).
    pub const CHOOSE: i16 = 1 << 4;
    /// Already picked by the answer being given (an attacker declared, a
    /// target chosen), when an answer is taken one pick at a time.
    pub const PICKED: i16 = 1 << 5;
}

/// Seats the globals have room for, the deciding seat first.
pub const SEATS: usize = 4;

/// Per-seat global columns, repeated for relative seats 0..[`SEATS`].
pub const SEAT_COLS: [&str; 14] = [
    "present",
    "life",
    "poison",
    "energy",
    "hand",
    "library",
    "graveyard",
    "lost",
    "pool_w",
    "pool_u",
    "pool_b",
    "pool_r",
    "pool_g",
    "pool_c",
];

/// Global columns before the per-seat block.
pub const GLOB_HEAD: [&str; 12] = [
    "turn",
    "phase",
    "step",
    "active_rel",
    "i_am_asked",
    "seats",
    "priority_held",
    "monarch_rel",
    "day_night",
    "pending_kind",
    "n_options",
    "commander_damage_taken",
];

/// Width of the globals row.
pub const GLOB_WIDTH: usize = GLOB_HEAD.len() + SEATS * SEAT_COLS.len();

/// Question kinds, as `pending_kind` numbers them.
pub const PENDING_KINDS: [&str; 21] = [
    "mulligan",
    "mulligan_bottom",
    "priority",
    "choose_attackers",
    "choose_blockers",
    "discard",
    "legend",
    "choose_cards",
    "choose_targets",
    "choose_subtype",
    "choose_color",
    "yes_no",
    "cast_mode",
    "choose_number",
    "choose_player",
    "arrange",
    "game_over",
    "choose_card_name",
    "choose_pile",
    "choose_damage_effect",
    "allocate_prevention",
];

/// The question's kind and how many answers it has, counting a multi-pick
/// by its candidates (plus one for picking none where none is allowed).
#[must_use]
pub fn question(pending: &Pending, hand: usize) -> (i16, u32) {
    let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
    match pending {
        Pending::ChooseDamageEffect { options, .. } => (19, n(options.len())),
        Pending::AllocatePrevention { damage, .. } => (20, n(damage.len())),
        Pending::Mulligan { .. } => (0, 2),
        Pending::MulliganBottom { .. } => (1, n(hand)),
        Pending::Priority { legal, .. } => (
            2,
            n(usize::from(legal.can_pass)
                + legal.lands.len()
                + legal.castable.len()
                + legal.abilities.len()
                + legal.mana_abilities.len()
                + legal.suspendable.len()),
        ),
        Pending::ChooseAttackers { attackers, .. } => (3, n(attackers.len() + 1)),
        Pending::ChooseBlockers { blockers, .. } => (4, n(blockers.len() + 1)),
        Pending::DiscardChoice { .. } => (5, n(hand)),
        Pending::LegendChoice { options, .. } => (6, n(options.len())),
        Pending::ChooseCards { options, min, .. } => (7, n(options.len() + usize::from(*min == 0))),
        Pending::ChooseTargets {
            options,
            player_options,
            min,
            ..
        } => (
            8,
            n(options.len() + player_options.len() + usize::from(*min == 0)),
        ),
        Pending::ChooseSubtype { options, .. } => (9, n(options.len())),
        Pending::ChooseColor { options, .. } => (10, n(options.len())),
        Pending::YesNo { .. } => (11, 2),
        Pending::ChooseCastMode { options, .. } => (12, n(options.len())),
        Pending::ChooseNumber { min, max, .. } => (13, max.saturating_sub(*min).saturating_add(1)),
        Pending::ChoosePlayer { options, .. } => (14, n(options.len())),
        Pending::Arrange { cards, .. } => (15, n(cards.len())),
        Pending::GameOver(_) => (16, 0),
        Pending::ChooseCardName { .. } => (17, 1),
        Pending::ChoosePile { piles, .. } => (18, n(piles.len())),
    }
}

/// The view a house chair answers from, rebuilt from an engine: what
/// `Session::agent_view` builds, with no clock, no policy acts and no
/// teammate's hand.
#[must_use]
pub fn seat_view(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    pending: &Pending,
    seq: u64,
) -> PlayerView {
    player_view(
        engine.state(),
        seat,
        seq,
        Some(pending),
        &SeatContext {
            awaiting: awaiting_for(engine, seat),
            deciding: deciding(engine),
            held: engine.automation(seat).hold.suppresses(),
            owed: owed_payment(engine),
            library_reveal_blocked: engine.library_reveal_blocked(),
            decision_remaining_ms: None,
            policy_acts: &[],
        },
        &[],
    )
}

/// One decision, encoded for the actor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Encoded {
    /// Per entity: its id in the embedding table.
    pub cards: Vec<i32>,
    /// Per entity: [`ENT_COLS`] values.
    pub rows: Vec<[i16; ENT_COLS.len()]>,
    /// The globals row.
    pub globals: Vec<i16>,
    /// Objects past [`MAX_ENTITIES`] that were left out.
    pub dropped: usize,
    /// Of those, objects the question offered. Always 0 unless a question
    /// offers more objects than a decision has rows: offered objects are
    /// kept before any other.
    pub offered_dropped: usize,
    /// The question's kind ([`PENDING_KINDS`]).
    pub kind: i16,
    /// How many answers it has ([`question`]).
    pub options: u32,
    /// Each kept object's row.
    pub slots: BTreeMap<ObjectId, i16>,
}

/// A seat relative to the deciding one: 0 for itself, then around the table.
pub(crate) fn rel(seat: PlayerId, other: PlayerId, seats: usize) -> i16 {
    let n = seats.max(1);
    ((usize::from(other.get()) + n - usize::from(seat.get())) % n) as i16
}

pub(crate) fn clamp(v: i64) -> i16 {
    v.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16
}

/// The table id of a public object.
pub(crate) fn public_id(o: &PublicObject) -> (i32, bool) {
    match (&o.card, o.token) {
        (Some(card), _) => ((card.index.get() + 1) as i32, true),
        (None, Some(token)) => ((TOKEN_BASE + u32::from(token)) as i32, true),
        (None, None) => (UNKNOWN_ID as i32, false),
    }
}

/// An object the seat can see: a card in its hand, or a public object with
/// its position on the stack (0 elsewhere).
enum Seen<'a> {
    Hand(&'a baylee_view::HandObject),
    Public(&'a PublicObject, usize),
}

/// Encodes the decision `view` shows with `pending` asked, `picked` picked so
/// far by an answer given one pick at a time.
#[must_use]
#[allow(clippy::too_many_lines)] // one pass per zone, one column per line
pub fn encode(view: &PlayerView, pending: &Pending, picked: &crate::policy::Picked) -> Encoded {
    let seat = view.seat;
    let seats = view.seats.len();
    let (kind, options) = question(pending, view.hand.len());

    // What the question lets the seat do with each object.
    let mut offers: BTreeMap<ObjectId, i16> = BTreeMap::new();
    let mut offer = |id: ObjectId, bit: i16| *offers.entry(id).or_default() |= bit;
    match pending {
        Pending::Priority { legal, .. } => {
            for id in &legal.lands {
                offer(*id, offered::LAND);
            }
            for id in legal.castable.iter().chain(&legal.suspendable) {
                offer(*id, offered::CAST);
            }
            for (id, _) in &legal.abilities {
                offer(*id, offered::ABILITY);
            }
            for id in &legal.mana_abilities {
                offer(*id, offered::MANA);
            }
        }
        Pending::ChooseAttackers { attackers, .. } => {
            for id in attackers {
                offer(*id, offered::CHOOSE);
            }
        }
        Pending::ChooseBlockers { blockers, .. } => {
            for b in blockers {
                offer(b.blocker, offered::CHOOSE);
            }
        }
        Pending::LegendChoice { options, .. }
        | Pending::ChooseCards { options, .. }
        | Pending::ChooseTargets { options, .. } => {
            for id in options {
                offer(*id, offered::CHOOSE);
            }
        }
        Pending::Arrange { cards, .. } => {
            for id in cards {
                offer(*id, offered::CHOOSE);
            }
        }
        _ => {}
    }

    // Every object the seat can see, in keeping order.
    let mut seen: Vec<(i16, ObjectId, Seen<'_>)> = Vec::new();
    for h in &view.hand {
        seen.push((zone::HAND, h.id, Seen::Hand(h)));
    }
    for (pos, o) in view.stack.iter().enumerate() {
        seen.push((zone::STACK, o.id, Seen::Public(o, pos)));
    }
    let publics = [
        (
            zone::BATTLEFIELD,
            view.battlefield.iter().collect::<Vec<_>>(),
        ),
        (zone::COMMAND, view.command.iter().flatten().collect()),
        (zone::LOOKING, view.looking_at.iter().collect()),
        (zone::LIBRARY_TOP, view.library_tops.iter().collect()),
        (zone::GRAVEYARD, view.graveyards.iter().flatten().collect()),
        (zone::EXILE, view.exile.iter().flatten().collect()),
    ];
    for (z, objects) in publics {
        for o in objects {
            seen.push((z, o.id, Seen::Public(o, 0)));
        }
    }
    // Offered objects first, so a table of more than MAX_ENTITIES objects
    // loses only objects the answer cannot name. The order is otherwise
    // kept; the trunk has no positions, so order means only what is kept.
    let hand: Vec<ObjectId> = view.hand.iter().map(|h| h.id).collect();
    let offered_ids = crate::policy::offered_objects(pending, &hand);
    let (mut first, rest): (Vec<_>, Vec<_>) = seen
        .into_iter()
        .partition(|(_, id, _)| offered_ids.contains(id));
    first.extend(rest);
    let mut seen = first;
    let dropped = seen.len().saturating_sub(MAX_ENTITIES);
    let offered_dropped = seen
        .iter()
        .skip(MAX_ENTITIES)
        .filter(|(_, id, _)| offered_ids.contains(id))
        .count();
    seen.truncate(MAX_ENTITIES);
    // An object can be seen twice (looked at while on the stack); its first
    // row is the one others point at.
    let mut slot: BTreeMap<ObjectId, i16> = BTreeMap::new();
    for (i, (_, id, _)) in seen.iter().enumerate() {
        slot.entry(*id).or_insert(i as i16);
    }
    let row_of = |id: ObjectId| slot.get(&id).copied().unwrap_or(-1);

    let attacking: BTreeMap<ObjectId, &baylee_view::AttackerView> = view
        .combat
        .attackers
        .iter()
        .map(|a| (a.creature, a))
        .collect();
    let blocking: BTreeMap<ObjectId, ObjectId> = view
        .combat
        .blockers
        .iter()
        .map(|b| (b.blocker, b.attacker))
        .collect();

    let mut out = Encoded {
        dropped,
        offered_dropped,
        kind,
        options,
        ..Encoded::default()
    };
    for (z, id, object) in &seen {
        let mut r = [0_i16; ENT_COLS.len()];
        r[0] = *z;
        r[29] = offers.get(id).copied().unwrap_or(0)
            | if picked.objects.contains(id) {
                offered::PICKED
            } else {
                0
            };
        r[21] = -1;
        r[22] = -1;
        r[23] = -1;
        r[24] = -1;
        r[25] = -1;
        r[27] = -1;
        r[28] = -1;
        match object {
            Seen::Hand(h) => {
                out.cards.push((h.card.index.get() + 1) as i32);
                r[1] = 0;
                r[2] = 0;
                r[4] = h.types.bits() as i16;
                r[6] = i16::from(h.colors.bits());
                r[7] = clamp(i64::from(h.mana_value));
                r[12] = flag::KNOWN | if h.commander { flag::COMMANDER } else { 0 };
            }
            Seen::Public(o, pos) => {
                let (card, known) = public_id(o);
                out.cards.push(card);
                r[1] = rel(seat, o.controller, seats);
                r[2] = rel(seat, o.owner, seats);
                r[3] = i16::from(o.status.bits());
                r[4] = o.types.bits() as i16;
                r[5] = i16::from(o.supertypes.bits());
                r[6] = i16::from(o.colors.bits());
                r[7] = clamp(i64::from(o.mana_value));
                r[8] = o.power.unwrap_or(0);
                r[9] = o.toughness.unwrap_or(0);
                r[10] = o.base_power.unwrap_or(0);
                r[11] = o.base_toughness.unwrap_or(0);
                let mut flags = 0;
                if o.power.is_some() || o.toughness.is_some() {
                    flags |= flag::HAS_PT;
                }
                if o.summoning_sick {
                    flags |= flag::SICK;
                }
                if o.token.is_some() || (o.card.is_none() && known) {
                    flags |= flag::TOKEN;
                }
                if o.commander {
                    flags |= flag::COMMANDER;
                }
                if o.suspended {
                    flags |= flag::SUSPENDED;
                }
                if known {
                    flags |= flag::KNOWN;
                }
                if let Some(a) = attacking.get(id) {
                    flags |= flag::ATTACKING;
                    if a.blocked {
                        flags |= flag::BLOCKED;
                    }
                    match a.defending {
                        Defender::Player(p) => r[22] = rel(seat, p, seats),
                        Defender::Planeswalker(pw) => r[23] = row_of(pw),
                    }
                }
                if let Some(attacker) = blocking.get(id) {
                    flags |= flag::BLOCKING;
                    r[24] = row_of(*attacker);
                }
                r[12] = flags;
                r[13] = clamp(i64::from(o.loyalty.unwrap_or(0)));
                r[14] = clamp(i64::from(o.damage));
                for c in &o.counters {
                    let at = match c.kind {
                        CounterKind::Plus { .. } => 15,
                        CounterKind::Minus { .. } => 16,
                        CounterKind::Lore => 17,
                        CounterKind::Time => 18,
                        CounterKind::Charge => 19,
                        _ => 20,
                    };
                    r[at] = clamp(i64::from(r[at]) + i64::from(c.count));
                }
                r[21] = o.attached_to.map_or(-1, row_of);
                if *z == zone::STACK {
                    r[25] = clamp(*pos as i64);
                    r[26] = match o.stack_item {
                        Some(StackItem::Spell) => 1,
                        Some(_) => 2,
                        None => 0,
                    };
                }
                for t in &o.targets {
                    match t {
                        TargetRef::Object(target) if r[27] < 0 => r[27] = row_of(*target),
                        TargetRef::Player(p) if r[28] < 0 => r[28] = rel(seat, *p, seats),
                        _ => {}
                    }
                }
                for (k, col) in (30..38).enumerate() {
                    r[col] = (o.keywords >> (16 * k)) as u16 as i16;
                }
            }
        }
        out.rows.push(r);
    }

    // The globals.
    let mut g = vec![0_i16; GLOB_WIDTH];
    g[0] = clamp(i64::from(view.turn));
    g[1] = view.phase as i16;
    g[2] = view.step as i16;
    g[3] = rel(seat, view.active, seats);
    g[4] = i16::from(view.awaiting == Some(seat));
    g[5] = seats as i16;
    g[6] = i16::from(view.priority_held);
    g[7] = view.monarch.map_or(-1, |m| rel(seat, m, seats));
    g[8] = match view.day_night {
        None => 0,
        Some(baylee_view::DayNight::Day) => 1,
        Some(baylee_view::DayNight::Night) => 2,
    };
    g[9] = kind;
    g[10] = clamp(i64::from(options));
    for sv in &view.seats {
        let r = usize::try_from(rel(seat, sv.player, seats)).unwrap_or(0);
        if r >= SEATS {
            continue;
        }
        if r == 0 {
            g[11] = clamp(
                sv.commander_damage
                    .iter()
                    .map(|d| i64::from(d.amount))
                    .sum(),
            );
        }
        let base = GLOB_HEAD.len() + r * SEAT_COLS.len();
        let p = &sv.mana_pool;
        let values = [
            1,
            i64::from(sv.life),
            i64::from(sv.poison),
            i64::from(sv.energy),
            i64::from(sv.hand_count),
            i64::from(sv.library_count),
            i64::from(sv.graveyard_count),
            i64::from(sv.loss.is_some()),
            i64::from(p.white),
            i64::from(p.blue),
            i64::from(p.black),
            i64::from(p.red),
            i64::from(p.green),
            i64::from(p.colorless),
        ];
        for (k, v) in values.into_iter().enumerate() {
            g[base + k] = clamp(v);
        }
    }
    out.globals = g;
    out.slots = slot;
    out
}

/// What the seat's view hides, for training only: per other seat its hand,
/// and the top of every library.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Omni {
    /// Card ids.
    pub cards: Vec<i32>,
    /// Per card: `seat_rel * 16 + what`, `what` 1 = hand, 2 = library top
    /// (then `position` counts from the top).
    pub tags: Vec<i16>,
    /// Per card: its position (library depth; 0 in a hand).
    pub positions: Vec<i16>,
}

/// Cards of each library's top that [`omniscient`] keeps.
pub const OMNI_LIBRARY_TOP: usize = 8;

/// The hidden part of the game `engine` is in, as `seat` would not see it.
#[must_use]
pub fn omniscient(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Omni {
    let state = engine.state();
    let seats = state.players.len();
    let mut omni = Omni::default();
    let id_of = |id: ObjectId| {
        state
            .object(id)
            .and_then(|o| o.card)
            .map_or(UNKNOWN_ID as i32, |c| (c.index.get() + 1) as i32)
    };
    for p in 0..seats {
        let player = PlayerId::new(p as u8);
        let r = rel(seat, player, seats);
        if player != seat {
            for id in state.zones.list(ZoneLocation::Hand(player)) {
                omni.cards.push(id_of(*id));
                omni.tags.push(r * 16 + 1);
                omni.positions.push(0);
            }
        }
        let library = state.zones.list(ZoneLocation::Library(player));
        // The library's top is its last element.
        for (depth, id) in library.iter().rev().take(OMNI_LIBRARY_TOP).enumerate() {
            omni.cards.push(id_of(*id));
            omni.tags.push(r * 16 + 2);
            omni.positions.push(depth as i16);
        }
    }
    omni
}
