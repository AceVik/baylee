//! Game state: the complete, cloneable, hashable world.

use crate::arena::Arena;
use crate::event::{Cause, GameEvent, Journal};
use crate::object::{CardRef, Characteristics, CounterKind, GameObject, ObjectKind, Rider};
use crate::rng::GameRng;
use crate::turn::TurnInfo;
use crate::zone::{Zone, ZoneLocation, ZonePosition, Zones};
use baylee_cards_dsl::CardDef;
use baylee_core::ids::{CardIndex, Defender, NameRef, ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaPool, ManaSymbol};
use baylee_core::preset::{FormatId, GamePreset, PresetError};
use rustc_hash::FxHashMap;
use xxhash_rust::xxh3::Xxh3;

/// Registry seam: the engine resolves card definitions through this trait
/// and never depends on the compiled registry directly — a future runtime
/// card pack (custom cards, bosses) implements the same seam.
pub trait CardLookup {
    /// Resolves a card index to its definition.
    fn card(&self, index: CardIndex) -> Option<&'static CardDef>;
}

/// A seat's mutable state.
#[derive(Clone, Debug)]
pub struct Player {
    /// Seat handle.
    pub id: PlayerId,
    /// Life total.
    pub life: i32,
    /// Poison counters.
    pub poison: u16,
    /// Energy counters.
    pub energy: u16,
    /// Mana pool.
    pub mana_pool: ManaPool,
    /// Maximum hand size modifier (Reliquary Tower & co.).
    pub hand_modifier: i8,
    /// Lands played this turn (CR 305.2: one per turn).
    pub lands_played_this_turn: u8,
    /// Set when a draw was attempted from an empty library (SBA loses).
    pub tried_empty_draw: bool,
    /// Whether this player has lost (stays seated in multiplayer until
    /// CR 800.4 cleanup runs).
    pub has_lost: bool,
}

/// Deterministic name interner (rules identity, not display).
#[derive(Clone, Debug, Default)]
pub struct Names {
    map: FxHashMap<String, NameRef>,
    list: Vec<String>,
}

impl Names {
    /// Interns a name.
    pub fn intern(&mut self, name: &str) -> NameRef {
        if let Some(&id) = self.map.get(name) {
            return id;
        }
        let id = NameRef::new(self.list.len() as u32);
        let owned = name.to_string();
        self.list.push(owned.clone());
        self.map.insert(owned, id);
        id
    }

    /// Resolves a name.
    #[must_use]
    pub fn get(&self, id: NameRef) -> &str {
        &self.list[id.get() as usize]
    }

    /// Number of interned names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether no names are interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// A delayed trigger registered for a future game point (suspend finishes,
/// pact payments, rebound re-casts).
#[derive(Clone, Debug)]
pub struct DelayedTrigger {
    /// Controlling player.
    pub controller: PlayerId,
    /// When it fires.
    pub when: DelayedWhen,
    /// What it does.
    pub action: DelayedAction,
}

/// When a delayed trigger fires.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DelayedWhen {
    /// At the controller's next upkeep.
    NextUpkeep,
    /// At the controller's next first main phase (Mana Drain).
    NextFirstMain,
    /// At the beginning of the next end step (Venser +2).
    NextEndStep,
    /// At the controller's next cleanup.
    NextCleanup,
}

/// What a delayed trigger does.
#[derive(Clone, Debug)]
pub enum DelayedAction {
    /// Cast a card from exile without paying its mana cost (rebound,
    /// suspend finish).
    CastFromExileWithoutPaying {
        /// The card in exile.
        card: ObjectId,
    },
    /// Pay a cost or lose the game (Pact of Negation).
    PayCostOrLose {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
    },
    /// Pay a cost or sacrifice the permanent (echo).
    PayCostOrSacrifice {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
        /// The permanent to sacrifice when not paid.
        card: ObjectId,
    },
    /// Add mana (Mana Drain's next-main-phase mana).
    AddMana {
        /// Color.
        color: baylee_core::mana::ManaColor,
        /// Amount.
        amount: u16,
    },
    /// Return an exiled card to the battlefield under its owner's control
    /// (Venser +2).
    ReturnToBattlefield {
        /// The card in exile.
        card: ObjectId,
    },
}

/// Per-turn counters for conditional triggers (reset at every turn start).
#[derive(Clone, Debug)]
pub struct PerTurn {
    /// Noncreature spells cast this turn, per player.
    pub noncreature_spells: Vec<u32>,
    /// Cards drawn this turn, per player.
    pub draws: Vec<u32>,
    /// All spells cast this turn, per player (second-spell triggers).
    pub spells_cast: Vec<u32>,
    /// Whether each player lost life this turn (Luminarch Ascension).
    pub life_lost: Vec<bool>,
    /// Creatures that died this turn, all players (Emeritus of Woe's
    /// re-prepare condition).
    pub creatures_died: u32,
}

impl PerTurn {
    /// Zeroed counters for `players` seats.
    #[must_use]
    pub fn new(players: usize) -> Self {
        Self {
            noncreature_spells: vec![0; players],
            spells_cast: vec![0; players],
            life_lost: vec![false; players],
            creatures_died: 0,
            draws: vec![0; players],
        }
    }

    /// Resets all counters (called at every turn start).
    pub fn reset(&mut self) {
        self.noncreature_spells.iter_mut().for_each(|v| *v = 0);
        self.draws.iter_mut().for_each(|v| *v = 0);
        self.spells_cast.iter_mut().for_each(|v| *v = 0);
        self.life_lost.iter_mut().for_each(|v| *v = false);
        self.creatures_died = 0;
    }
}

/// A registered replacement rule from a permanent on the battlefield.
#[derive(Clone, Copy, Debug)]
pub struct ReplacementEntry {
    /// The source permanent.
    pub source: ObjectId,
    /// The rule's controller (for "you" in its filters).
    pub controller: PlayerId,
    /// The rule.
    pub rule: baylee_cards_dsl::ReplacementRule,
}

/// Setup failures.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    /// The preset is structurally invalid.
    #[error("invalid preset: {0}")]
    Preset(#[from] PresetError),
    /// A deck entry references a card the lookup cannot resolve.
    #[error("unknown card index {0}")]
    UnknownCard(CardIndex),
}

/// State operation failures.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// The object does not exist (anymore).
    #[error("no such object: {0}")]
    NoSuchObject(ObjectId),
}

/// The whole game world: cloneable for AI, hashable for determinism.
#[derive(Clone, Debug)]
pub struct GameState {
    /// All game objects.
    pub arena: Arena<GameObject>,
    /// Ordered zone contents.
    pub zones: Zones,
    /// Seats in turn order.
    pub players: Vec<Player>,
    /// Turn bookkeeping.
    pub turn: TurnInfo,
    /// Timestamp at which the current turn began (summoning sickness).
    pub turn_start_timestamp: u64,
    /// Journal sequence at the start of the current turn (per-turn event
    /// scans: "lost life this turn", …).
    pub turn_start_seq: u64,
    /// Combat phase state.
    pub combat: crate::combat::CombatState,
    /// Per-turn counters for conditional triggers (Esper Sentinel, Orcish
    /// Bowmasters): noncreature spells cast and cards drawn per player this
    /// turn, reset at every turn start.
    pub per_turn: PerTurn,
    /// Registered delayed triggers (suspend finishes, pact payments).
    pub delayed: Vec<DelayedTrigger>,
    /// First-of-turn drawn cards awaiting a miracle offer (CR 702.94).
    pub pending_miracle: std::collections::VecDeque<(PlayerId, ObjectId)>,
    /// Queued extra turns (CR 500.7); the front player takes the next
    /// turn instead of the normal successor.
    pub extra_turns: std::collections::VecDeque<PlayerId>,
    /// Restriction-id → (source, spell filter, spend rider) for
    /// restricted mana in players' pools (Cavern, Path of Ancestry).
    pub restriction_info: rustc_hash::FxHashMap<
        u32,
        (
            ObjectId,
            &'static baylee_cards_dsl::Filter,
            baylee_cards_dsl::SpendRider,
        ),
    >,
    /// Next restriction id to hand out (0 = unrestricted sentinel).
    pub next_restriction_id: u32,
    /// Times each player cast a commander from the command zone this
    /// game (Commander's Insight).
    pub commander_casts: Vec<u32>,
    /// The monarch designation (CR 718), if any.
    pub monarch: Option<PlayerId>,
    /// The player who took the first turn (Surgical Metamorph & co.).
    pub starting_player: PlayerId,
    /// Per-turn fire counts for once-per-turn triggers (reset each turn).
    pub ability_fires: rustc_hash::FxHashMap<(ObjectId, u32), u32>,
    /// Seeded randomness.
    pub rng: GameRng,
    /// The event journal.
    pub journal: Journal,
    /// Name interner.
    pub names: Names,
    /// Monotonic timestamp source (effects ordering).
    pub timestamp: u64,
    /// Registered continuous effects (anthems, type changes, pumps).
    pub effects: crate::effects::EffectTable,
    /// Registered replacement rules (Doubling Season, Panharmonicon, …).
    pub replacement_rules: Vec<ReplacementEntry>,
    /// The effect generation the characteristic caches were computed at.
    pub characteristics_generation: u64,
    /// Effect-set generation for characteristic caches (M2).
    pub effect_generation: u64,
    /// Scratch list reused by [`GameState::refresh_characteristics`].
    ///
    /// A refresh runs after every effect-set change, so allocating its
    /// working list each time is a per-effect malloc for the whole game.
    /// Always left empty, which keeps it free to clone.
    projection_ids: Vec<ObjectId>,
}

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
                    mana_pool: ManaPool::new(),
                    hand_modifier: 0,
                    lands_played_this_turn: 0,
                    tried_empty_draw: false,
                    has_lost: false,
                })
                .collect(),
            turn: TurnInfo::new(PlayerId::new(0)),
            turn_start_timestamp: 0,
            turn_start_seq: 0,
            combat: crate::combat::CombatState::default(),
            per_turn: PerTurn::new(preset.seats.len()),
            delayed: Vec::new(),
            pending_miracle: std::collections::VecDeque::new(),
            extra_turns: std::collections::VecDeque::new(),
            restriction_info: rustc_hash::FxHashMap::default(),
            next_restriction_id: 1,
            commander_casts: vec![0; preset.seats.len()],
            monarch: None,
            starting_player: PlayerId::new(0),
            ability_fires: rustc_hash::FxHashMap::default(),
            rng: GameRng::new(preset.seed),
            journal: Journal::default(),
            names: Names::default(),
            timestamp: 0,
            effects: crate::effects::EffectTable::default(),
            replacement_rules: Vec::new(),
            characteristics_generation: u64::MAX,
            effect_generation: 0,
            projection_ids: Vec::new(),
        };
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
                && seat.emblems.is_empty();
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
            for &entry in &seat.starting_battlefield {
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
            // outside the game (CR 400.1) until a wish reaches them; folding
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

    fn create_card(
        &mut self,
        owner: PlayerId,
        entry: baylee_core::preset::DeckEntry,
        lookup: &impl CardLookup,
    ) -> Result<ObjectId, SetupError> {
        let def = lookup
            .card(entry.card)
            .ok_or(SetupError::UnknownCard(entry.card))?;
        let name = self.names.intern(def.name());
        let base = Characteristics::from_face(def, 0, name);
        let card = CardRef {
            index: entry.card,
            print: entry.print,
        };
        self.timestamp += 1;
        let ts = self.timestamp;
        let id = self.arena.insert_with(|id| {
            let mut obj = GameObject::new_card(id, owner, card, base);
            obj.timestamp = ts;
            obj
        });
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
        let base = Characteristics {
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
        };
        self.timestamp += 1;
        let ts = self.timestamp;
        let id = self.arena.insert_with(|id| {
            let mut obj = GameObject::new_bare(id, owner, kind, base);
            obj.timestamp = ts;
            obj
        });
        self.zones.insert(id, loc, ZonePosition::Top);
        {
            let obj = self.arena.get_mut(id).expect("fresh object");
            obj.zone = loc.zone();
            obj.zone_owner = loc.player();
        }
        id
    }

    /// Monotonic timestamp (effects ordering, summoning sickness).
    pub fn next_timestamp(&mut self) -> u64 {
        self.timestamp += 1;
        self.timestamp
    }

    /// CR 302.6: a creature must have been controlled continuously since
    /// its controller's most recent turn began, so *any* control change
    /// makes it summoning-sick again — including the one at end of turn
    /// that hands a stolen creature back.
    fn restart_summoning_sickness(&mut self, id: ObjectId) {
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.timestamp = ts;
        }
    }

    /// Switches an object to another face of its card (MDFC cast/land
    /// play, CR 712.4): rebuilds base characteristics from the face and
    /// invalidates the layered projection cache.
    pub fn switch_face(&mut self, id: ObjectId, def: &CardDef, face: usize) {
        let face = face.min(def.faces.len() - 1);
        let name = self.names.intern(def.faces[face].name);
        let base = crate::object::Characteristics::from_face(def, face, name);
        if let Some(obj) = self.object_mut(id) {
            obj.face_index = face as u8;
            obj.base = base;
            obj.cache.clear();
        }
        // The projection is now stale for this object; the next refresh
        // has to rebuild it even if no effect was added or removed.
        self.characteristics_generation = u64::MAX;
    }

    /// Hot path: one generation compare. When stale, permanents and stack
    /// objects are re-projected through the layer system (CR 613).
    ///
    /// # Panics
    /// Internal invariant violations (zone objects always exist).
    pub fn refresh_characteristics(&mut self) {
        if self.characteristics_generation == self.effects.generation {
            return;
        }
        let generation = self.effects.generation;
        // Bucket and dependency-order the effect table ONCE for the whole
        // pass; the ordering does not depend on the object being projected
        // (see `layers::LayerPlan`).
        let plan = crate::layers::LayerPlan::build(&self.effects);
        // Cross-zone effects (Maskwood Nexus & co.) reach into library,
        // hand, graveyard — then every object must be projected, not only
        // battlefield + stack.
        let cross_zone = self
            .effects
            .iter()
            .any(|fx| matches!(fx.filter, crate::effects::EffectFilter::Dsl(f) if filter_reaches_other_zones(f)));
        // Scratch buffers live in the state so a refresh — which runs
        // after every single effect-set change — allocates nothing.
        let mut ids = std::mem::take(&mut self.projection_ids);
        ids.clear();
        if cross_zone {
            ids.extend(self.arena.iter().map(|(id, _)| id));
        } else {
            ids.extend(
                self.zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .chain(self.zones.list(ZoneLocation::Stack).iter())
                    .copied(),
            );
        }
        for &id in &ids {
            let Some(obj) = self.object(id) else {
                continue;
            };
            if !crate::layers::needs_projection(&plan, obj) {
                // Nothing can change this object's characteristics, so the
                // base is the projection. Dropping the cache is not just
                // cheaper than recomputing it — it is what keeps an
                // untouched board's per-object projection memory at zero.
                let obj = self.object_mut(id).expect("checked above");
                obj.cache.clear();
                // No effects means no layer 2 either: whoever the base
                // says controls it does, which is how a "gain control
                // until end of turn" hands the permanent back.
                let moved = obj.controller != obj.base_controller;
                obj.controller = obj.base_controller;
                if moved {
                    self.restart_summoning_sickness(id);
                }
                continue;
            }
            let projection = crate::layers::recompute_with(self, obj, &plan);
            let obj = self.object_mut(id).expect("zone object exists");
            let moved = obj.controller != projection.controller;
            obj.controller = projection.controller;
            // `cache` and `base` are disjoint fields, so this is one
            // mutable borrow and one shared borrow of the same object.
            let crate::object::GameObject { cache, base, .. } = obj;
            cache.store(generation, projection.characteristics, base);
            if moved {
                self.restart_summoning_sickness(id);
            }
        }
        ids.clear();
        self.projection_ids = ids;
        self.characteristics_generation = generation;
    }

    /// Object access.
    #[must_use]
    pub fn object(&self, id: ObjectId) -> Option<&GameObject> {
        self.arena.get(id)
    }

    /// Sets the monarch and releases monarch-linked exiles: when a player
    /// becomes monarch, cards exiled "until an opponent becomes monarch"
    /// (Palace Jailer) return if the new monarch is an opponent of the
    /// jailer's controller.
    pub fn set_monarch(&mut self, player: PlayerId) {
        let previous = self.monarch;
        self.monarch = Some(player);
        if previous == Some(player) {
            return;
        }
        // Monarch-link releases (Palace Jailer): return cards whose host's
        // controller is not the new monarch.
        let mut returning = Vec::new();
        for seat in 0..self.players.len() {
            let p = PlayerId::new(seat as u8);
            for &card in self.zones.list(ZoneLocation::Exile(p)) {
                if let Some(host) = self.object(card).and_then(|o| {
                    o.riders.iter().find_map(|r| match r {
                        crate::object::Rider::Linked { host } => Some(host),
                        _ => None,
                    })
                }) {
                    let host_controller = self.object(*host).map_or(player, |h| h.controller);
                    if host_controller != player {
                        returning.push(card);
                    }
                }
            }
        }
        for card in returning {
            if let Some(obj) = self.object_mut(card) {
                obj.kind = crate::object::ObjectKind::Permanent;
            }
            let _ = self.move_object(
                card,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }

    /// The battlefield as rules see it: phased-out permanents are treated
    /// as though they don't exist (CR 702.26).
    #[must_use]
    pub fn battlefield_view(&self) -> Vec<ObjectId> {
        self.zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|o| !o.status.contains(crate::object::Status::PHASED_OUT))
            })
            .collect()
    }

    /// Mutable object access.
    #[must_use]
    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut GameObject> {
        self.arena.get_mut(id)
    }

    /// Moves an object between zones (CR 400.7: `version` bumps — it
    /// becomes a new object for rules that track identity).
    ///
    /// # Errors
    /// [`StateError::NoSuchObject`] for stale or unknown handles.
    ///
    /// # Panics
    /// Internal invariant violations (existence is checked above).
    pub fn move_object(
        &mut self,
        id: ObjectId,
        to: ZoneLocation,
        pos: ZonePosition,
        cause: Cause,
    ) -> Result<ObjectId, StateError> {
        let (from_zone, from_player) = {
            let obj = self.object(id).ok_or(StateError::NoSuchObject(id))?;
            (obj.zone, obj.zone_owner.unwrap_or(obj.owner))
        };
        let from_loc = ZoneLocation::of(from_zone, from_player);
        self.zones.remove(id, from_loc);
        self.timestamp += 1;
        let ts = self.timestamp;
        {
            let obj = self.object_mut(id).expect("checked above");
            obj.zone = to.zone();
            obj.zone_owner = to.player();
            obj.timestamp = ts;
            obj.version = obj.version.wrapping_add(1);
            // CR 400.7: it becomes a new object. The old projection must
            // not survive the move — the refresh pass only revisits the
            // battlefield and the stack, so a creature that died under an
            // anthem would otherwise sit in the graveyard still pumped,
            // and every filter reading its power would agree.
            obj.cache.clear();
        }
        self.zones.insert(id, to, pos);
        // Creature deaths this turn (Emeritus of Woe's re-prepare).
        if from_zone == crate::zone::Zone::Battlefield && to.zone() == crate::zone::Zone::Graveyard
        {
            let is_creature = self.object(id).is_some_and(|o| {
                o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
            });
            if is_creature {
                self.per_turn.creatures_died = self.per_turn.creatures_died.saturating_add(1);
            }
        }
        self.journal.record(GameEvent::ZoneChanged {
            object: id,
            from: from_zone,
            to: to.zone(),
            cause,
        });
        Ok(id)
    }

    /// Shuffles a player's library (journaled).
    pub fn shuffle_library(&mut self, player: PlayerId) {
        let loc = ZoneLocation::Library(player);
        let rng = &mut self.rng;
        rng.shuffle(self.zones.list_mut(loc).as_mut_slice());
        self.journal.record(GameEvent::Shuffled {
            player,
            zone: Zone::Library,
        });
    }

    /// Moves the top `n` cards of a player's library to their hand.
    /// Drawing from an empty library flags [`Player::tried_empty_draw`] —
    /// the loss is a state-based action (CR 704.5b).
    pub fn draw_cards(&mut self, player: PlayerId, n: usize) -> Vec<ObjectId> {
        let mut drawn = Vec::with_capacity(n);
        let first_of_turn = self
            .per_turn
            .draws
            .get(player.get() as usize)
            .copied()
            .unwrap_or(0)
            == 0;
        for _ in 0..n {
            let Some(&top) = self.zones.list(ZoneLocation::Library(player)).last() else {
                if let Some(p) = self.players.get_mut(player.get() as usize) {
                    p.tried_empty_draw = true;
                }
                break;
            };
            if self
                .move_object(
                    top,
                    ZoneLocation::Hand(player),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .is_ok()
            {
                drawn.push(top);
            }
        }
        // Miracle (CR 702.94): the first card drawn this turn may be
        // revealed and cast for its miracle cost — the engine offers it.
        if first_of_turn && let Some(&card) = drawn.first() {
            self.pending_miracle.push_back((player, card));
        }
        if !drawn.is_empty() {
            if let Some(v) = self.per_turn.draws.get_mut(player.get() as usize) {
                *v = v.saturating_add(drawn.len() as u32);
            }
            self.journal.record(crate::event::GameEvent::CardsDrawn {
                player,
                count: drawn.len() as u16,
            });
        }
        drawn
    }

    /// Streaming xxh3 hash over the entire deterministic state.
    ///
    /// Caches and the journal *content* are excluded (they are derived
    /// data); everything that can influence future outcomes is included.
    #[must_use]
    pub fn snapshot_hash(&self) -> u64 {
        let mut h = Hasher::new();
        h.u64(self.timestamp);
        h.u64(self.turn_start_timestamp);
        h.u64(self.effect_generation);
        h.u64(self.characteristics_generation);
        for fx in self.effects.iter() {
            h.u32(fx.source.map_or(u32::MAX, baylee_core::ids::ObjectId::slot));
            h.u8(fx.controller.get());
            h.u8(fx.layer as u8);
            h.u64(fx.timestamp);
            h.u8(fx.duration as u8);
            hash_modifier(&mut h, &fx.modifier);
        }
        h.u32(self.turn.number);
        h.u8(self.turn.active.get());
        h.u8(self.turn.phase as u8);
        h.u8(self.turn.step as u8);
        h.bytes(&self.rng.seed());
        h.bytes(&self.rng.word_pos().to_le_bytes());
        h.usize(self.names.len());
        h.usize(self.players.len());
        for p in &self.players {
            h.u8(p.id.get());
            h.i32(p.life);
            h.u16(p.poison);
            h.u16(p.energy);
            h.i8(p.hand_modifier);
            h.boolean(p.has_lost);
            for color in ManaColor::ALL {
                h.u16(p.mana_pool.available(color));
            }
            h.usize(p.mana_pool.restricted().len());
            for r in p.mana_pool.restricted() {
                h.u8(r.color as u8);
                h.u16(r.amount);
                h.u8(r.flags.bits());
                h.u32(r.restriction.0);
            }
        }
        for (slot, generation, value) in self.arena.slots() {
            h.u32(slot);
            h.u8(generation);
            h.boolean(value.is_some());
            if let Some(obj) = value {
                hash_object(&mut h, obj);
            }
        }
        hash_zone(&mut h, self.zones.list(ZoneLocation::Battlefield));
        hash_zone(&mut h, self.zones.list(ZoneLocation::Stack));
        h.usize(self.combat.attackers.len());
        for a in &self.combat.attackers {
            h.u32(a.creature.slot());
            hash_defender(&mut h, a.defending, ObjectId::slot);
        }
        h.usize(self.combat.blockers.len());
        for b in &self.combat.blockers {
            h.u32(b.blocker.slot());
            h.u32(b.attacker.slot());
        }
        for seat in 0..self.players.len() {
            let p = PlayerId::new(seat as u8);
            for loc in [
                ZoneLocation::Library(p),
                ZoneLocation::Hand(p),
                ZoneLocation::Graveyard(p),
                ZoneLocation::Exile(p),
                ZoneLocation::Command(p),
            ] {
                hash_zone(&mut h, self.zones.list(loc));
            }
        }
        h.finish()
    }

    /// A hash of the *rules-visible situation*, blind to object identity and
    /// to time.
    ///
    /// [`Self::snapshot_hash`] answers "is this the same game state?" — it is
    /// what resync and replay compare, and it deliberately hashes object
    /// slots, generations and timestamps. That makes it useless for the
    /// question the loop detector asks. Slots are never recycled and
    /// timestamps only go up, so a permanent that dies and comes back is a
    /// different object at a later time: a genuine endless loop never hashes
    /// the same twice.
    ///
    /// This hashes what a player would see instead — who is where, with what
    /// characteristics, counters, damage and status — and reduces every
    /// object reference (attachments, targets, combat, an ability's source)
    /// to a position in a canonical ordering of the zones, so that a
    /// re-created permanent looks like the one it replaced.
    ///
    /// The turn *number* is left out for the same reason: a loop that spans a
    /// turn boundary would otherwise look different every time round.
    ///
    /// See [`crate::loops`] for how the detector uses it.
    #[must_use]
    pub fn loop_signature(&self) -> u64 {
        let zones = self.signature_zones();

        // Canonical position of every object, so references can be hashed
        // without their ids. Sorted by slot for a binary search; slots are
        // unique, so the mapping is exact.
        let mut by_slot: Vec<(u32, u32)> = Vec::new();
        for loc in &zones {
            for id in self.zones.list(*loc) {
                let position = by_slot.len() as u32;
                by_slot.push((id.slot(), position));
            }
        }
        by_slot.sort_unstable_by_key(|(slot, _)| *slot);
        let position = |id: ObjectId| -> u32 {
            by_slot
                .binary_search_by_key(&id.slot(), |(slot, _)| *slot)
                .map_or(u32::MAX, |i| by_slot[i].1)
        };

        let mut h = Hasher::new();
        h.u8(self.turn.active.get());
        h.u8(self.turn.phase as u8);
        h.u8(self.turn.step as u8);
        h.u8(self.monarch.map_or(255, PlayerId::get));
        h.usize(self.players.len());
        for p in &self.players {
            h.u8(p.id.get());
            h.i32(p.life);
            h.u16(p.poison);
            h.u16(p.energy);
            h.i8(p.hand_modifier);
            h.boolean(p.has_lost);
            for color in ManaColor::ALL {
                h.u16(p.mana_pool.available(color));
            }
        }
        for loc in &zones {
            let list = self.zones.list(*loc);
            h.usize(list.len());
            for id in list {
                match self.object(*id) {
                    Some(obj) => hash_object_situation(&mut h, obj, &position),
                    None => h.u8(0),
                }
            }
        }
        h.usize(self.combat.attackers.len());
        for a in &self.combat.attackers {
            h.u32(position(a.creature));
            hash_defender(&mut h, a.defending, position);
        }
        h.usize(self.combat.blockers.len());
        for b in &self.combat.blockers {
            h.u32(position(b.blocker));
            h.u32(position(b.attacker));
        }
        h.finish()
    }

    /// Every zone, in a fixed order — the canonical ordering object
    /// positions in [`Self::loop_signature`] are taken from.
    fn signature_zones(&self) -> Vec<ZoneLocation> {
        let mut locs = Vec::with_capacity(2 + self.players.len() * 6);
        locs.push(ZoneLocation::Battlefield);
        locs.push(ZoneLocation::Stack);
        for p in &self.players {
            locs.push(ZoneLocation::Library(p.id));
            locs.push(ZoneLocation::Hand(p.id));
            locs.push(ZoneLocation::Graveyard(p.id));
            locs.push(ZoneLocation::Exile(p.id));
            locs.push(ZoneLocation::Command(p.id));
            locs.push(ZoneLocation::OutsideGame(p.id));
        }
        locs
    }
}

struct Hasher {
    inner: Xxh3,
}

impl Hasher {
    fn new() -> Self {
        Self { inner: Xxh3::new() }
    }
    fn finish(self) -> u64 {
        self.inner.digest()
    }
    fn bytes(&mut self, b: &[u8]) {
        self.inner.update(b);
    }
    fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }
    fn i8(&mut self, v: i8) {
        self.bytes(&v.to_le_bytes());
    }
    fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    fn i16(&mut self, v: i16) {
        self.bytes(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
    fn u128(&mut self, v: u128) {
        self.bytes(&v.to_le_bytes());
    }
    fn usize(&mut self, v: usize) {
        self.bytes(&(v as u64).to_le_bytes());
    }
    fn boolean(&mut self, v: bool) {
        self.u8(u8::from(v));
    }
    fn option_u32(&mut self, v: Option<u32>) {
        match v {
            Some(x) => {
                self.u8(1);
                self.u32(x);
            }
            None => self.u8(0),
        }
    }
}

fn hash_zone(h: &mut Hasher, list: &[ObjectId]) {
    h.usize(list.len());
    for id in list {
        h.u32(id.slot());
        h.u8(id.generation());
    }
}

/// Hashes a defender. `locate` maps an object to whatever identity the
/// caller's hash is built on — the arena slot for the snapshot, a
/// canonical position for the loop signature.
///
/// The discriminant is hashed first so that a planeswalker in slot 3 and
/// the player with id 3 cannot collide.
fn hash_defender(h: &mut Hasher, defender: Defender, locate: impl Fn(ObjectId) -> u32) {
    match defender {
        Defender::Player(p) => {
            h.u8(0);
            h.u32(u32::from(p.get()));
        }
        Defender::Planeswalker(id) => {
            h.u8(1);
            h.u32(locate(id));
        }
    }
}

/// Hashes one object as a *situation*: everything a player could observe
/// about it, with object references reduced to canonical positions.
///
/// The identity fields `hash_object` includes — slot, generation — are
/// exactly what has to be left out here; see
/// [`GameState::loop_signature`].
fn hash_object_situation(h: &mut Hasher, obj: &GameObject, position: &impl Fn(ObjectId) -> u32) {
    h.u8(1);
    h.u8(obj.owner.get());
    h.u8(obj.controller.get());
    // Two boards that look identical but differ in who gets the permanent
    // back when a control effect ends are different situations.
    h.u8(obj.base_controller.get());
    h.u8(obj.zone as u8);
    h.u8(obj.zone_owner.map_or(255, PlayerId::get));
    h.u8(obj.kind as u8);
    h.u8(obj.face_index);
    match &obj.card {
        Some(c) => {
            h.u8(1);
            h.u32(c.index.get());
        }
        None => h.u8(0),
    }
    let b = &obj.base;
    h.u32(b.name.get());
    hash_mana_cost(h, &b.mana_cost);
    h.u8(b.colors.bits());
    h.u16(b.types.bits());
    h.u8(b.supertypes.bits());
    for word in b.subtypes.words() {
        h.u64(*word);
    }
    h.u128(b.keywords.bits());
    h.option_u32(b.power.map(|v| v as u32));
    h.option_u32(b.toughness.map(|v| v as u32));
    h.option_u32(b.loyalty.map(u32::from));
    let counters: Vec<_> = obj.counters.iter().collect();
    h.usize(counters.len());
    for (kind, n) in counters {
        h.u8(counter_tag(kind));
        h.u16(n);
    }
    h.u16(obj.damage);
    // Status and the deathtouch mark share one word; bit 8 is out of the
    // status byte, so packing them cannot collide.
    h.u16(u16::from(obj.status.bits()) | (u16::from(obj.deathtouched) << 8));
    h.option_u32(obj.attached_to.map(position));
    h.usize(obj.targets.len());
    for t in &obj.targets {
        h.u32(position(*t));
    }
    match &obj.ability {
        Some(loc) => {
            h.u8(1);
            h.u32(loc.card.get());
            h.u32(loc.index);
            h.u32(position(loc.source));
        }
        None => h.u8(0),
    }
}

fn hash_object(h: &mut Hasher, obj: &GameObject) {
    h.u32(obj.id.slot());
    h.u8(obj.id.generation());
    h.u8(obj.owner.get());
    h.u8(obj.controller.get());
    // Not derivable from the projected controller: it is who the permanent
    // goes back to when a control effect ends, so a resync that lost it
    // would hand the permanent to the wrong seat later.
    h.u8(obj.base_controller.get());
    h.u8(obj.zone as u8);
    h.u8(obj.zone_owner.map_or(255, baylee_core::ids::PlayerId::get));
    h.u8(obj.kind as u8);
    match &obj.card {
        Some(c) => {
            h.u8(1);
            h.u32(c.index.get());
            h.u16(c.print.get());
        }
        None => h.u8(0),
    }
    // Base characteristics (copiable values).
    let b = &obj.base;
    h.u32(b.name.get());
    hash_mana_cost(h, &b.mana_cost);
    h.u8(b.colors.bits());
    h.u16(b.types.bits());
    h.u8(b.supertypes.bits());
    for word in b.subtypes.words() {
        h.u64(*word);
    }
    h.u128(b.keywords.bits());
    h.option_u32(b.power.map(|v| v as u32));
    h.option_u32(b.toughness.map(|v| v as u32));
    h.option_u32(b.loyalty.map(u32::from));
    // Counters.
    let counters: Vec<_> = obj.counters.iter().collect();
    h.usize(counters.len());
    for (kind, n) in counters {
        h.u8(counter_tag(kind));
        h.u16(n);
    }
    h.u16(obj.damage);
    // Status and the deathtouch mark share one word; bit 8 is out of the
    // status byte, so packing them cannot collide.
    h.u16(u16::from(obj.status.bits()) | (u16::from(obj.deathtouched) << 8));
    h.option_u32(obj.attached_to.map(baylee_core::ids::ObjectId::slot));
    h.u64(obj.timestamp);
    h.u32(obj.version);
    // Targets + ability location.
    h.usize(obj.targets.len());
    for t in &obj.targets {
        h.u32(t.slot());
    }
    match &obj.ability {
        Some(loc) => {
            h.u8(1);
            h.u32(loc.card.get());
            h.u32(loc.index);
            h.u32(loc.source.slot());
        }
        None => h.u8(0),
    }
    // Exile riders.
    h.usize(obj.riders.len());
    for rider in &obj.riders {
        match rider {
            Rider::Linked { host } => {
                h.u8(1);
                h.u32(host.slot());
            }
            Rider::Rebound => h.u8(2),
            Rider::Adventure => h.u8(3),
            Rider::Foretold => h.u8(4),
            Rider::Plotted => h.u8(5),
            Rider::Suspend => h.u8(6),
            Rider::Flashback => h.u8(7),
            Rider::Uncounterable => h.u8(8),
            Rider::PlayableFromExileFor(p) => {
                h.u8(9);
                h.u8(p.get());
            }
            Rider::Prepared => h.u8(10),
        }
    }
}

/// Whether a DSL filter mentions non-battlefield zones (then its effect
/// needs cross-zone projection).
fn filter_reaches_other_zones(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::{Filter, ZoneRef};
    match filter {
        Filter::InZone(z) => !matches!(z, ZoneRef::Battlefield),
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reaches_other_zones),
        Filter::Not(f) => filter_reaches_other_zones(f),
        _ => false,
    }
}

/// Deterministic structural hash of a DSL filter (modifier payloads).
fn filter_hash(h: &mut Hasher, f: &baylee_cards_dsl::Filter) {
    use baylee_cards_dsl::Filter as F;
    match f {
        F::Any => h.u8(0),
        F::This => h.u8(1),
        F::Another => h.u8(2),
        F::And(parts) | F::Or(parts) => {
            h.u8(if matches!(f, F::And(_)) { 3 } else { 4 });
            for p in *parts {
                filter_hash(h, p);
            }
        }
        F::Not(inner) => {
            h.u8(5);
            filter_hash(h, inner);
        }
        F::HasType(t) => {
            h.u8(6);
            h.u16(t.bits());
        }
        F::LacksType(t) => {
            h.u8(7);
            h.u16(t.bits());
        }
        F::HasSupertype(s) => {
            h.u8(8);
            h.u8(s.bits());
        }
        F::HasSubtype(s) => {
            h.u8(9);
            h.u16(s.get());
        }
        F::HasColor(c) => {
            h.u8(10);
            h.u8(c.bits());
        }
        F::IsColorless => h.u8(11),
        F::Monocolored => h.u8(12),
        F::IsToken => h.u8(13),
        F::ControlledByYou => h.u8(14),
        F::ControlledByOpponent => h.u8(15),
        F::OwnedByYou => h.u8(16),
        F::Tapped => h.u8(17),
        F::Untapped => h.u8(18),
        F::Attacking => h.u8(19),
        F::MatchesChosenTypeOfSource => h.u8(20),
        F::AttachedToBySource => h.u8(25),
        F::SharesSubtypeWithCommander => h.u8(27),
        F::ToughnessAtMost(n) => {
            h.u8(26);
            h.i16(*n);
        }
        F::HasKeyword(k) => {
            h.u8(21);
            h.u128(k.bits());
        }
        F::CmcAtMost(n) | F::CmcAtLeast(n) => {
            h.u8(if matches!(f, F::CmcAtMost(_)) { 22 } else { 23 });
            h.u32(*n);
        }
        F::InZone(z) => {
            h.u8(24);
            h.u8(*z as u8);
        }
    }
}

fn hash_modifier(h: &mut Hasher, m: &baylee_cards_dsl::Modifier) {
    use baylee_cards_dsl::Modifier as M;
    match m {
        M::AddType(t) => {
            h.u8(1);
            h.u16(t.bits());
        }
        M::RemoveType(t) => {
            h.u8(2);
            h.u16(t.bits());
        }
        M::AddSubtype(s) => {
            h.u8(3);
            h.u16(s.get());
        }
        M::AllCreatureTypes => h.u8(4),
        M::AllBasicLandTypes => h.u8(13),
        M::AddColor(c) => {
            h.u8(5);
            h.u8(c.bits());
        }
        M::SetColor(c) => {
            h.u8(6);
            h.u8(c.bits());
        }
        M::AddKeyword(k) => {
            h.u8(7);
            h.u128(k.bits());
        }
        M::RemoveKeyword(k) => {
            h.u8(8);
            h.u128(k.bits());
        }
        M::LoseKeywords => h.u8(9),
        M::LegendRuleOff => h.u8(14),
        M::CantActivateArtifacts => h.u8(15),
        M::OpponentsCastAsSorcery => h.u8(16),
        M::PlayersCantLose => h.u8(17),
        M::CantLoseLife => h.u8(18),
        M::PreventDamageToIt => h.u8(19),
        M::PreventDamageFromIt => h.u8(20),
        M::OpponentsCantSearch => h.u8(21),
        M::NoMaxHandSize => h.u8(22),
        M::ProtectionFrom(f) => {
            h.u8(23);
            filter_hash(h, f);
        }
        M::BecomeCopyOf(id) => {
            h.u8(24);
            h.u32(id.slot());
        }
        M::GrantsFlashback => h.u8(25),
        M::PlayerHexproof => h.u8(26),
        M::GainControl => h.u8(35),
        M::SorceriesHaveFlash => h.u8(29),
        M::GrantTriggered { .. } => h.u8(30),
        M::ManaIsAnyColor => h.u8(31),
        M::SearchTakeover => h.u8(34),
        M::AddTypeIfCountersAtLeast { at_least, .. } => {
            h.u8(32);
            h.u8(*at_least);
        }
        M::AddKeywordIfCountersAtLeast { at_least, .. } => {
            h.u8(33);
            h.u8(*at_least);
        }
        M::GrantActivated { mana_ability, .. } => {
            h.u8(27);
            h.u8(u8::from(*mana_ability));
        }
        M::ModifyPTPerCount { filter, p, t } => {
            h.u8(28);
            filter_hash(h, filter);
            h.i16(*p);
            h.i16(*t);
        }
        M::ModifyPT(p, t) => {
            h.u8(10);
            h.i16(*p);
            h.i16(*t);
        }
        M::SetPT(p, t) => {
            h.u8(11);
            h.i16(*p);
            h.i16(*t);
        }
        M::SwitchPT => h.u8(12),
    }
}

fn counter_tag(kind: CounterKind) -> u8 {
    match kind {
        CounterKind::P1P1 => 1,
        CounterKind::M1M1 => 2,
        CounterKind::Loyalty => 3,
        CounterKind::Lore => 4,
        CounterKind::Time => 5,
        CounterKind::Charge => 6,
        CounterKind::Poison => 7,
        CounterKind::Energy => 8,
        CounterKind::Rad => 9,
        CounterKind::Lifelink => 10,
        CounterKind::Level => 11,
        CounterKind::Custom(id) => 100u8.saturating_add((id % 100) as u8),
    }
}

fn hash_mana_cost(h: &mut Hasher, cost: &baylee_core::mana::ManaCost) {
    h.u8(cost.len());
    for s in cost.symbols() {
        match s {
            ManaSymbol::Generic(n) => {
                h.u8(0);
                h.u32(n);
            }
            ManaSymbol::Colorless => h.u8(1),
            ManaSymbol::White => h.u8(2),
            ManaSymbol::Blue => h.u8(3),
            ManaSymbol::Black => h.u8(4),
            ManaSymbol::Red => h.u8(5),
            ManaSymbol::Green => h.u8(6),
            ManaSymbol::Hybrid(p) => {
                h.u8(7);
                h.u8(p.first() as u8);
                h.u8(p.second() as u8);
            }
            ManaSymbol::TwoOrColor(c) => {
                h.u8(8);
                h.u8(c as u8);
            }
            ManaSymbol::Phyrexian(c) => {
                h.u8(9);
                h.u8(c as u8);
            }
            ManaSymbol::HybridPhyrexian(p) => {
                h.u8(10);
                h.u8(p.first() as u8);
                h.u8(p.second() as u8);
            }
            ManaSymbol::Snow => h.u8(11),
            ManaSymbol::Variable(v) => {
                h.u8(12);
                h.u8(v as u8);
            }
            ManaSymbol::HalfGeneric => h.u8(13),
            ManaSymbol::Infinite => h.u8(14),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, SeatController, SeatSpec,
    };

    struct RegistryLookup;

    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn card_index(oracle_id: &str) -> CardIndex {
        baylee_cards::by_oracle_id(oracle_id)
            .expect("acceptance registry contains the card")
            .index
    }

    fn forest() -> CardIndex {
        card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
    }

    fn force_of_will() -> CardIndex {
        card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
    }

    fn make_preset(seed: u64) -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60)
            .map(|i| DeckEntry {
                card: if i % 3 == 0 {
                    force_of_will()
                } else {
                    forest()
                },
                print: PrintRef::new(0),
            })
            .collect();
        GamePreset {
            format: FormatId::Freeform,
            seed,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![baylee_core::preset::PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: (0..2)
                .map(|_| SeatSpec {
                    controller: SeatController::Ai(AIProfile::default()),
                    deck: deck.clone(),
                    sideboard: vec![],
                    starting_life: None,
                    starting_hand: None,
                    starting_battlefield: vec![],
                    emblems: vec![],
                    team: None,
                })
                .collect(),
        }
    }

    #[test]
    fn setup_is_deterministic() {
        let a = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
        let b = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
        let lib_a = a.zones.list(ZoneLocation::Library(PlayerId::new(0)));
        let lib_b = b.zones.list(ZoneLocation::Library(PlayerId::new(0)));
        assert_eq!(lib_a, lib_b);
        // 60-card deck minus 7 opening cards.
        assert_eq!(lib_a.len(), 53);
        assert_eq!(a.zones.list(ZoneLocation::Hand(PlayerId::new(0))).len(), 7);
    }

    /// Regression: every hosted game marks its human seat `Open` (the gateway
    /// and the dev server both do), and setup used to skip those seats
    /// entirely — the human started with no library, no opening hand, and lost
    /// to an empty draw on turn one.
    #[test]
    fn an_open_seat_is_dealt_in_like_any_other() {
        let mut preset = make_preset(7);
        preset.seats[0].controller = baylee_core::preset::SeatController::Open;
        let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");

        let human = PlayerId::new(0);
        assert_eq!(
            state.zones.list(ZoneLocation::Hand(human)).len(),
            7,
            "an unclaimed human chair still gets an opening hand"
        );
        assert_eq!(state.zones.list(ZoneLocation::Library(human)).len(), 53);

        // And the seat opposite is unaffected.
        let other = PlayerId::new(1);
        assert_eq!(state.zones.list(ZoneLocation::Hand(other)).len(), 7);
    }

    /// The case an empty deck on an `Open` seat is actually for: a chair in a
    /// lobby that nobody has sat down in yet.
    #[test]
    fn a_genuinely_empty_chair_is_still_skipped() {
        let mut preset = make_preset(7);
        preset.seats[0].controller = baylee_core::preset::SeatController::Open;
        preset.seats[0].deck.clear();
        let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");

        let empty = PlayerId::new(0);
        assert!(state.zones.list(ZoneLocation::Library(empty)).is_empty());
        assert!(state.zones.list(ZoneLocation::Hand(empty)).is_empty());
    }

    #[test]
    fn different_seeds_differ() {
        let a = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
        let b = GameState::from_preset(&make_preset(43), &RegistryLookup).unwrap();
        assert_ne!(a.snapshot_hash(), b.snapshot_hash());
    }

    #[test]
    fn draw_moves_top_card_and_bumps_version() {
        let mut state = GameState::from_preset(&make_preset(7), &RegistryLookup).unwrap();
        let hand = ZoneLocation::Hand(PlayerId::new(0));
        let before = state.zones.list(hand).len();
        let top = *state
            .zones
            .list(ZoneLocation::Library(PlayerId::new(0)))
            .last()
            .unwrap();
        let version_before = state.object(top).unwrap().version;
        let drawn = state.draw_cards(PlayerId::new(0), 1);
        assert_eq!(drawn, vec![top]);
        assert_eq!(state.zones.list(hand).len(), before + 1);
        assert_eq!(state.object(top).unwrap().version, version_before + 1);
        assert_eq!(state.object(top).unwrap().zone, crate::zone::Zone::Hand);
        // Twin state must draw the identical card.
        let mut twin = GameState::from_preset(&make_preset(7), &RegistryLookup).unwrap();
        assert_eq!(twin.draw_cards(PlayerId::new(0), 1), drawn);
        assert_eq!(state.snapshot_hash(), twin.snapshot_hash());
    }

    #[test]
    fn journal_records_setup() {
        let state = GameState::from_preset(&make_preset(1), &RegistryLookup).unwrap();
        assert!(matches!(
            state.journal.entries().first().map(|e| &e.event),
            Some(GameEvent::GameStarted { seed: 1, seats: 2 })
        ));
        assert!(
            state
                .journal
                .entries()
                .iter()
                .any(|e| matches!(e.event, GameEvent::Shuffled { .. }))
        );
        assert!(state.journal.entries().iter().any(|e| matches!(
            e.event,
            GameEvent::ZoneChanged {
                to: crate::zone::Zone::Hand,
                ..
            }
        )));
    }

    #[test]
    fn emblems_and_starting_battlefield_are_seeded() {
        let mut preset = make_preset(5);
        preset.seats[0].emblems = vec!["boss:test-emblem".to_string()];
        preset.seats[0].starting_battlefield = vec![DeckEntry {
            card: forest(),
            print: PrintRef::new(0),
        }];
        let state = GameState::from_preset(&preset, &RegistryLookup).unwrap();
        assert_eq!(
            state
                .zones
                .list(ZoneLocation::Command(PlayerId::new(0)))
                .len(),
            1
        );
        assert_eq!(state.zones.list(ZoneLocation::Battlefield).len(), 1);
        assert_eq!(
            state
                .object(state.zones.list(ZoneLocation::Battlefield)[0])
                .unwrap()
                .kind,
            ObjectKind::Permanent
        );
    }
}
