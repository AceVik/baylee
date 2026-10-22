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
/// hand, zones, and seat totals in [`PlayerView`], excluding transport sequence numbers
/// and wall-clock/presentation fields. Phase and priority changes are observations.
///
/// Used by [`SeatKnowledge4::observe`] so passive observation frames fire only
/// when public or seat-visible game state actually changes (`P0-4`).
///
/// # Panics
/// If serializing the view fails; the current view contains only JSON-safe types.
#[must_use]
pub fn board_observation_hash(view: &PlayerView) -> u64 {
    // Serialize the complete authorized view, not an incomplete hand-written
    // list of fields (which missed phases, colors of floating mana, faces,
    // combat, revealed library tops and several card-specific choices).
    let mut value = serde_json::to_value(view).expect("PlayerView serializes");
    if let Some(obj) = value.as_object_mut() {
        for key in [
            "seq",
            "decision_remaining_ms",
            "clocks",
            "lost",
            "policy_acts",
        ] {
            obj.remove(key);
        }
    }
    let bytes = serde_json::to_vec(&value).expect("PlayerView value serializes");
    bytes
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
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

/// A card revealed in an opponent's `hand/library/looking_at` that has not yet
/// been seen moving to a public zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownCardMemory4 {
    /// Owner seat (`PlayerId::get()`).
    pub owner_seat: u8,
    /// `CardIndex` of the revealed card.
    pub card_index: u32,
    /// Turn when last observed.
    pub last_seen_turn: u32,
    /// `1` = revealed in `hand/looking_at`, `2` = revealed on library top.
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
                if idx != UNKNOWN_CARD {
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
    #[allow(clippy::too_many_lines, reason = "One ordered update of public event counters and remembered cards")]
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
                    .saturating_add(delta.min(i32::from(u16::MAX)) as u16);
            } else if delta < 0 {
                self.seat_history[i].life_lost_total = self.seat_history[i]
                    .life_lost_total
                    .saturating_add((-delta).min(i32::from(u16::MAX)) as u16);
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
                    self.seat_history[c].creatures_entered_total = self.seat_history[c]
                        .creatures_entered_total
                        .saturating_add(1);
                }
            }
            self.known_cards.remove(&(obj.owner.get(), id));
        }
        for obj in view
            .graveyards
            .iter()
            .flatten()
            .chain(view.exile.iter().flatten())
        {
            self.known_cards
                .remove(&(obj.owner.get(), pack_object_id(obj.id)));
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
