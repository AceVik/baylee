//! Resolving the top of the stack, and finishing a resolution.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// What CR 608.2b's re-check found about the object on top of the stack.
///
/// Three answers and not two, because "no legal target left" and "this was
/// never a targeted spell" have to be told apart: the first removes the
/// spell from the stack and the second is most of the stack.
pub(super) enum TargetLegality {
    /// Nothing to ask. The object specifies no targets, or the targets it
    /// specifies are not ones a player chose.
    NotAsked,
    /// At least one chosen target is still legal, and these are the ones
    /// that are. CR 608.2b: the spell resolves, and "the spell or ability
    /// won't do anything to an illegal target".
    Kept {
        /// The chosen objects that are still legal targets.
        objects: SmallVec<[ObjectId; 2]>,
        /// The chosen players that still are.
        players: SeatSet,
        /// The objects chosen for the second instance of the word that
        /// still are — a list of its own, narrowed by its own requirement.
        second: SmallVec<[ObjectId; 1]>,
    },
    /// Every chosen target is now illegal: it does not resolve.
    AllIllegal,
}

impl<L: CardLookup> Engine<L> {
    /// What the object on top of the stack is allowed to target.
    ///
    /// One arm list for a question that is asked twice — CR 608.2b's
    /// re-check below, and `Resolution::targeted`, which is what tells
    /// `Filter::This` apart from itself. It was written out once before and
    /// the second copy would have been the third: `AbilityDef::Activated`
    /// and `AbilityDef::ActivatedConditional` are twins that six readers
    /// across this workspace have already matched one of and not the other.
    ///
    /// A spell carries its requirement on the object instead of in the card:
    /// the cast wizard writes `target_req` there so a copy can be retargeted
    /// without a lookup (CR 707.10c), and this is a second reader with the
    /// same reason — by resolution the mode, the face and the copy status
    /// have all been settled and the object is where they were settled.
    pub(super) fn stack_target_req(&self, on_stack: ObjectId) -> Option<TargetReq> {
        let obj = self.state.object(on_stack)?;
        if obj.target_req.is_some() || obj.kind != ObjectKind::AbilityOnStack {
            return obj.target_req;
        }
        let loc = obj.ability?;
        if loc.index == AbilityRef::SYNTHETIC {
            // A synthetic trigger prints nothing to read a requirement off,
            // so it carries one on its object when it has one: a granted
            // triggered ability's target, or a reflexive one's. Prowess and
            // ward carry none and answer `None`, as before. Answering `None`
            // for all of them meant CR 608.2b never re-checked a synthetic
            // target. Eden's reflexive ability returned a card that had been
            // exiled in response, from exile to hand.
            return obj.target_req;
        }
        let abilities = obj.own_abilities.as_ref().map_or_else(
            || {
                self.state
                    .object(loc.source)
                    .map_or(crate::copiable_abilities::AbilityDefs::EMPTY, |o| {
                        o.printed_abilities(&self.lookup)
                    })
            },
            crate::copiable_abilities::AbilityDefs::from,
        );
        crate::object::ability_target_req(abilities, loc.index, obj.mode_index)
    }

    /// The damage a triggered ability on the stack divides as its controller
    /// chooses ([`baylee_cards_dsl::Effect::DealDamageDivided`]), read off
    /// the list it was put on the stack with, as its second target is.
    pub(in crate::engine) fn stack_divided_amount(&self, on_stack: ObjectId) -> Option<u32> {
        let obj = self.state.object(on_stack)?;
        let loc = obj.ability?;
        if obj.kind != ObjectKind::AbilityOnStack || loc.index == AbilityRef::SYNTHETIC {
            return None;
        }
        let abilities = obj.own_abilities.as_ref().map_or_else(
            || {
                self.state
                    .object(loc.source)
                    .map_or(crate::copiable_abilities::AbilityDefs::EMPTY, |o| {
                        o.printed_abilities(&self.lookup)
                    })
            },
            crate::copiable_abilities::AbilityDefs::from,
        );
        let AbilityDef::Triggered { effects, .. } = abilities.get(loc.index as usize)? else {
            return None;
        };
        effects.iter().find_map(|effect| match effect {
            baylee_cards_dsl::Effect::DealDamageDivided { amount } => Some(*amount),
            _ => None,
        })
    }

    /// What the top of the stack may target with its **second** instance of
    /// the word "target", read from the same places [`Self::stack_target_req`]
    /// reads the first: the spell's own object, or the ability's definition.
    ///
    /// Two arms and not the whole list, because only two shapes can say it —
    /// [`AbilityDef::Spell`] through the object and the activated twins here.
    /// Both twins, for the reason `stack_target_req` gives.
    pub(in crate::engine) fn stack_second_target_req(
        &self,
        on_stack: ObjectId,
    ) -> Option<TargetReq> {
        let obj = self.state.object(on_stack)?;
        // A requirement written on the object wins: the cast wizard writes a
        // spell's, and a trigger's is written bound to the player its first
        // instance named (`Engine::ask_trigger_second_target`), which is the
        // question its re-check has to ask again.
        if obj.kind != ObjectKind::AbilityOnStack || obj.second_target_req().is_some() {
            return obj.second_target_req();
        }
        let loc = obj.ability?;
        if loc.index == AbilityRef::SYNTHETIC {
            return None;
        }
        let abilities = obj.own_abilities.as_ref().map_or_else(
            || {
                self.state
                    .object(loc.source)
                    .map_or(crate::copiable_abilities::AbilityDefs::EMPTY, |o| {
                        o.printed_abilities(&self.lookup)
                    })
            },
            crate::copiable_abilities::AbilityDefs::from,
        );
        match abilities.get(loc.index as usize)? {
            AbilityDef::Activated { second_targets, .. }
            | AbilityDef::ActivatedConditional { second_targets, .. }
            | AbilityDef::Loyalty { second_targets, .. }
            | AbilityDef::Triggered { second_targets, .. } => *second_targets,
            _ => None,
        }
    }

    /// CR 608.2b, asked of the top of the stack as it begins to resolve.
    ///
    /// > If the spell or ability specifies targets, it checks whether the
    /// > targets are still legal. […] If all its targets, for every instance
    /// > of the word "target," are now illegal, the spell or ability doesn't
    /// > resolve.
    ///
    /// Each instance is re-checked by [`Self::instance_legality`], which
    /// says how; this is where the instances are put back together.
    ///
    /// CR 400.7: each announced slot names an incarnation, not the arena
    /// handle of a card which may since have left and returned.
    fn target_legality(&self, on_stack: ObjectId) -> TargetLegality {
        let Some(obj) = self.state.object(on_stack) else {
            return TargetLegality::NotAsked;
        };
        // "For every instance of the word 'target'": each instance is asked
        // on its own, against its own requirement, and the spell fizzles
        // only when every instance that chose something has lost all of it.
        // A fight whose second creature was bounced still resolves — the
        // first target is legal — and does nothing to either creature,
        // which is CR 701.14b's business in the resolver and not this one's.
        let first = self
            .stack_target_req(on_stack)
            .and_then(|req| self.instance_legality(obj, req, &obj.targets, true));
        let second = self
            .stack_second_target_req(on_stack)
            .and_then(|req| self.instance_legality(obj, req, obj.second_targets(), false));
        if first.is_none() && second.is_none() {
            return TargetLegality::NotAsked;
        }
        let lost = |kept: &Option<(SmallVec<[ObjectId; 2]>, SeatSet)>| {
            kept.as_ref()
                .is_none_or(|(objects, players)| objects.is_empty() && players.is_empty())
        };
        if lost(&first) && lost(&second) {
            return TargetLegality::AllIllegal;
        }
        // An instance that was not asked keeps what it holds.
        let (objects, players) = first.unwrap_or_else(|| (obj.targets.clone(), obj.target_players));
        let second = second.map_or_else(
            || SmallVec::from_slice(obj.second_targets()),
            |(objects, _)| objects.into_iter().collect(),
        );
        TargetLegality::Kept {
            objects,
            players,
            second,
        }
    }

    /// One instance of the word "target", re-checked: what it chose that is
    /// still legal, or `None` when there is nothing of it to ask.
    ///
    /// The question is asked with the very enumeration that offered the
    /// targets in the first place — `eval::stack_target_options`, which is
    /// `eval::target_options` and `eval::target_player_options` with the same
    /// `(you, this)` the cast wizard and `ability_has_a_target` pass. One
    /// predicate read from both ends: an offer and a re-check that disagreed
    /// would be a target the engine let a player choose and then refused to
    /// resolve at. A change of targets (CR 115.7) asks it too.
    ///
    /// `players` is whether this instance is the one whose players ride in
    /// `target_players` — the first, since a second instance is objects only
    /// in every shape that can print one.
    fn instance_legality(
        &self,
        obj: &crate::object::GameObject,
        req: TargetReq,
        chosen: &[ObjectId],
        players: bool,
    ) -> Option<(SmallVec<[ObjectId; 2]>, SeatSet)> {
        // Two specs name no *chosen* target. `EventObject` is the object the
        // trigger fired on, and `Player(rel)` derives its players from a
        // relation at resolution — `resolve::players_of` reads neither list
        // for it. Both enumerate empty by construction, so asking them this
        // question would read every target they have as illegal.
        if matches!(req.spec, TargetSpec::EventObject | TargetSpec::Player(_)) {
            return None;
        }
        // The player half exists only for the three specs that can name one;
        // `eval::targeted_players` says why the bound matters.
        let chosen_players = if players {
            eval::targeted_players(obj, &req.spec)
        } else {
            SeatSet::new()
        };
        // CR 608.2b is about targets that were chosen. A requirement with a
        // minimum of zero, taken with nothing pointed at, has none to lose —
        // and an empty list satisfies "all of them are illegal" vacuously,
        // which would fizzle every untargeted half of the pool.
        if chosen.is_empty() && chosen_players.is_empty() {
            return None;
        }
        let (legal_objects, legal_players) =
            eval::stack_target_options(&self.state, obj, &req.spec);
        let objects: SmallVec<[ObjectId; 2]> = chosen
            .iter()
            .enumerate()
            .filter_map(|(index, &id)| {
                let announced = self.state.recorded_target_reference(
                    obj.id,
                    !players,
                    u32::try_from(index).expect("target slot"),
                );
                (legal_objects.contains(&id)
                    && announced.is_some()
                    && announced == self.state.source_identity(id))
                .then_some(id)
            })
            .collect();
        let mut kept = SeatSet::new();
        for player in chosen_players.iter() {
            if legal_players.contains(&player) {
                kept.insert(player);
            }
        }
        Some((objects, kept))
    }

    /// CR 608.2b's removal: off the stack, without having resolved.
    ///
    /// **Not [`Self::finalize_spell`]**, which is the path a spell that *did*
    /// resolve takes and owes two riders this one does not. Rebound exiles a
    /// spell "as it resolves" (CR 702.88) and an Adventure likewise
    /// (CR 715.3d); a spell that never resolved has done neither, and
    /// borrowing that function would have given a fizzled Ephemerate its
    /// rebound. Flashback is the rider that *does* apply, because CR 702.34a
    /// exiles the card "any time it would leave the stack" rather than on
    /// resolution.
    fn leave_stack_without_resolving(&mut self, top: ObjectId) {
        let Some(obj) = self.state.object(top) else {
            return;
        };
        if obj.kind == ObjectKind::AbilityOnStack {
            // An ability ceases to exist rather than going anywhere
            // (CR 608.2n) — the same removal CR 603.4's arm above makes.
            self.state.zones.remove(top, ZoneLocation::Stack);
            self.state.remember_damage_source(top);
            let _ = self.state.arena.remove(top);
            return;
        }
        let owner = obj.owner;
        let flashback = obj.riders.contains(&crate::object::Rider::Flashback);
        if let Some(obj) = self.state.object_mut(top) {
            obj.kind = ObjectKind::Card;
        }
        let destination = if flashback {
            ZoneLocation::Exile(owner)
        } else {
            ZoneLocation::Graveyard(owner)
        };
        // `Cause::Spell` and not `Cause::Effect`: nothing's effect moved this
        // card. It is the same clause's other half — CR 608.2n puts a
        // resolved spell in its owner's graveyard and `finalize_spell` calls
        // that `Cause::Spell` — arriving by the door one sentence earlier.
        let _ = self
            .state
            .move_object(top, destination, ZonePosition::Top, Cause::Spell);
    }

    #[allow(clippy::too_many_lines)] // resolution dispatch is a flat router; extraction would obscure it
    pub(crate) fn resolve_stack_top(&mut self) {
        let Some(&top) = self.state.zones.list(ZoneLocation::Stack).last() else {
            return;
        };
        // CR 603.4, asked before anything records that this resolved: an
        // ability whose intervening-`if` clause is no longer true "is removed
        // from the stack and does nothing". Not a counter and not a
        // resolution — a journal that said `StackObjectResolved` here and
        // then took the ability away would be describing a different rule to
        // everything that reads it.
        if self.intervening_if_failed(top) {
            self.state
                .journal
                .record(GameEvent::StackObjectDidNotResolve { object: top });
            // As when an ability is countered: an ability on the stack
            // ceases to exist rather than going anywhere (CR 608.2n).
            self.state.zones.remove(top, ZoneLocation::Stack);
            self.state.remember_damage_source(top);
            let _ = self.state.arena.remove(top);
            return;
        }
        // CR 608.2b, asked in the same place and for the same reason: a
        // spell or ability all of whose targets have become illegal does not
        // resolve. Before the spell/ability split below, because an Aura is
        // a targeted *permanent* spell and a check inside either branch
        // would miss one of them.
        let mut resolution_object = self.state.object(top).expect("stack object exists").clone();
        match self.target_legality(top) {
            TargetLegality::AllIllegal => {
                self.state
                    .journal
                    .record(GameEvent::StackObjectDidNotResolve { object: top });
                self.leave_stack_without_resolving(top);
                return;
            }
            // "…won't do anything to an illegal target" (CR 608.2b): the
            // rest of it still happens. Narrow a local resolution image,
            // preserving the original announcement for copies and history.
            // Every resolution constructor below reads this same image.
            //
            // Safe to narrow because a `TargetReq` carries **one** spec: the
            // positions in `targets` are a set and not a tuple, and nothing
            // in this crate reads one by index. A second instance of the
            // word "target" is the one place that would not hold, which is
            // why it is a list of its own and is narrowed on its own line:
            // a fight whose first creature became illegal must not find its
            // second creature standing in the first one's place.
            TargetLegality::Kept {
                objects,
                players,
                second,
            } => {
                let obj = &mut resolution_object;
                if obj.targets.len() != objects.len() {
                    obj.targets = objects;
                }
                if obj.second_targets().len() != second.len() {
                    let req = obj.second_target_req();
                    obj.set_second(second, req);
                }
                if obj.target_players != players {
                    obj.target_players = players;
                    // `chosen_player` is the same choice written twice
                    // and `resolve::players_of` reads *it* for
                    // `PlayerRel::Chosen`. Left behind, a player who
                    // gained hexproof in response would still be dealt
                    // to by the half of the spell that reads the scalar.
                    obj.chosen_player = obj.chosen_player.filter(|p| players.contains(*p));
                }
            }
            TargetLegality::NotAsked => {}
        }
        self.begin_controlled_resolution(top);
        self.state
            .journal
            .record(GameEvent::StackObjectResolved { object: top });
        let kind = self.state.object(top).map(|o| o.kind);
        if kind == Some(ObjectKind::AbilityOnStack) {
            // "The second time this ability has resolved this turn": counted
            // here, as it begins to resolve, so the resolution asking is one
            // of those it counts. A synthetic keyword trigger has no index of
            // its own to count under.
            if let Some(loc) = self.state.object(top).and_then(|o| o.ability)
                && loc.index != baylee_core::ids::AbilityRef::SYNTHETIC
            {
                let version = self.state.object(loc.source).map_or(0, |o| o.version);
                self.state
                    .per_turn
                    .note_resolution(loc.source, version, loc.index);
            }
            let obj = &resolution_object;
            let loc = obj.ability.expect("ability object has a location");
            // The list `loc.index` points into, captured when the ability was
            // put on the stack — a card face, an emblem's stored list, a
            // token's definition or a copy's, all the same case by then. It
            // is read from the ability object rather than from the source
            // because the two have been separate objects since it was put
            // there (CR 113.7a): the source may have died, changed face, or
            // stopped being a copy in the meantime.
            let abilities = obj.own_abilities.as_ref().map_or_else(
                || {
                    self.state
                        .object(loc.source)
                        .map_or(crate::copiable_abilities::AbilityDefs::EMPTY, |o| {
                            o.printed_abilities(&self.lookup)
                        })
                },
                crate::copiable_abilities::AbilityDefs::from,
            );
            let effects = if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC {
                // Synthetic keyword trigger (prowess, ward): effects live
                // in the side map, resolved below.
                &[][..]
            } else {
                match abilities.get(loc.index as usize) {
                    // `ActivatedConditional` belongs here beside `Activated`:
                    // the condition is a restriction on *activating* it —
                    // CR 602.5, "a player can't begin to activate an ability
                    // that's prohibited from being activated" — checked once
                    // in `start_activation` and spent there. What reaches the
                    // stack is an ordinary ability,
                    // and leaving it out of this arm meant every conditional
                    // ability that uses the stack panicked the engine as it
                    // resolved — Wizard Class could be levelled and not
                    // survive it. The `targeted` match just below had it all
                    // along, which is why nothing else noticed.
                    Some(
                        AbilityDef::Activated { effects, .. }
                        | AbilityDef::ActivatedConditional { effects, .. }
                        | AbilityDef::Triggered { effects, .. }
                        | AbilityDef::Loyalty { effects, .. }
                        | AbilityDef::SagaChapter { effects, .. },
                    ) => *effects,
                    Some(AbilityDef::ModalTriggered { modes, .. }) => {
                        // `expect` and not "mode 0 if nobody said": the mode
                        // is announced as the ability is put on the stack
                        // (CR 603.3c), so an ability that reached resolution
                        // without one came off a push site that forgot to
                        // carry it — and falling back to the first mode is
                        // how a card resolves the wrong half of itself in
                        // silence, which is the fault entry 34 is about.
                        let idx = obj
                            .mode_index
                            .expect("a modal trigger on the stack has its mode")
                            as usize;
                        modes
                            .get(idx)
                            .map(|m| m.effects)
                            .expect("modal trigger mode exists")
                    }
                    _ => panic!(
                        "ability object references non-resolvable ability: source {:?} index {}",
                        loc.source, loc.index
                    ),
                }
            };
            // Whether this ability said "target" at all, which is what tells
            // `Filter::This` apart from itself — see `Resolution::targeted`.
            //
            // The arm list this used to spell out is `stack_target_req`,
            // which CR 608.2b's check above already needs: two copies of a
            // match over `Activated` and `ActivatedConditional` is two
            // chances to add a variant to one of them, and this workspace
            // has found six readers matching one twin and not the other.
            let targeted = self.stack_target_req(top).is_some();
            if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC {
                // Synthetic keyword trigger (prowess & co.): effects live in
                // the side map instead of the card definition.
                let synthetic = self
                    .synthetic_fx
                    .remove(&top)
                    .expect("synthetic trigger has effects");
                let mut res = Resolution {
                    source: loc.source,
                    on_stack: top,
                    controller: obj.controller,
                    effects: resolve::flatten(synthetic),
                    pc: 0,
                    targets: obj.targets.clone(),
                    second_targets: SmallVec::from_slice(obj.second_targets()),
                    x: None,
                    chosen_player: obj.chosen_player,
                    target_players: obj.target_players,
                    event_object: obj.event_object,
                    // Whether it said "target", which a synthetic trigger
                    // with a requirement did: see `stack_target_req`.
                    targeted,
                    awaiting: None,
                    mana_ability: false,
                    countered_source: None,
                    target_lki: None,
                    subject: crate::resolve::SubjectContext::default(),
                    text: crate::text_changes::TextChangeMap::IDENTITY,
                    event_mana: None,
                    retarget_left: None,
                };
                match resolve::run(&mut self.state, &mut res) {
                    resolve::Flow::Complete => self.finish_resolution(&res),
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                }
                return;
            }
            let mut res = Resolution {
                source: loc.source,
                on_stack: top,
                controller: obj.controller,
                effects: resolve::flatten(effects),
                pc: 0,
                targets: obj.targets.clone(),
                second_targets: SmallVec::from_slice(obj.second_targets()),
                // Zero on every ability the pool prints today, and read
                // rather than assumed because an activation with a counter-X
                // cost writes one here (`push_ability_to_stack`). `Some(0)`
                // and `None` are the same number to `eval::amount`.
                x: Some(obj.x_value),
                chosen_player: obj.chosen_player,
                target_players: obj.target_players,
                event_object: obj.event_object,
                targeted,
                awaiting: None,
                mana_ability: false,
                countered_source: None,
                target_lki: None,
                subject: crate::resolve::SubjectContext::default(),
                text: crate::text_changes::TextChangeMap::IDENTITY,
                event_mana: None,
                retarget_left: None,
            };
            #[cfg(test)]
            crate::ability_log::resolving(&self.state, &self.lookup, top);
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => self.finish_resolution(&res),
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
            }
            return;
        }
        // A spell resolves (plain spells and modal spells alike — the
        // chosen mode is stored on the spell object).
        let spell_fx = self
            .state
            .object(top)
            .and_then(|o| {
                let face = o.face_index as usize;
                o.card
                    .and_then(|c| self.lookup.card(c.index))
                    .map(|def| def.abilities_for_face(face))
            })
            .and_then(|abilities| {
                abilities.iter().find_map(|a| match a {
                    AbilityDef::Spell {
                        effects, targets, ..
                    } if !effects.is_empty() => {
                        Some((resolve::flatten(effects), targets.is_some(), None))
                    }
                    _ => None,
                })
            })
            .or_else(|| self.modal_program(top));
        if let Some((program, targeted, retarget_left)) = spell_fx {
            let obj = &resolution_object;
            let mut res = Resolution {
                source: top,
                on_stack: top,
                controller: obj.controller,
                effects: program,
                pc: 0,
                targets: obj.targets.clone(),
                second_targets: SmallVec::from_slice(obj.second_targets()),
                x: Some(obj.x_value),
                chosen_player: obj.chosen_player,
                target_players: obj.target_players,
                event_object: None,
                targeted,
                awaiting: None,
                mana_ability: false,
                countered_source: None,
                target_lki: None,
                subject: crate::resolve::SubjectContext::default(),
                text: crate::text_changes::TextChangeMap::IDENTITY,
                retarget_left,
                event_mana: None,
            };
            #[cfg(test)]
            crate::ability_log::resolving(&self.state, &self.lookup, top);
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => self.finish_resolution(&res),
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
            }
        } else {
            #[cfg(test)]
            crate::ability_log::resolved(&self.state, &self.lookup, top);
            self.finalize_spell(top);
        }
    }

    /// What a modal spell on the stack does: its chosen mode's effects, or,
    /// for a spell cast with several (`GameObject::modes`), every chosen
    /// mode's, in the order they are printed and not the order they were
    /// picked in (CR 608.2c).
    ///
    /// Beside the program, whether it targets, and where the second of its
    /// modes that says "target" begins: that mode's targets were chosen as
    /// the spell's second instance of the word, and from there on they are
    /// the ones "target" means (`Resolution::retarget_left`).
    fn modal_program(&self, top: ObjectId) -> Option<(Vec<Effect>, bool, Option<usize>)> {
        let obj = self.state.object(top)?;
        let def = obj.card.and_then(|c| self.lookup.card(c.index))?;
        let modes = def
            .abilities_for_face(obj.face_index as usize)
            .iter()
            .find_map(|a| match a {
                AbilityDef::ModalSpell { modes, .. } => Some(*modes),
                _ => None,
            })?;
        if obj.modes != 0 {
            let mut program = Vec::new();
            let mut targeting = 0_usize;
            let mut second = None;
            for (_, mode) in crate::casting::chosen_modes(modes, obj.modes) {
                if mode.targets.is_some() {
                    if targeting == 1 {
                        second = Some(program.len());
                    }
                    targeting += 1;
                }
                program.extend(resolve::flatten(mode.effects));
            }
            let left = second.map(|at| program.len() - at);
            return Some((program, targeting > 0, left));
        }
        let mode = modes.get(usize::from(obj.mode_index?))?;
        Some((resolve::flatten(mode.effects), mode.targets.is_some(), None))
    }

    /// Applies a face switch queued by a resolution effect (transforms).
    pub(crate) fn apply_pending_face_changes(&mut self) {
        let pending: Vec<(ObjectId, u8)> = self
            .state
            .arena
            .iter()
            .filter_map(|(id, o)| o.pending_face_change.map(|f| (id, f)))
            .collect();
        for (id, face) in pending {
            if let Some(obj) = self.state.object_mut(id) {
                obj.pending_face_change = None;
            }
            if let Some(def) = self
                .state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            {
                self.state.transform(id, def, face as usize);
            }
        }
    }

    /// Hands every token copy the printed rules text it was created owing
    /// (CR 707.2), from [`GameState::pending_copied_faces`].
    ///
    /// The same division of labour as [`Self::day_night_statics`] below, for
    /// the same reason: a face's abilities are behind the card registry,
    /// `resolve` has no lookup for it and cannot be given one — the rules
    /// kernel does not depend on `baylee-cards` — so the copy is created
    /// naming the face it copied and is finished here, at the nearest place
    /// that holds a lookup.
    ///
    /// It runs first in the pass rather than beside the daybound checks
    /// because what follows in the same pass is what asks a permanent what
    /// it can do: [`Self::sync_static_effects`] on the very next line, then
    /// the trigger scan and the offer, all three reading
    /// [`GameObject::abilities`], and a copy still answering an empty list
    /// would have its text a whole priority window late.
    ///
    /// `own_abilities` wins where both are set: a copy of a copy was handed
    /// the list it is copying, and that list is the copiable one — the card
    /// underneath it is not what it is a copy of.
    pub(super) fn settle_copied_rules_text(&mut self) {
        if self.state.pending_copied_faces.is_empty() {
            return;
        }
        for (id, card, face) in std::mem::take(&mut self.state.pending_copied_faces) {
            let printed = self
                .lookup
                .card(card)
                .map(|def| crate::object::AbilityList {
                    token: None,
                    abilities: def.abilities_for_face(face as usize).into(),
                    printed: crate::object::PrintedFace::new(card, face),
                });
            if let Some(obj) = self.state.object_mut(id)
                && obj.own_abilities.is_none()
                && let Some(printed) = printed
            {
                obj.take_abilities(printed);
            }
        }
    }

    /// Daybound and nightbound's continuous checks (CR 702.145c–g).
    ///
    /// They are not state-based actions — CR 702.145c and f say so in as
    /// many words — so they do not belong in [`sba::run`], and they need
    /// the card definition behind a permanent to know how many faces it
    /// has, which `sba::run` has no lookup for. They run as their own step
    /// of the machine's fixpoint, after the state-based actions have
    /// settled and before triggers are collected, so that a permanent that
    /// turns over does so before anything asks what triggered.
    ///
    /// Returns whether anything changed, which sends the fixpoint round
    /// again.
    ///
    /// The order inside is the order the rules fall in: the two that hand
    /// a game with *neither* designation one (d, then g) come before the
    /// two that read the designation (c and f). Otherwise a lone daybound
    /// creature entering a fresh game would wait a whole iteration for the
    /// day it is about to cause.
    pub(super) fn day_night_statics(&mut self) -> bool {
        use baylee_cards_dsl::KeywordSet as K;
        // The common case by a wide margin: no daybound card at the table,
        // so the whole step is one scan of the battlefield and out. A
        // phased-out permanent is not at the table (CR 702.26b).
        let mut any_daybound = false;
        let mut any_nightbound = false;
        for id in self.state.battlefield_seen() {
            let Some(kw) = self.state.object(id).map(|o| o.characteristics().keywords) else {
                continue;
            };
            any_daybound |= kw.contains(K::DAYBOUND);
            any_nightbound |= kw.contains(K::NIGHTBOUND);
        }
        if !any_daybound && !any_nightbound {
            return false;
        }
        // CR 702.145d, then g. The nightbound clause is the conditional
        // one: it makes it night only when no daybound permanent is on the
        // battlefield at all, which is why both flags are collected before
        // either is acted on.
        if self.state.day_night.is_none() {
            if any_daybound {
                self.state.become_day();
                return true;
            }
            self.state.become_night();
            return true;
        }
        // CR 702.145c and f: front face up with daybound at night, or back
        // face up with nightbound by day, turns over. "Immediately", and by
        // its controller — but the transform is the whole of it, so there
        // is nobody to ask.
        let night = self.state.day_night == Some(DayNight::Night);
        let turning: Vec<ObjectId> = self
            .state
            .battlefield_seen()
            .filter(|&id| {
                let Some(obj) = self.state.object(id) else {
                    return false;
                };
                let kw = obj.characteristics().keywords;
                (night && obj.face_index == 0 && kw.contains(K::DAYBOUND))
                    || (!night && obj.face_index == 1 && kw.contains(K::NIGHTBOUND))
            })
            .collect();
        let mut changed = false;
        for id in turning {
            // CR 701.27c: only a permanent represented by a transforming
            // double-faced card can transform. A token has no card, and a
            // clone of a werewolf carries the copied keyword over a card
            // with one face — turning either one over would rebuild its
            // base from a face that is not there and wipe the copy. Both
            // are skipped *without* reporting a change, or the fixpoint
            // would find work to do forever.
            let Some(def) = self
                .state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            else {
                continue;
            };
            if def.faces.len() < 2 {
                continue;
            }
            changed |= self.state.transform(id, def, usize::from(night));
        }
        changed
    }

    pub(crate) fn finish_resolution(&mut self, res: &Resolution) {
        #[cfg(test)]
        if res.mana_ability {
            crate::ability_log::mana_finished(&self.lookup, res.source);
        } else {
            crate::ability_log::resolved(&self.state, &self.lookup, res.on_stack);
        }
        // The reflexive triggers this resolution created (CR 603.12) join
        // the queue as it ends, and from there take the ordinary path.
        // `queue_new_triggers` sorts them with that pass's other triggers
        // (CR 603.3b), and `collect_triggers` stacks them, asking for a
        // target or dropping one with none (CR 603.3d). It happens here,
        // the first thing every completed stack resolution passes, and
        // not in `queue_new_triggers`. Step 0b of `run_machine` can publish
        // a question (an as-enters choice) before that runs, and the list
        // must be empty whenever a question is out.
        self.trigger_queue.extend(self.state.reflexive.drain(..));
        // What this resolution left for the engine to do as it ends
        // (cascade's cast, CR 702.85a), ahead of everything already queued.
        let mut i = self.state.delayed.len();
        while i > 0 {
            i -= 1;
            if matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::AsResolutionEnds
            ) {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue
                    .push_front((trigger.controller, trigger.action));
            }
        }
        // A card this resolution discovered is offered once it is over
        // (CR 701.57a), through the queue step 3b of `run_machine` drains
        // before anybody receives priority. CR 608.2g casts it *during* the
        // resolution; the difference is only in what the resolution's own
        // triggers see, which step 3 stacks first.
        for (player, card, version) in self.state.discovered.drain(..) {
            self.delayed_queue.push_back((
                player,
                crate::state::DelayedAction::CastDiscovered { card, version },
            ));
        }
        // A copy of a synthetic ability (CR 707.10) takes the original's
        // effects, which live here and not on the object the resolver made.
        for (original, copy) in std::mem::take(&mut self.state.synthetic_copies) {
            if let Some(&effects) = self.synthetic_fx.get(&original) {
                self.synthetic_fx.insert(copy, effects);
            }
        }
        // A mana ability never went on the stack (CR 605.3b), and its
        // `on_stack` is the source permanent itself. Falling through here
        // treated that permanent as a resolving spell: `finalize_spell`
        // untapped the land and "moved" it to the battlefield it was
        // already on, so every colour-choice source — Badlands, City of
        // Brass, Command Tower, Harabaz Druid — untapped itself and made
        // unbounded mana. Nothing to finalize; the activating player keeps
        // priority (CR 605.3a).
        if res.mana_ability {
            // A triggered mana ability may belong to another player (Mana
            // Flare). Its choice returns to the original mana activator.
            self.after_action(res.event_mana.map_or(res.controller, |event| event.player));
            return;
        }
        if self
            .state
            .object(res.on_stack)
            .is_some_and(|o| o.kind == ObjectKind::AbilityOnStack)
        {
            // Abilities on the stack simply cease to exist (CR 608.2n).
            //
            // CR 714.4's sacrifice used to be spelled out here, on the way
            // past: a resolving chapter carried its own Saga to the
            // graveyard. It is a state-based action and now runs as one, in
            // [`Self::finished_sagas`] — which is what a chapter ability
            // that *never resolves* needs. Countering the last chapter left
            // the Saga on the battlefield for the rest of the game.
            self.state.zones.remove(res.on_stack, ZoneLocation::Stack);
            self.state.remember_damage_source(res.on_stack);
            let _ = self.state.arena.remove(res.on_stack);
        } else {
            self.finalize_spell(res.on_stack);
        }
        self.apply_pending_face_changes();
        self.end_controlled_resolution(res.on_stack);
    }

    /// CR 707.10: a copy of a permanent spell stops being a copy of a spell
    /// and becomes a **token** permanent as it resolves — which is what
    /// Storm of Saruman's own reminder text says.
    ///
    /// Being a token is exactly "carries no card" here (`Filter::IsToken`),
    /// so the card has to go, and the rules text it would have answered with
    /// moves into `own_abilities` first: that is the slot a card-less object
    /// keeps its abilities in, and the one `move_object` leaves alone for an
    /// object with no card.
    ///
    /// The `SpellCopy` rider goes with it. It said "cease to exist off the
    /// stack" (CR 704.5e); from here CR 704.5d says the same thing about the
    /// token, and leaving both on would have the cleanup pass answer for one
    /// object twice.
    fn a_copy_becomes_a_token(&mut self, spell: ObjectId) {
        let is_copy = self
            .state
            .object(spell)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::SpellCopy));
        if !is_copy {
            return;
        }
        let printed = self
            .state
            .object(spell)
            .map(|o| o.printed_ability_list(&self.lookup));
        if let Some(obj) = self.state.object_mut(spell) {
            if obj.own_abilities.is_none()
                && let Some(printed) = printed
            {
                obj.take_abilities(printed);
            }
            obj.card = None;
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::SpellCopy));
        }
    }

    // Adventure, the permanent door with the copy question in front of it,
    // rebound, flashback and the graveyard — five endings for one spell, and
    // each carries the rule that picks it.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn finalize_spell(&mut self, spell: ObjectId) {
        let (is_permanent, owner) = {
            let Some(obj) = self.state.object(spell) else {
                return;
            };
            // CR 608.2m lets a spell that leaves the stack part-way through
            // finish resolving; CR 608.2n then puts *the spell* into its
            // owner's graveyard — and a card its own effect has already
            // moved is not one. This asked neither question and moved
            // whatever id it was handed, so `Effect::ExileSource` exiled the
            // card and the finalisation fetched it straight back out:
            // Teferi's Protection, Temporal Mastery and Spirit Water Revival
            // all had the one clause the DSL can express undone one step
            // later. A permanent spell is unaffected — it is still on the
            // stack when this runs, which is the whole reason this line can
            // be a single zone test rather than a per-effect flag.
            if obj.zone != crate::zone::Zone::Stack {
                return;
            }
            (obj.characteristics().types.is_permanent(), obj.owner)
        };
        // Adventure (CR 715): an Adventure spell resolves to exile; the
        // front face may then be cast from exile.
        let adventure_def = {
            let face = self
                .state
                .object(spell)
                .map_or(0, |o| o.face_index as usize);
            self.state
                .object(spell)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .filter(|def| def.faces.get(face).is_some_and(|f| f.adventure))
        };
        if let Some(def) = adventure_def {
            // The card is exiled *on an adventure*, and an adventurer card
            // has its normal characteristics in every zone but the stack: it
            // is the creature that sits in exile, not the instant that just
            // resolved. The face was left where the cast put it, so the
            // exiled card kept Swift Spiral's name, its instant type and its
            // `{1}{W}`. Everything downstream reads the object: `can_cast`
            // probed `{1}{W}` and asked the timing of an *instant*, and the
            // wizard's back-face loop priced the same face, so the adventure
            // was castable out of its own exile for two white mana, at
            // instant speed, every turn, for as long as the card sat there.
            self.state.switch_face(spell, def, 0);
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Card;
                obj.riders.push(crate::object::Rider::Adventure);
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            return;
        }
        if is_permanent {
            self.state.begin_permanent_resolution(spell);
            // Dash (CR 702.109a): "return the permanent this spell becomes to
            // its owner's hand at the beginning of the next end step" — a
            // delayed triggered ability that uses the stack (CR 603.7), and
            // asks as it resolves whether it is still that permanent.
            if let Some(obj) = self.state.object(spell)
                && obj.riders.contains(&crate::object::Rider::Dashed)
            {
                let controller = obj.controller;
                self.state
                    .delay_for_resolved_permanent(spell, controller, &DASH_RETURN);
            }
            self.a_copy_becomes_a_token(spell);
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Permanent;
            }
            self.state.set_tapped(spell, false);
            // CR 614.12a: a choice a replacement effect needs is made
            // *before* the permanent enters. Publishing a `Pending` returns
            // from here and the move is owed to the answer — see
            // `PlanKind::CopyOnEnter::before_entry`, which is where it is
            // paid.
            if self.ask_copy_before_entry(spell) {
                return;
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Spell,
            );
        } else {
            // Rebound (CR 702.88): cast from hand → exile with a rebound
            // rider and a delayed re-cast at the next upkeep.
            let rebound = self.state.object(spell).is_some_and(|o| {
                o.cast_from_hand
                    && o.characteristics()
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::REBOUND)
            });
            if rebound {
                if let Some(obj) = self.state.object_mut(spell) {
                    obj.kind = ObjectKind::Card;
                    obj.riders.push(crate::object::Rider::Rebound);
                }
                let _ = self.state.move_object(
                    spell,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                self.state.delayed.push(crate::state::DelayedTrigger {
                    controller: owner,
                    when: crate::state::DelayedWhen::NextUpkeep,
                    action: crate::state::DelayedAction::CastFromExileWithoutPaying {
                        card: spell,
                        version: self.state.object(spell).map_or(0, |o| o.version),
                    },
                });
                return;
            }
            // Flashback (CR 702.34): exile instead of the graveyard.
            let flashback = self
                .state
                .object(spell)
                .is_some_and(|o| o.riders.contains(&crate::object::Rider::Flashback));
            if flashback {
                if let Some(obj) = self.state.object_mut(spell) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = self.state.move_object(
                    spell,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                return;
            }
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Card;
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Spell,
            );
        }
    }
}
