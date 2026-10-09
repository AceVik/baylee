//! Building a game from its preset, and the objects it is made of.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl GameState {
    /// Builds a game from a preset: seats, decks, shuffles, opening hands,
    /// starting battlefield, emblems.
    ///
    /// # Errors
    /// [`SetupError::Preset`] for structural violations,
    /// [`SetupError::UnknownCard`] for unresolvable deck entries.
    ///
    /// # Panics
    /// Internal invariant violations (freshly created objects are always present).
    #[allow(clippy::too_many_lines)] // setup is a linear checklist; extraction would obscure it
    pub fn from_preset(preset: &GamePreset, lookup: &impl CardLookup) -> Result<Self, SetupError> {
        preset.validate()?;
        for placement in &preset.house_rules.starting_counters {
            for counter in &placement.counters {
                if CounterKind::from_setup_name(&counter.kind).is_none() {
                    return Err(SetupError::UnknownCounter(counter.kind.clone()));
                }
            }
        }
        let default_life = match preset.format {
            FormatId::Commander => 40,
            _ => 20,
        };
        let seats = preset.seats.len() as u8;
        let mut state = Self {
            arena: Arena::with_capacity(512),
            zones: Zones::new(preset.seats.len()),
            players: preset
                .seats
                .iter()
                .enumerate()
                .map(|(i, s)| Player {
                    id: PlayerId::new(i as u8),
                    life: s.starting_life.unwrap_or(default_life),
                    poison: 0,
                    energy: 0,
                    enduring_story: false,
                    citys_blessing: false,
                    mana_pool: ManaPool::new(),
                    hand_modifier: 0,
                    lands_played_this_turn: 0,
                    turn_start_timestamp: 0,
                    tried_empty_draw: false,
                    commander_damage: Vec::new(),
                    loss: None,
                    team: s.team,
                })
                .collect(),
            turn: TurnInfo::new(PlayerId::new(0)),
            combat: crate::combat::CombatState::default(),
            per_turn: PerTurn::new(preset.seats.len()),
            graveyard_order: crate::graveyard_order::Ordering::default(),
            sba_legend_decisions: Vec::new(),
            delayed: Vec::new(),
            counter_links: Vec::new(),
            ltb_versions: Vec::new(),
            damage_deaths: Vec::new(),
            pending_miracle: std::collections::VecDeque::new(),
            draws_to_offer: std::collections::VecDeque::new(),
            discard_answers: Vec::new(),
            discards_on_top: Vec::new(),
            extra_turns: std::collections::VecDeque::new(),
            resume_after: None,
            skip_followups: Vec::new(),
            reanimated_auras: Vec::new(),
            reanimation_finishes: Vec::new(),
            restriction_info: rustc_hash::FxHashMap::default(),
            next_restriction_id: 1,
            commander_casts: vec![0; preset.seats.len()],
            commander_redirect: Vec::new(),
            pending_copied_faces: Vec::new(),
            ltb_mana_values: Vec::new(),
            ltb_controllers: Vec::new(),
            ltb_powers: Vec::new(),
            ltb_abilities: Vec::new(),
            ltb_counters: Vec::new(),
            ltb_characteristics: Vec::new(),
            ltb_attachments: Vec::new(),
            ceased: Vec::new(),
            reflexive: Vec::new(),
            discovered: Vec::new(),
            divided: Vec::new(),
            synthetic_copies: Vec::new(),
            commanders: vec![Vec::new(); preset.seats.len()],
            monarch: None,
            day_night: None,
            previous_turn: None,
            starting_player: PlayerId::new(0),
            ability_fires: rustc_hash::FxHashMap::default(),
            rng: GameRng::new(preset.seed),
            journal: Journal::default(),
            names: Names::default(),
            bases: Arc::default(),
            timestamp: 0,
            effects: crate::effects::EffectTable::default(),
            text_changes: crate::text_changes::TextChanges::default(),
            effect_text_overrides: Vec::new(),
            copy_snapshots: Vec::new(),
            numeric_failure: None,
            constrained_payments: Vec::new(),
            replacement_rules: Vec::new(),
            shields: crate::prevention::ShieldStore::default(),
            granted_actions: Vec::new(),
            next_granted_action: 0,
            next_damage_batch: 0,
            damage_sources: Vec::new(),
            source_memory: crate::sources::SourceMemory::default(),
            characteristics_generation: u64::MAX,
            projection_ids: Vec::new(),
            projected_cross_zone: false,
            printed_pt_cda: Vec::new(),
            token_cleanup: Vec::new(),
            snapshot_memo: SnapshotMemo::default(),
        };
        // Casting probes need the nameless face without mutating this interner.
        let nameless = state.names.intern("");
        debug_assert_eq!(nameless, NAMELESS);
        state.journal.record(GameEvent::GameStarted {
            seed: preset.seed,
            seats,
        });

        for (i, seat) in preset.seats.iter().enumerate() {
            let player = PlayerId::new(i as u8);
            // An `Open` seat is a human chair that no account has claimed yet,
            // not an absent player — every hosted game marks its human seat
            // `Open`, and that seat still needs a library and an opening hand.
            // Only a chair with nothing to set up is genuinely unoccupied,
            // which is the case `GamePreset::validate` allows an empty deck
            // for.
            let unoccupied = seat.deck.is_empty()
                && seat.starting_battlefield.is_empty()
                && seat.starting_hand.is_none()
                && seat.emblems.is_empty()
                && seat.commanders.is_empty();
            if unoccupied {
                continue;
            }
            // Emblems first (they exist from turn 0, CR 114.2).
            for emblem in &seat.emblems {
                let name = state.names.intern(emblem);
                state.create_bare(
                    player,
                    ObjectKind::Emblem,
                    name,
                    ZoneLocation::Command(player),
                );
            }
            // Commanders next (CR 903.6): they begin in the command zone,
            // and are never shuffled into the library — which is why they
            // are their own list on the seat and not a marked deck entry.
            for &entry in &seat.commanders {
                let id = state.create_card(player, entry, lookup)?;
                state
                    .move_object(
                        id,
                        ZoneLocation::Command(player),
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
                state.commanders[i].push(Commander {
                    object: id,
                    casts: 0,
                    answered: 0,
                });
            }
            for (position, &entry) in seat.starting_battlefield.iter().enumerate() {
                let id = state.create_card(player, entry, lookup)?;
                state.object_mut(id).expect("freshly created object").kind = ObjectKind::Permanent;
                state
                    .move_object(
                        id,
                        ZoneLocation::Battlefield,
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
                for placement in &preset.house_rules.starting_counters {
                    if placement.seat == i && placement.permanent == position {
                        for counter in &placement.counters {
                            if let Some(kind) = CounterKind::from_setup_name(&counter.kind) {
                                state
                                    .object_mut(id)
                                    .expect("freshly created object")
                                    .counters
                                    .add(kind, counter.amount);
                            }
                        }
                    }
                }
            }
            for &entry in &seat.deck {
                let id = state.create_card(player, entry, lookup)?;
                state
                    .move_object(
                        id,
                        ZoneLocation::Library(player),
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
            }
            state.shuffle_library(player);
            // The sideboard is created but never shuffled in. These cards are
            // outside the game (CR 400.11a) until a wish reaches them; folding
            // them into the library would silently make every deck bigger
            // than the one the player registered.
            for &entry in &seat.sideboard {
                let id = state.create_card(player, entry, lookup)?;
                state
                    .move_object(
                        id,
                        ZoneLocation::OutsideGame(player),
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
            }
            match &seat.starting_hand {
                Some(hand) => {
                    for &entry in hand {
                        let id = state.create_card(player, entry, lookup)?;
                        state
                            .move_object(
                                id,
                                ZoneLocation::Hand(player),
                                ZonePosition::Top,
                                Cause::Setup,
                            )
                            .expect("freshly created object");
                    }
                }
                None => {
                    state.draw_cards(player, 7);
                }
            }
        }
        Ok(state)
    }

    /// The shared printed face of a card, interned on first use.
    ///
    /// Front face only: an MDFC that turns over goes through
    /// [`GameState::switch_face`], which builds a face of its own.
    fn card_base(&mut self, def: &'static CardDef, index: CardIndex) -> Arc<Characteristics> {
        if let Some(base) = self.bases.cards.get(&index) {
            return Arc::clone(base);
        }
        Arc::make_mut(&mut self.bases).rules.insert(index, def);
        let name = self.names.intern(def.name());
        let base = Arc::new(Characteristics::from_face(def, 0, name));
        Arc::make_mut(&mut self.bases)
            .cards
            .insert(index, Arc::clone(&base));
        base
    }

    /// The shared blank face behind every card-less, token-less object:
    /// an ability on the stack, an emblem. A name and nothing else.
    ///
    /// This is the one that carries the million-ability stack: a deck that
    /// puts six figures of triggers up puts up a handful of *distinct*
    /// ones, so they all end up pointing here.
    pub fn bare_base(&mut self, name: NameRef) -> Arc<Characteristics> {
        if let Some(base) = self.bases.bare.get(&name) {
            return Arc::clone(base);
        }
        let base = Arc::new(Characteristics {
            name,
            mana_cost: baylee_core::mana::ManaCost::ZERO,
            colors: baylee_core::color::ColorSet::EMPTY,
            types: baylee_core::types::TypeSet::EMPTY,
            supertypes: baylee_core::types::SupertypeSet::EMPTY,
            subtypes: baylee_core::types::SubtypeSet::EMPTY,
            keywords: baylee_cards_dsl::KeywordSet::EMPTY,
            power: None,
            toughness: None,
            loyalty: None,
            color_identity: baylee_core::color::ColorSet::EMPTY,
            produced_colors: baylee_core::color::ColorSet::EMPTY,
            produced_colorless: false,
            produced_chosen: false,
            has_mana_ability: false,
            rules_text_lost: false,
            abilities_lost: None,
            front_mana_value: None,
        });
        Arc::make_mut(&mut self.bases)
            .bare
            .insert(name, Arc::clone(&base));
        base
    }

    /// The shared printed face of a token, at the size the effect asked for.
    ///
    /// `size` is `None` for the printed size and `Some(x)` when the effect
    /// computed one (Skyclave Apparition's Illusion is "X/X"); the two are
    /// different faces of the same definition and are interned apart.
    pub fn token_base(
        &mut self,
        token: &'static baylee_cards_dsl::TokenDef,
        size: Option<i16>,
    ) -> Arc<Characteristics> {
        // The definition is a `&'static` handed out by the card registry,
        // so its address identifies it. Nothing hashes or orders on this —
        // it is a lookup key inside one process — so a different address in
        // a different run cannot change a game.
        let key = (std::ptr::from_ref(token) as usize, size);
        if let Some(base) = self.bases.tokens.get(&key) {
            return Arc::clone(base);
        }
        let name = self.names.intern(token.name);
        let base = Arc::new(Characteristics {
            name,
            mana_cost: baylee_core::mana::ManaCost::ZERO,
            colors: token.colors,
            types: token.types,
            supertypes: token.supertypes,
            subtypes: baylee_core::types::SubtypeSet::from_slice(token.subtypes),
            keywords: token.keywords,
            power: size.or(token.power),
            toughness: size.or(token.toughness),
            loyalty: None,
            color_identity: baylee_core::color::ColorSet::EMPTY,
            produced_colors: baylee_core::color::ColorSet::EMPTY,
            produced_colorless: false,
            produced_chosen: false,
            has_mana_ability: false,
            rules_text_lost: false,
            abilities_lost: None,
            front_mana_value: None,
        });
        Arc::make_mut(&mut self.bases)
            .tokens
            .insert(key, Arc::clone(&base));
        base
    }

    fn create_card(
        &mut self,
        owner: PlayerId,
        entry: baylee_core::preset::DeckEntry,
        lookup: &impl CardLookup,
    ) -> Result<ObjectId, SetupError> {
        let def = lookup
            .card(entry.card)
            .ok_or(SetupError::UnknownCard(entry.card))?;
        self.graveyard_order.enabled |= crate::graveyard_order::card_reads_order(def);
        let base = self.card_base(def, entry.card);
        let card = CardRef {
            index: entry.card,
            print: entry.print,
        };
        self.timestamp += 1;
        let ts = self.timestamp;
        let id = self.arena.insert_with(|id| {
            let mut obj = GameObject::new_card(id, owner, card, base);
            obj.timestamp = ts;
            obj.controlled_since = ts;
            obj
        });
        if let Some(modifier) = printed_pt_cda(def) {
            self.printed_pt_cda.push((id, modifier));
        }
        Ok(id)
    }

    /// Creates a card-less object (tokens, emblems).
    ///
    /// # Panics
    /// On zone insertion failure (internal invariant).
    pub fn create_bare(
        &mut self,
        owner: PlayerId,
        kind: ObjectKind,
        name: NameRef,
        loc: ZoneLocation,
    ) -> ObjectId {
        let base = self.bare_base(name);
        self.timestamp += 1;
        let ts = self.timestamp;
        let id = self.arena.insert_with(|id| {
            let mut obj = GameObject::new_bare(id, owner, kind, base);
            obj.timestamp = ts;
            obj.controlled_since = ts;
            obj
        });
        self.zones.insert(
            id,
            loc,
            ZonePosition::Top,
            kind != ObjectKind::AbilityOnStack,
        );
        {
            let obj = self.arena.get_mut(id).expect("fresh object");
            obj.zone = loc.zone();
            obj.zone_owner = loc.player();
        }
        // Same rule as `move_object`: a card-less object created straight
        // into a zone that is not the battlefield or the stack is a
        // cleanup candidate. Emblems are card-less and live in the command
        // zone, which is why `sba::run` — not this call — decides.
        if !matches!(loc.zone(), Zone::Battlefield | Zone::Stack) {
            self.watch_token_cleanup(id);
        }
        id
    }
}
