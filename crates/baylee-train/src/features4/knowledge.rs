struct Fnv1aWriter(u64);

impl Fnv1aWriter {
    const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn feed(&mut self, bytes: &[u8]) {
        let mut h = self.0;
        for &b in bytes {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = h;
    }
}

impl std::io::Write for Fnv1aWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.feed(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Serialize)]
struct ObservableView<'a> {
    decision_player: &'a Option<PlayerId>,
    controlled_hands: &'a [baylee_view::SharedHand],
    damage_sources: &'a [baylee_view::DamageSourceView],
    target_objects: &'a [baylee_view::DamageSourceView],
    targeting: &'a Option<baylee_view::TargetingContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    casting: &'a Option<ObjectId>,
    seat: &'a PlayerId,
    turn: &'a u32,
    phase: &'a baylee_view::Phase,
    step: &'a baylee_view::Step,
    active: &'a PlayerId,
    awaiting: &'a Option<PlayerId>,
    deciding: &'a baylee_core::ids::SeatSet,
    priority_held: &'a bool,
    owed: &'a Option<baylee_core::mana::ManaPayment>,
    monarch: &'a Option<PlayerId>,
    day_night: &'a Option<baylee_view::DayNight>,
    seats: &'a [baylee_view::SeatView],
    hand: &'a [baylee_view::HandObject],
    shared_hands: &'a [baylee_view::SharedHand],
    hand_shared_with: &'a baylee_core::ids::SeatSet,
    hand_requests: &'a baylee_core::ids::SeatSet,
    hand_requested: &'a baylee_core::ids::SeatSet,
    battlefield: &'a [baylee_view::PublicObject],
    stack: &'a [baylee_view::PublicObject],
    graveyards: &'a [Vec<baylee_view::PublicObject>],
    exile: &'a [Vec<baylee_view::PublicObject>],
    command: &'a [Vec<baylee_view::PublicObject>],
    combat: &'a baylee_view::CombatView,
    looking_at: &'a [baylee_view::PublicObject],
    library_tops: &'a [baylee_view::PublicObject],
    sorcery_lock: &'a Option<ObjectId>,
    sorceries_have_flash: &'a bool,
}

impl<'a> ObservableView<'a> {
    fn from_view(view: &'a PlayerView) -> Self {
        let PlayerView {
            decision_player,
            controlled_hands,
            damage_sources,
            target_objects,
            targeting,
            casting,
            announced_cast: _,
            announced_x: _,
            seq: _,
            seat,
            turn,
            phase,
            step,
            active,
            awaiting,
            deciding,
            decision_remaining_ms: _,
            clocks: _,
            lost: _,
            priority_held,
            policy_acts: _,
            owed,
            monarch,
            day_night,
            seats,
            hand,
            shared_hands,
            hand_shared_with,
            hand_requests,
            hand_requested,
            battlefield,
            stack,
            graveyards,
            exile,
            command,
            combat,
            looking_at,
            library_tops,
            sorcery_lock,
            sorceries_have_flash,
        } = view;
        Self {
            decision_player,
            controlled_hands,
            damage_sources,
            target_objects,
            targeting,
            casting,
            seat,
            turn,
            phase,
            step,
            active,
            awaiting,
            deciding,
            priority_held,
            owed,
            monarch,
            day_night,
            seats,
            hand,
            shared_hands,
            hand_shared_with,
            hand_requests,
            hand_requested,
            battlefield,
            stack,
            graveyards,
            exile,
            command,
            combat,
            looking_at,
            library_tops,
            sorcery_lock,
            sorceries_have_flash,
        }
    }
}

/// Computes a deterministic 64-bit FNV-1a hash of the seat-authorized
/// [`PlayerView`] and [`Pending`] ONLY.
///
/// Used by `netplay4`, `arena`, and `league` to detect no-progress loops
/// without ever reading `Session::snapshot_hash()` (which hashes hidden library
/// and opponent hand state, P0-6).
#[must_use]
pub fn view_only_state_hash(view: &PlayerView, pending: &Pending) -> u64 {
    let mut writer = Fnv1aWriter::new();
    let _ = serde_json::to_writer(&mut writer, &ObservableView::from_view(view));
    let _ = serde_json::to_writer(&mut writer, pending);
    writer.0
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
    let mut writer = Fnv1aWriter::new();
    serde_json::to_writer(&mut writer, &ObservableView::from_view(view))
        .expect("ObservableView serializes");
    writer.0
}

/// SHA256 of the same Encoder-5/6/7 observation projection as
/// [`board_observation_hash`], excluding transport sequence and clock fields.
/// This additive audit identity does not change the observation clock or hashes.
/// Encoder-8/9 announcement context requires its own versioned projection.
///
/// # Panics
/// If serializing the observable projection fails; its types are JSON-safe.
#[must_use]
pub fn board_observation_sha256(view: &PlayerView) -> [u8; 32] {
    use sha2::Digest as _;
    let bytes = serde_json::to_vec(&ObservableView::from_view(view))
        .expect("ObservableView serializes");
    sha2::Sha256::digest(bytes).into()
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
    /// Exact own-deck and sideboard movement ledger (`InventoryLedger7`).
    pub inventory: crate::features7::inventory::InventoryLedger7,
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
    /// Turn when each currently-present battlefield permanent was first observed on the battlefield.
    bf_entered_turn: BTreeMap<u64, u32>,
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
    /// Initializes a socket player's knowledge from its own submitted cards.
    /// No opponent deck or engine state belongs to this interface.
    #[must_use]
    pub fn from_own_cards(viewer: PlayerId, seat_count: usize, cards: &[(u32, u8)]) -> Self {
        let mut knowledge = Self::empty(viewer, seat_count);
        let mut counts = BTreeMap::<u32, u8>::new();
        let mut counts_u32 = Vec::<(CardIndex, u32)>::new();
        for &(card, count) in cards {
            if card != UNKNOWN_CARD {
                let entry = counts.entry(card).or_default();
                *entry = entry.saturating_add(count);
                counts_u32.push((CardIndex::new(card), u32::from(count)));
            }
        }
        knowledge.own_deck_cards = counts.into_iter().collect();
        knowledge.inventory =
            crate::features7::inventory::InventoryLedger7::from_own_cards(viewer, counts_u32);
        knowledge.own_deck_summary = summarize_deck(&knowledge.own_deck_cards);
        knowledge
    }

    /// Initializes a socket player's knowledge from its full [`baylee_seat::DeckList`]
    /// (preserving `main`, `sideboard`, `commanders`, and exact `u32` counts).
    #[cfg(feature = "live-seat")]
    #[must_use]
    pub fn from_deck_list(
        viewer: PlayerId,
        seat_count: usize,
        deck: &baylee_seat::DeckList,
    ) -> Self {
        let mut knowledge = Self::empty(viewer, seat_count);
        let mut counts = BTreeMap::<u32, u8>::new();
        for entry in deck.main.iter().chain(&deck.commanders) {
            let cid = entry.card.get();
            if cid != UNKNOWN_CARD {
                let slot = counts.entry(cid).or_default();
                *slot = slot.saturating_add(u8::try_from(entry.count).unwrap_or(u8::MAX));
            }
        }
        knowledge.own_deck_cards = counts.into_iter().collect();
        knowledge.inventory = crate::features7::inventory::InventoryLedger7::from_sections(
            viewer,
            deck.main.iter().map(|e| (e.card, e.count)),
            deck.sideboard.iter().map(|e| (e.card, e.count)),
            deck.commanders.iter().map(|e| (e.card, e.count)),
        );
        knowledge.own_deck_summary = summarize_deck(&knowledge.own_deck_cards);
        knowledge
    }

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
        let inventory = crate::features7::inventory::InventoryLedger7::from_preset(preset, viewer)
            .unwrap_or_else(|_| {
                crate::features7::inventory::InventoryLedger7::from_own_cards(
                    viewer,
                    own_deck_cards
                        .iter()
                        .map(|&(c, n)| (CardIndex::new(c), u32::from(n))),
                )
            });
        Self {
            viewer,
            seat_count: preset.seats.len().clamp(1, MAX_SEATS),
            own_deck_cards,
            inventory,
            own_deck_summary,
            starting_life: [start_life; MAX_SEATS],
            last_life: [start_life; MAX_SEATS],
            last_turn: 0,
            seen_stack_ids: Vec::new(),
            seen_bf_ids: Vec::new(),
            bf_entered_turn: BTreeMap::new(),
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
            inventory: crate::features7::inventory::InventoryLedger7::empty(),
            own_deck_summary: [0.0; 24],
            starting_life: [20; MAX_SEATS],
            last_life: [20; MAX_SEATS],
            last_turn: 0,
            seen_stack_ids: Vec::new(),
            seen_bf_ids: Vec::new(),
            bf_entered_turn: BTreeMap::new(),
            known_cards: BTreeMap::new(),
            seat_history: [SeatHistoryStats4::default(); MAX_SEATS],
            cursor: 0,
            last_frame_digest: 0,
        }
    }

    /// Number of turns `id` has been continuously present on the battlefield.
    #[must_use]
    pub fn permanent_age_turns(&self, id: ObjectId) -> u32 {
        self.bf_entered_turn
            .get(&pack_object_id(id))
            .map_or(0, |&entered| self.last_turn.saturating_sub(entered))
    }

    /// Whether `id` was first observed on the battlefield during the current turn.
    #[must_use]
    pub fn entered_battlefield_this_turn(&self, id: ObjectId) -> bool {
        self.bf_entered_turn
            .get(&pack_object_id(id))
            .is_some_and(|&entered| entered == self.last_turn)
    }

    /// Updates `SeatKnowledge4` from a seat-authorized `PlayerView` and returns
    /// `true` if the observable state changed since the last call.
    pub fn observe(&mut self, view: &PlayerView) -> bool {
        self.observe_with_digest(view, board_observation_hash(view))
    }

    /// Encoder 8 observation clock, including the owner's announced spell even
    /// while a nested mana choice temporarily prevents cancelling that spell.
    pub fn observe_with_payment_context(&mut self, view: &PlayerView) -> bool {
        self.observe_with_digest(view, crate::features8::board_observation_hash8(view))
    }

    /// Encoder 9 observation clock, including authorized selected X.
    pub fn observe_with_announcement_context(&mut self, view: &PlayerView) -> bool {
        self.observe_with_digest(view, crate::features9::board_observation_hash9(view))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "One ordered update of public event counters and remembered cards"
    )]
    fn observe_with_digest(&mut self, view: &PlayerView, digest: u64) -> bool {
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
        // Track battlefield arrivals and continuous battlefield presence
        let mut current_bf = std::collections::BTreeSet::<u64>::new();
        for obj in &view.battlefield {
            let id = pack_object_id(obj.id);
            current_bf.insert(id);
            self.bf_entered_turn.entry(id).or_insert(view.turn);
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
        self.bf_entered_turn.retain(|id, _| current_bf.contains(id));
        self.inventory.observe(view);
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
