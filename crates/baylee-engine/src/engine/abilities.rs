use super::{
    AbilityDef, AbilityLoc, ActivationTiming, CardLookup, Cause, Cost, CostPart, Engine,
    EngineError, GameEvent, GameObject, LegalActions, NameRef, ObjectId, ObjectKind, Pending,
    Phase, PlanKind, PlayerId, Resolution, SmallVec, Status, TypeSet, Zone, ZoneLocation,
    ZonePosition, casting, eval, mana_pay, resolve,
};
use crate::choice::TargetPrompt;
use baylee_cards_dsl::ActivationZone;

impl<L: CardLookup> Engine<L> {
    /// Whether a spell has something it could legally be cast at.
    ///
    /// CR 601.2c: a spell with a mandatory target and no legal target cannot
    /// be cast at all. Offering it anyway is not a harmless over-approximation
    /// — the cast wizard aborts at the targeting step, so a human sees a
    /// button that only ever errors, and an agent picks the same illegal cast
    /// on every pass because nothing about the state has changed.
    ///
    /// Deliberately conservative, and returns `true` whenever it cannot be
    /// sure: modal spells choose targets per mode, X-counted targets depend on
    /// a value nobody has picked yet, and player targets are not objects. All
    /// three stay the wizard's problem.
    fn has_a_legal_target(&self, player: PlayerId, card: ObjectId) -> bool {
        let Some(def) = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
        else {
            return true;
        };
        let abilities = def.abilities_for_face(0);
        if abilities
            .iter()
            .any(|a| matches!(a, AbilityDef::ModalSpell { .. }))
        {
            return true;
        }
        let Some(req) = abilities.iter().find_map(|a| match a {
            AbilityDef::Spell { targets, .. } => *targets,
            _ => None,
        }) else {
            return true;
        };
        if req.min == 0
            || req.count_is_x
            || matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer
                    | baylee_cards_dsl::TargetSpec::AnyOpponent
                    | baylee_cards_dsl::TargetSpec::Player(_)
                    | baylee_cards_dsl::TargetSpec::ThisObject
            )
        {
            return true;
        }
        eval::target_options(&req.spec, &self.state, player, card).len() >= req.min as usize
    }
}

impl<L: CardLookup> Engine<L> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn compute_legal(&self, player: PlayerId) -> LegalActions {
        let mut legal = LegalActions {
            can_pass: true,
            ..LegalActions::default()
        };
        let main_phase = matches!(self.state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        let sorcery_timing =
            main_phase && self.state.turn.active == player && self.state.zones.stack_is_empty();
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            let any_face_is_land = if obj.characteristics().types.contains(TypeSet::LAND) {
                true
            } else {
                // MDFC: a back land face is playable (CR 712.4a).
                obj.card
                    .and_then(|c| self.lookup.card(c.index))
                    .is_some_and(|def| def.faces.iter().any(|f| f.types.contains(TypeSet::LAND)))
            };
            if any_face_is_land
                && sorcery_timing
                && self.state.players[player.get() as usize].lands_played_this_turn == 0
            {
                legal.lands.push(card);
            }
            if casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
            {
                legal.castable.push(card);
            }
        }
        // Adventure + Opposition Agent: cards castable from exile.
        for &card in self.state.zones.list(ZoneLocation::Exile(player)) {
            let exiled_castable = self.state.object(card).is_some_and(|o| {
                o.riders.contains(&crate::object::Rider::Adventure)
                    || o.riders
                        .iter()
                        .any(|r| matches!(r, crate::object::Rider::PlayableFromExileFor(p) if *p == player))
            });
            if exiled_castable
                && casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
            {
                legal.castable.push(card);
            }
        }
        // Commander (CR 903.8): your own commanders in the command zone.
        // The emblems sharing that zone are filtered out by `can_cast`,
        // which asks the marker list rather than the zone.
        for &card in self.state.zones.list(ZoneLocation::Command(player)) {
            if casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
            {
                legal.castable.push(card);
            }
        }
        // Flashback: granted cards in your graveyard are castable.
        for &card in self.state.zones.list(ZoneLocation::Graveyard(player)) {
            let granted = self.state.effects.iter().any(|fx| {
                matches!(fx.modifier, baylee_cards_dsl::Modifier::GrantsFlashback)
                    && matches!(&fx.filter, crate::effects::EffectFilter::ObjectIs(id) if *id == card)
            });
            // Disturb: a face with disturb is castable from the graveyard.
            let disturb = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .is_some_and(|def| def.faces.iter().any(|f| f.disturb));
            if (granted || disturb)
                && casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
            {
                legal.castable.push(card);
            }
        }
        for &id in self.state.zones.list(ZoneLocation::Battlefield) {
            // Karn's lock, asked on the offering side too. It stops every
            // activated ability of the permanent, a mana ability included —
            // CR 605.1 makes a mana ability a kind of activated ability, not
            // an exception to one. What it does *not* stop is a cast, which
            // is why this is two guards and not a `continue`: the Prepared
            // rider below offers a spell.
            let locked = self.artifact_activations_are_locked(id);
            if !locked && casting::can_activate_mana(&self.state, player, id) {
                legal.mana_abilities.push(id);
            }
            // Activated abilities of controlled permanents.
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            if obj.controller != player {
                continue;
            }
            // A token's abilities come from its definition rather than from a
            // card; everything below reads the same `AbilityDef`s either way.
            let offered: &[AbilityDef] = if locked {
                &[]
            } else {
                obj.abilities(&self.lookup)
            };
            for (i, ability) in offered.iter().enumerate() {
                match ability {
                    AbilityDef::Activated {
                        cost,
                        timing,
                        zone,
                        target,
                        ..
                    } => {
                        if *zone != ActivationZone::Battlefield {
                            continue; // hand-zone abilities are scanned below
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if !self.ability_has_a_target(
                            player,
                            id,
                            target.map(baylee_cards_dsl::TargetReq::one),
                        ) {
                            continue;
                        }
                        if self.can_afford(player, id, cost) {
                            legal.abilities.push((id, i as u32));
                        }
                    }
                    AbilityDef::ActivatedConditional {
                        cost,
                        timing,
                        zone,
                        condition,
                        target,
                        ..
                    } => {
                        if *zone != ActivationZone::Battlefield {
                            continue;
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if !self.check_activation_condition(player, id, *condition) {
                            continue;
                        }
                        if !self.ability_has_a_target(
                            player,
                            id,
                            target.map(baylee_cards_dsl::TargetReq::one),
                        ) {
                            continue;
                        }
                        if self.can_afford(player, id, cost) {
                            legal.abilities.push((id, i as u32));
                        }
                    }
                    AbilityDef::Loyalty { cost, targets, .. } => {
                        // Loyalty abilities: sorcery timing, once per turn
                        // per walker, enough loyalty for negative costs.
                        if !sorcery_timing || self.loyalty_used_this_turn.contains(&id) {
                            continue;
                        }
                        let loyalty = obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty);
                        if *cost < 0 && loyalty < (-*cost) as u16 {
                            continue;
                        }
                        if !self.ability_has_a_target(player, id, *targets) {
                            continue;
                        }
                        legal.abilities.push((id, i as u32));
                    }
                    _ => {}
                }
            }
            // Prepared (Emeritus of Woe): a prepared permanent may cast
            // a copy of its linked spell — synthetic index `choice::PREPARED_CAST`.
            if obj.riders.contains(&crate::object::Rider::Prepared) {
                let linked = obj.card.and_then(|c| {
                    let face = obj.face_index as usize;
                    self.lookup.card(c.index).and_then(|def| {
                        def.abilities_for_face(face).iter().find_map(|a| match a {
                            AbilityDef::Prepared { card } => Some(*card),
                            _ => None,
                        })
                    })
                });
                if let Some(linked_card) = linked {
                    let affordable = self.lookup.card(linked_card).is_some_and(|spell_def| {
                        let cost = &spell_def.faces[0].mana_cost;
                        casting::affordable(
                            &self.state,
                            &self.state.players[player.get() as usize].mana_pool,
                            cost,
                        )
                    });
                    if affordable {
                        legal.abilities.push((id, crate::choice::PREPARED_CAST));
                    }
                }
            }
            // Karn's lock reaches the grants below as well: an ability a
            // continuous effect gave an artifact is an activated ability of
            // that artifact like any other. This is the third door onto the
            // same list — printed, intrinsic, granted — and the one that made
            // moving the check to the top of `start_activation` necessary,
            // because `start_granted_activation` returns before the branch it
            // used to sit in. (Prepared, in between, is the door the lock
            // leaves open: that one offers a spell.)
            if locked {
                continue;
            }
            // Granted abilities (Urza's Saga chapters, a Chromatic Lantern's
            // lands): each surfaces under its own synthetic index. The slot
            // is the *position among the grants that apply*, affordable or
            // not — an index that shifted when a cost became payable would
            // name a different ability from one priority window to the next.
            for (n, granted) in crate::effects::granted_activated(&self.state, id)
                .take(crate::choice::GRANTED_SLOTS as usize)
                .enumerate()
            {
                if !self.can_afford(player, id, &granted.cost) {
                    continue;
                }
                legal
                    .abilities
                    .push((id, crate::choice::granted_ability(n as u32)));
                if granted.mana_ability {
                    legal.mana_abilities.push(id);
                }
            }
        }
        // Hand-zone activations (cycling) and suspensions.
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            let Some(card_ref) = obj.card else { continue };
            let Some(def) = self.lookup.card(card_ref.index) else {
                continue;
            };
            for (i, ability) in def
                .abilities_for_face(obj.face_index as usize)
                .iter()
                .enumerate()
            {
                match ability {
                    AbilityDef::Activated {
                        cost, timing, zone, ..
                    } => {
                        if *zone != ActivationZone::Hand {
                            continue;
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.can_afford(player, card, cost) {
                            legal.abilities.push((card, i as u32));
                        }
                    }
                    AbilityDef::Suspend { .. } if sorcery_timing => {
                        legal.suspendable.push(card);
                    }
                    _ => {}
                }
            }
        }
        legal
    }

    /// Whether a targeting ability has anything legal to point at
    /// (CR 601.2c, applied to activations by CR 602.2b).
    ///
    /// `apply` already refuses such an activation with "no legal targets";
    /// this is the same probe on the offering side, because the two have to
    /// be one probe or the client lights a permanent up and the click is
    /// refused. Riptide Laboratory is the case that found it — "{1}{U}, {T}:
    /// Return target Wizard you control" was offered at a table whose only
    /// Wizard was the opponent's.
    /// Whether an ability can point at what it has to point at.
    ///
    /// The *count* is half the question and used not to be asked at all: a
    /// bare spec reads as "exactly one", so "up to one target" (`min: 0`) was
    /// an ability nobody could activate with an empty board — which on Karn,
    /// the Great Creator hides a loyalty tick and on Teferi, Time Raveler
    /// hides a drawn card.
    fn ability_has_a_target(
        &self,
        player: PlayerId,
        source: ObjectId,
        targets: Option<baylee_cards_dsl::TargetReq>,
    ) -> bool {
        let Some(req) = targets else {
            return true;
        };
        if req.min == 0 {
            return true;
        }
        let wanted = req.min as usize;
        if matches!(
            req.spec,
            baylee_cards_dsl::TargetSpec::AnyPlayer | baylee_cards_dsl::TargetSpec::AnyOpponent
        ) {
            return eval::target_player_options(&self.state, &req.spec, player).len() >= wanted;
        }
        eval::target_options(&req.spec, &self.state, player, source).len() >= wanted
    }

    /// Whether Karn's lock covers this permanent: "activated abilities of
    /// artifacts your *opponents* control can't be activated".
    ///
    /// Not "everyone but me" — a teammate is neither, and at a two-headed
    /// table the two readings differ by every artifact on Karn's own side of
    /// the table but Karn's controller's.
    ///
    /// One probe read from both sides, for the reason
    /// [`Engine::ability_has_a_target`] gives. The lock lived only in `apply`,
    /// which is invisible while the walker is yours — the abilities it stops
    /// are on the *other* seat's board, and that seat was still being offered
    /// every one of them.
    pub(crate) fn artifact_activations_are_locked(&self, id: ObjectId) -> bool {
        let Some(obj) = self.state.object(id) else {
            return false;
        };
        obj.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::ARTIFACT)
            && self.state.effects.iter().any(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::CantActivateArtifacts
                ) && self.state.is_opponent(obj.controller, fx.controller)
            })
    }

    /// Precondition check for `ActivatedConditional` abilities (B1).
    pub(crate) fn check_activation_condition(
        &self,
        player: PlayerId,
        source: ObjectId,
        condition: baylee_cards_dsl::ActivationCondition,
    ) -> bool {
        match condition {
            baylee_cards_dsl::ActivationCondition::ControlCount(filter, min) => {
                let count = self
                    .state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .filter(|id| {
                        self.state.object(**id).is_some_and(|o| {
                            o.controller == player
                                && crate::eval::matches(filter, &self.state, o, player, **id)
                        })
                    })
                    .count();
                count >= min as usize
            }
            baylee_cards_dsl::ActivationCondition::OpponentGraveyardCountAtLeast(min) => (0..self
                .state
                .players
                .len())
                .map(|i| PlayerId::new(i as u8))
                .filter(|id| self.state.is_opponent(*id, player))
                .any(|id| self.state.zones.list(ZoneLocation::Graveyard(id)).len() >= min as usize),
            baylee_cards_dsl::ActivationCondition::CountersOnSelf(kind, min) => self
                .state
                .object(source)
                .is_some_and(|o| o.counters.get(kind) >= u16::from(min)),
            baylee_cards_dsl::ActivationCondition::CountersOnSelfExactly(kind, n) => self
                .state
                .object(source)
                .is_some_and(|o| o.counters.get(kind) == u16::from(n)),
        }
    }

    pub(crate) fn can_afford(&self, player: PlayerId, source: ObjectId, cost: &Cost) -> bool {
        let pool = &self.state.players[player.get() as usize].mana_pool;
        // Mycosynth Lattice: mana spends as though it were any colour, so a
        // five-colour activation cost is payable off five Islands.
        let payable = if casting::mana_is_wild(&self.state) {
            mana_pay::can_pay_wild(pool, &cost.mana)
        } else {
            mana_pay::can_pay(pool, &cost.mana)
        };
        if !payable {
            return false;
        }
        for part in cost.parts {
            match part {
                // CR 302.6, second sentence: a creature's activated ability
                // with the tap or the untap symbol in its cost cannot be
                // activated unless the creature has been under its
                // controller's control since their most recent turn began.
                // The check belongs here and not in the cost payment: a
                // creature tapped to pay someone *else's* cost (convoke,
                // crew) is not activating an ability of its own, and
                // summoning sickness has never stopped that.
                CostPart::TapSelf | CostPart::UntapSelf => {
                    let Some(obj) = self.state.object(source) else {
                        return false;
                    };
                    if crate::combat::summoning_sick(&self.state, obj) {
                        return false;
                    }
                    // {T} needs it untapped. {Q} ought to need it tapped and
                    // does not — `docs/observed-faults.md` entry 53.
                    if matches!(part, CostPart::TapSelf) && obj.status.contains(Status::TAPPED) {
                        return false;
                    }
                }
                CostPart::PayLife(n) => {
                    if self.state.players[player.get() as usize].life <= i32::from(*n) {
                        return false;
                    }
                }
                CostPart::SacrificeSelf
                | CostPart::Sacrifice(_)
                | CostPart::Discard(_)
                | CostPart::DiscardSelf
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::ExileFromHand(_)
                | CostPart::PayLifeX => {}
            }
        }
        true
    }

    // ---------------------------------------------------- S3: abilities

    /// Prepared cast (Emeritus of Woe): pays the linked spell's cost,
    /// puts a copy of it on the stack, and removes the prepared marker.
    fn start_prepared_cast(
        &mut self,
        player: PlayerId,
        source: ObjectId,
    ) -> Result<(), EngineError> {
        let linked_card = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .and_then(|c| {
                let face = self
                    .state
                    .object(source)
                    .map_or(0, |o| o.face_index as usize);
                self.lookup.card(c.index).and_then(|def| {
                    def.abilities_for_face(face).iter().find_map(|a| match a {
                        AbilityDef::Prepared { card } => Some(*card),
                        _ => None,
                    })
                })
            })
            .ok_or(EngineError::IllegalAction("no prepared spell"))?;
        let spell_def = self
            .lookup
            .card(linked_card)
            .ok_or(EngineError::IllegalAction("unknown linked card"))?;
        let face = &spell_def.faces[0];
        let wild = casting::mana_is_wild(&self.state);
        if !casting::pay_with(
            wild,
            &mut self.state.players[player.get() as usize].mana_pool,
            &face.mana_cost,
        ) {
            return Err(EngineError::IllegalAction("cannot pay the spell's cost"));
        }
        // Unprepare the source.
        if let Some(obj) = self.state.object_mut(source) {
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::Prepared));
        }
        // The copy of the linked spell, built the way `Effect::CopyTargetSpell`
        // builds one (CR 707.10): a fresh object carrying the copied card.
        let name = self.state.names.intern(face.name);
        let base = crate::object::Characteristics::from_face(spell_def, 0, name);
        let ts = self.state.next_timestamp();
        let id = self.state.arena.insert_with(|oid| {
            let mut obj = GameObject::new_bare(oid, player, ObjectKind::Spell, base);
            obj.timestamp = ts;
            obj.cast_from_hand = false;
            // A fresh object starts in its owner's library, and putting it
            // somewhere with `Zones::insert` does not say otherwise —
            // `move_object` reads the object, not the zone lists. Left at
            // the default, the spell resolved *out of the library*: the
            // stack kept the id, the spell ceased to exist beneath it, and
            // `stack_projectable` was still pointing at nothing.
            obj.zone = Zone::Stack;
            // And it has to *be* the linked card. `resolve_stack_top` reads a
            // spell's effects off `obj.card` rather than through
            // `GameObject::abilities`, so a card-less spell resolves to
            // nothing at all — which is what this one did: Demonic Tutor went
            // on the stack, both seats passed, and no library was ever
            // searched.
            obj.card = Some(crate::object::CardRef {
                index: linked_card,
                // The rules identity is the linked card; the *printing* is
                // one nobody brought to the table, and `PrintRef::new(0)`
                // here would be another card's art under this one's name —
                // and would hand the opponent a row of the print table out
                // of a deck they have never seen.
                print: baylee_core::ids::PrintRef::UNKNOWN,
            });
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, true);
        // Per-turn tracking, exactly as an ordinary cast keeps it. The card
        // says "you may **cast** a copy of its spell", so this is a cast and
        // the turn has to count it: without these two the prepared spell was
        // invisible to Storm of Saruman's "your second spell each turn" and
        // handed Esper Sentinel's "first noncreature spell" to whatever was
        // cast next.
        if !face.types.contains(baylee_core::types::TypeSet::CREATURE)
            && let Some(v) = self
                .state
                .per_turn
                .noncreature_spells
                .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        if let Some(v) = self
            .state
            .per_turn
            .spells_cast
            .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        self.state
            .journal
            .record(GameEvent::SpellCast { object: id, player });
        self.after_action(player);
        Ok(())
    }

    /// Activates a granted ability (Urza's Saga's Construct chapter):
    /// pays the cost, then pushes the granted effects via the synthetic
    /// side map (or resolves immediately for mana abilities).
    fn start_granted_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        slot: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        // `slot` counts the grants that *apply*, in the order the offer
        // counted them. Reading it back through the same iterator is what
        // makes the two agree; a mismatch would run the ability next to the
        // one the player pressed.
        let granted = crate::effects::granted_activated(&self.state, source)
            .nth(slot as usize)
            .ok_or(EngineError::IllegalAction("no granted ability"))?;
        let (cost, effects, mana_ability) = (granted.cost, granted.effects, granted.mana_ability);
        self.pay_cost(player, source, &cost)?;
        if mana_ability {
            let mut res = crate::resolve::Resolution {
                source,
                on_stack: source,
                controller: player,
                effects: crate::resolve::flatten(effects),
                pc: 0,
                targets,
                x: None,
                chosen_player: None,
                target_players: baylee_core::ids::SeatSet::new(),
                event_object: None,
                targeted: false,
                awaiting: None,
                mana_ability: true,
            };
            match crate::resolve::run(&mut self.state, &mut res) {
                crate::resolve::Flow::Complete => {}
                crate::resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        } else {
            let name = self
                .state
                .object(source)
                .map_or(NameRef::new(0), |o| o.base.name);
            let base = self.state.bare_base(name);
            let card = self
                .state
                .object(source)
                .and_then(|o| o.card)
                .map(|c| c.index)
                .ok_or(EngineError::IllegalAction("source not card-backed"))?;
            let id = self.state.arena.insert_with(|id| {
                GameObject::new_ability_on_stack(
                    id,
                    player,
                    crate::object::AbilityLoc {
                        card,
                        index: baylee_core::ids::AbilityRef::SYNTHETIC,
                        source,
                    },
                    targets,
                    base,
                )
            });
            self.synthetic_fx.insert(id, effects);
            self.state
                .zones
                .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
            self.state.journal.record(GameEvent::AbilityTriggered {
                object: id,
                source,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                controller: player,
            });
        }
        self.after_action(player);
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // activation is a staged checklist; extraction would obscure it
    pub(crate) fn start_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        // Prepared cast (`choice::PREPARED_CAST`): pay the linked
        // spell's cost, put a copy on the stack, unprepare the source.
        if ability_index == crate::choice::PREPARED_CAST {
            return self.start_prepared_cast(player, source);
        }
        // Karn's lock, ahead of every route below it. It used to sit inside
        // the printed-ability branch, which the granted and loyalty branches
        // return before reaching — so a locked artifact's *granted* ability
        // passed under it. Structural rather than observed: the offer guard
        // above refuses first, and no test gets this far.
        if self.artifact_activations_are_locked(source) {
            return Err(EngineError::IllegalAction(
                "activated abilities of artifacts can't be activated (Karn)",
            ));
        }
        // Granted abilities (synthetic index): resolve via the side map.
        if let Some(slot) = crate::choice::granted_slot(ability_index) {
            return self.start_granted_activation(player, source, slot, targets);
        }
        // Loyalty abilities route to their own activation path first.
        if let Some(AbilityDef::Loyalty { cost, .. }) = self
            .state
            .object(source)
            .map(|o| o.abilities(&self.lookup))
            .and_then(|abilities| abilities.get(ability_index as usize))
        {
            return self.start_loyalty_activation(player, source, ability_index, targets, *cost);
        }
        let (card_index, cost, effects, target, mana_ability, zone) = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            let card = obj
                .card
                .ok_or(EngineError::IllegalAction("not a card-backed object"))?;
            // The object's own list, which is what `legal_actions` indexed
            // when it offered this. Reading the card's instead was the
            // "two probes must agree" bug once more: a Glasspool Mimic
            // copying Werefox Bodyguard was offered the sacrifice ability
            // at index 1 and refused it with "no such ability", because
            // Glasspool Mimic's printed list is one entry long.
            match obj
                .abilities(&self.lookup)
                .get(ability_index as usize)
                .ok_or(EngineError::IllegalAction("no such ability"))?
            {
                AbilityDef::Activated {
                    cost,
                    effects,
                    target,
                    mana_ability,
                    zone,
                    ..
                } => (card.index, *cost, *effects, *target, *mana_ability, *zone),
                AbilityDef::ActivatedConditional {
                    cost,
                    effects,
                    target,
                    mana_ability,
                    zone,
                    condition,
                    ..
                } => {
                    if !self.check_activation_condition(player, source, *condition) {
                        return Err(EngineError::IllegalAction("activation condition not met"));
                    }
                    (card.index, *cost, *effects, *target, *mana_ability, *zone)
                }
                _ => return Err(EngineError::IllegalAction("not an activated ability")),
            }
        };
        // Read before any cost is paid, because a cost may move the source
        // and a moved copy is no longer one — see `Engine::activating_abilities`.
        self.activating_abilities = self
            .state
            .object(source)
            .map(|o| (source, o.abilities(&self.lookup)));
        // Zone validation (battlefield abilities vs. hand abilities).
        let in_right_zone = match zone {
            ActivationZone::Battlefield => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield),
            ActivationZone::Hand => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player)),
        };
        if !in_right_zone {
            return Err(EngineError::IllegalAction(
                "ability not usable from this zone",
            ));
        }
        let _ = card_index;
        let _ = zone;
        // Targets (chosen first unless already provided).
        if targets.is_empty()
            && let Some(spec) = target
        {
            let options = eval::target_options(&spec, &self.state, player, source);
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            self.pending_plan = Some(PlanKind::ActivateAbility {
                source,
                ability_index,
            });
            self.pending = Pending::ChooseTargets {
                player,
                options,
                player_options: Vec::new(),
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        if !self.can_afford(player, source, &cost) {
            return Err(EngineError::IllegalAction("cannot pay the cost"));
        }
        self.pay_cost(player, source, &cost)?;
        // Targets are chosen after the cost is paid (CR 602.1b); the
        // target-choice plan completes via finish_activation.
        if targets.is_empty()
            && let Some(spec) = target
        {
            let options = eval::target_options(&spec, &self.state, player, source);
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            self.pending_plan = Some(PlanKind::ActivateAbility {
                source,
                ability_index,
            });
            self.pending = Pending::ChooseTargets {
                player,
                options,
                player_options: Vec::new(),
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        if mana_ability {
            // Mana abilities resolve immediately, without the stack
            // (CR 605.3b). Choice-mana abilities (any-color lands, Command
            // Tower) suspend on the color choice like any resolution.
            let mut res = Resolution {
                source,
                on_stack: source,
                controller: player,
                effects: resolve::flatten(effects),
                pc: 0,
                targets,
                x: None,
                chosen_player: None,
                target_players: baylee_core::ids::SeatSet::new(),
                event_object: None,
                targeted: false,
                awaiting: None,
                mana_ability: true,
            };
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => {}
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        } else {
            self.push_ability_to_stack(player, source, ability_index, targets)?;
        }
        self.after_action(player);
        Ok(())
    }

    /// Completes a loyalty activation after targeting: pushes the ability
    /// to the stack without re-paying (cost was paid at activation).
    pub(crate) fn finish_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        let card_index = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .map(|c| c.index)
            .ok_or(EngineError::IllegalAction("not a card-backed object"))?;
        let loc = AbilityLoc {
            card: card_index,
            index: ability_index,
            source,
        };
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 608.2, as in `push_ability_to_stack`. A loyalty cost never moves
        // the walker, so the source is still there to be read.
        let abilities = self
            .state
            .object(source)
            .map_or(&[][..], |o| o.abilities(&self.lookup));
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(id, player, loc, targets, base);
            obj.own_abilities = Some(abilities);
            obj.chosen_player = self.loyalty_player_choice.take();
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller: player,
        });
        self.after_action(player);
        Ok(())
    }

    /// Activates a planeswalker loyalty ability: applies the loyalty cost
    /// and puts the ability on the stack (CR 606.2-606.4).
    #[allow(clippy::too_many_lines)] // loyalty activation is a staged checklist
    pub(crate) fn start_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        cost: i8,
    ) -> Result<(), EngineError> {
        if self.loyalty_used_this_turn.contains(&source) {
            return Err(EngineError::IllegalAction("loyalty already used this turn"));
        }
        let (card_index, effects, wanted) = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            let card = obj
                .card
                .ok_or(EngineError::IllegalAction("not a card-backed object"))?;
            let AbilityDef::Loyalty {
                effects, targets, ..
            } = obj
                .abilities(&self.lookup)
                .get(ability_index as usize)
                .ok_or(EngineError::IllegalAction("no such ability"))?
            else {
                return Err(EngineError::IllegalAction("not a loyalty ability"));
            };
            (card.index, *effects, *targets)
        };
        // Loyalty cost is paid at activation (CR 606.3) — after checking
        // that required targets exist, before targeting.
        let old = self.state.object(source).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        if cost < 0 && old < (-cost) as u16 {
            return Err(EngineError::IllegalAction("not enough loyalty"));
        }
        if let Some(req) = wanted
            && req.min > 0
            && !matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer | baylee_cards_dsl::TargetSpec::AnyOpponent
            )
        {
            let options = eval::target_options(&req.spec, &self.state, player, source);
            if options.len() < req.min as usize {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
        }
        let new = if cost >= 0 {
            old.saturating_add(cost as u16)
        } else {
            old - (-cost) as u16
        };
        {
            let obj = self.state.object_mut(source).expect("walker exists");
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, new);
        }
        self.state.journal.record(GameEvent::CounterChanged {
            object: source,
            kind: baylee_cards_dsl::CounterKind::Loyalty,
            old,
            new,
        });
        self.loyalty_used_this_turn.push(source);
        // Targets first if required.
        if targets.is_empty()
            && let Some(req) = wanted
        {
            if matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer | baylee_cards_dsl::TargetSpec::AnyOpponent
            ) {
                let options = eval::target_player_options(&self.state, &req.spec, player);
                if options.len() < req.min as usize {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                self.pending_plan = Some(PlanKind::LoyaltyPlayer {
                    source,
                    ability_index,
                });
                self.pending = Pending::ChoosePlayer { player, options };
                self.awaiting_answer = true;
                return Ok(());
            }
            let options = eval::target_options(&req.spec, &self.state, player, source);
            if options.len() < req.min as usize {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            // "Up to one target" with nothing on the board to point at is not
            // a question: there would be one answer to it. The ability goes
            // on the stack targeting nothing, which is what the printing says
            // it may do.
            if !options.is_empty() {
                self.pending_plan = Some(PlanKind::ActivateAbility {
                    source,
                    ability_index,
                });
                self.pending = Pending::ChooseTargets {
                    player,
                    options,
                    player_options: Vec::new(),
                    min: req.min,
                    max: req.max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return Ok(());
            }
        }
        // The ability goes on the stack.
        let loc = AbilityLoc {
            card: card_index,
            index: ability_index,
            source,
        };
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 608.2, as in `push_ability_to_stack`. A loyalty cost never moves
        // the walker, so the source is still there to be read.
        let abilities = self
            .state
            .object(source)
            .map_or(&[][..], |o| o.abilities(&self.lookup));
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(id, player, loc, targets, base);
            obj.own_abilities = Some(abilities);
            obj.chosen_player = self.loyalty_player_choice.take();
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller: player,
        });
        self.after_action(player);
        let _ = effects;
        Ok(())
    }

    pub(crate) fn pay_cost(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
    ) -> Result<(), EngineError> {
        if !cost.mana.is_empty() {
            // Mycosynth Lattice: any mana pays any pip (read before the pool
            // is borrowed mutably).
            let wild = casting::mana_is_wild(&self.state);
            let pool = &mut self.state.players[player.get() as usize].mana_pool;
            if !casting::pay_with(wild, pool, &cost.mana) {
                return Err(EngineError::IllegalAction("not enough mana"));
            }
        }
        for part in cost.parts {
            match part {
                CostPart::TapSelf => {
                    if let Some(obj) = self.state.object_mut(source) {
                        obj.status.insert(Status::TAPPED);
                    }
                    self.state.journal.record(GameEvent::ObjectTapped {
                        object: source,
                        cause: Cause::Cost,
                    });
                }
                CostPart::UntapSelf => {
                    if let Some(obj) = self.state.object_mut(source) {
                        obj.status.remove(Status::TAPPED);
                    }
                }
                CostPart::DiscardSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::SacrificeSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::ReturnSelfToHand => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Hand(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::PayLife(n) => {
                    let p = &mut self.state.players[player.get() as usize];
                    let old = p.life;
                    p.life -= i32::from(*n);
                    let new = p.life;
                    self.state.journal.record(GameEvent::LifeChanged {
                        player,
                        old,
                        new,
                        cause: Cause::Cost,
                    });
                }
                CostPart::ExileSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Exile(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::ExileFromHand(_) | CostPart::PayLifeX => {
                    // Choice/X-driven parts are paid in the casting wizard.
                }
                CostPart::Sacrifice(_) | CostPart::Discard(_) => {
                    return Err(EngineError::IllegalAction(
                        "choice costs are not supported yet (M2)",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Pushes an emblem's triggered ability onto the stack (command-zone
    /// source, not card-backed).
    pub(crate) fn push_emblem_ability_to_stack(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) {
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        let abilities = self
            .state
            .object(source)
            .map_or(&[][..], |o| o.abilities(&self.lookup));
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(
                id,
                controller,
                AbilityLoc {
                    // Sentinel: an emblem has no card, so the handle a
                    // client is given names none. What resolves is the
                    // list captured below (CR 608.2), as for any ability.
                    card: baylee_core::ids::CardIndex::new(0),
                    index: ability_index,
                    source,
                },
                targets,
                base,
            );
            obj.own_abilities = Some(abilities);
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller,
        });
    }

    pub(crate) fn push_ability_to_stack(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        let card = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .ok_or(EngineError::IllegalAction("source is not card-backed"))?;
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 608.2: the ability on the stack exists independently of the
        // permanent it came from, so it takes the list `index` points into
        // with it. Reading it back off the source at resolution time was
        // right only while a source's abilities could not change under it —
        // a copy that dies with its ability on the stack stops being a copy
        // (CR 400.7), and the index would then be read against the printed
        // card, which is a different ability or none at all.
        //
        // An activation captured its list before paying a cost that may
        // already have moved the source; a trigger has no such window and
        // reads it here.
        let captured = self
            .activating_abilities
            .take()
            .and_then(|(id, abilities)| (id == source).then_some(abilities));
        let abilities = captured.unwrap_or_else(|| {
            self.state
                .object(source)
                .map_or(&[][..], |o| o.abilities(&self.lookup))
        });
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(
                id,
                controller,
                AbilityLoc {
                    card: card.index,
                    index: ability_index,
                    source,
                },
                targets,
                base,
            );
            obj.own_abilities = Some(abilities);
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller,
        });
        Ok(())
    }
}
