use super::{
    AbilityDef, AttackerInfo, CardLookup, Cause, CombatDeclared, Engine, EngineError, GameEvent,
    LossReason, ObjectId, ObjectKind, PaymentWindow, Pending, PlanKind, PlayerAction, PlayerId,
    SmallVec, Zone, ZoneLocation, ZonePosition, cast_wizard, casting, combat, mana_pay, resolve,
    sba,
};
use crate::choice::CastModeKind;

impl<L: CardLookup> Engine<L> {
    /// Asks a commander's owner whether it goes to the command zone rather
    /// than staying in the graveyard or in exile (CR 903.9a).
    ///
    /// The pass that found it has already written down that it asked, so
    /// this must not be the place a question can be dropped.
    pub(crate) fn ask_commander_zone(&mut self, player: PlayerId, card: ObjectId) {
        self.pending_plan = Some(PlanKind::CommanderZone { card });
        self.pending = Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::CommanderZone { card },
            // The rules ask this question, not the card — but they ask it
            // *about* a card, and that is what a standing answer has to be
            // filed under. "Always put Katara back" is an answer about
            // Katara, not about every commander this seat will ever have.
            source: self.state.object(card).and_then(|o| o.card).map(|c| {
                baylee_core::ids::AbilityRef::new(
                    c.index,
                    baylee_core::ids::AbilityRef::COMMANDER_ZONE,
                )
            }),
        };
        self.awaiting_answer = true;
    }

    /// Offers a draw to every other player still in the game (CR 104.4i).
    ///
    /// Only from your own priority: the offer suspends a decision that has to
    /// be handed back untouched if anyone refuses, and priority is the only
    /// point where the game is quiet enough for that to be true.
    pub(crate) fn offer_draw(&mut self, player: PlayerId) -> Result<(), EngineError> {
        let Pending::Priority { player: holder, .. } = &self.pending else {
            return Err(EngineError::IllegalAction("a draw offer needs priority"));
        };
        if *holder != player {
            return Err(EngineError::IllegalAction("a draw offer needs priority"));
        }
        let mut remaining: Vec<PlayerId> = self
            .state
            .players
            .iter()
            .filter(|p| !p.has_lost() && p.id != player)
            .map(|p| p.id)
            .collect();
        if remaining.is_empty() {
            return Err(EngineError::IllegalAction("nobody left to offer a draw to"));
        }
        let asked = remaining.remove(0);
        self.pending_plan = Some(PlanKind::DrawOffer {
            proposer: player,
            remaining,
            resume: Box::new(self.pending.clone()),
        });
        self.pending = Pending::YesNo {
            player: asked,
            prompt: crate::choice::YesNoPrompt::DrawOffer { proposer: player },
            // Not a card asking: no ability to remember an answer for.
            source: None,
        };
        self.awaiting_answer = true;
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // flat match over the choice taxonomy — splitting would obscure, not clarify
    pub(crate) fn apply_inner(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
    ) -> Result<(), EngineError> {
        // "Target creature" and "any target" are one prompt with one answer;
        // the second shape exists only because a player has no `ObjectId`.
        // Normalising here keeps that a detail of the wire format instead of
        // a fork in every step targeting reaches.
        let action = match action {
            PlayerAction::ChooseObjects { objects }
                if matches!(self.pending, Pending::ChooseTargets { .. }) =>
            {
                PlayerAction::ChooseTargets {
                    objects,
                    players: Vec::new(),
                }
            }
            other => other,
        };
        match (&self.pending, action) {
            (Pending::Mulligan { player: p, .. }, PlayerAction::MulliganKeep) if *p == player => {
                let taken = self.mulligans[player.get() as usize];
                let bottom = self.mulligan_bottom_count(taken);
                if bottom > 0 {
                    self.pending = Pending::MulliganBottom {
                        player,
                        count: bottom,
                    };
                    self.awaiting_answer = true;
                } else {
                    self.advance_mulligan();
                }
                Ok(())
            }
            (
                Pending::Mulligan {
                    player: p, taken, ..
                },
                PlayerAction::MulliganTake,
            ) if *p == player => {
                // Hand goes back, reshuffle, draw 7 (CR 103.5).
                let hand: Vec<_> = self.state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                self.state.shuffle_library(player);
                self.state.draw_cards(player, 7);
                let taken = *taken + 1;
                self.mulligans[player.get() as usize] = taken;
                self.pending = Pending::Mulligan {
                    player,
                    taken,
                    next_is_free: false,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            (
                Pending::MulliganBottom { player: p, count },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                // `[c, c]` has the right length and both halves are in hand;
                // it bottomed one card for two and kept an eighth (CR 103.5).
                if objects.len() != *count as usize || names_one_twice(&objects) {
                    return Err(EngineError::IllegalAction(
                        "must bottom exactly the required number of cards",
                    ));
                }
                for card in &objects {
                    if !self.in_hand(player, *card) {
                        return Err(EngineError::IllegalAction("card not in hand"));
                    }
                }
                for card in objects {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                self.advance_mulligan();
                Ok(())
            }
            // Passing inside a CR 605.3a payment window says "I have made
            // what mana I am going to make", and must not reach the arm
            // below: a pass counted toward the priority round would, once
            // every seat had passed, resolve the top of the stack while the
            // resolution that asked for this payment is still suspended
            // underneath it.
            (Pending::Priority { player: p, .. }, PlayerAction::PassPriority)
                if *p == player
                    && self
                        .mana_window
                        .as_ref()
                        .is_some_and(|w| w.player == player) =>
            {
                self.close_mana_window();
                Ok(())
            }
            (Pending::Priority { player: p, .. }, PlayerAction::PassPriority) if *p == player => {
                self.passes += 1;
                Ok(())
            }
            (Pending::Priority { player: p, legal }, PlayerAction::PlayLand { card })
                if *p == player =>
            {
                if !legal.lands.contains(&card) {
                    return Err(EngineError::IllegalAction("land not playable now"));
                }
                // MDFC: which land face is played (CR 712.12)? Only the
                // faces the offer counted, so a transforming card's land back
                // is never one of them.
                let land_faces: Vec<usize> = self
                    .state
                    .object(card)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))
                    .map_or_else(|| vec![0], |def| def.land_faces_from_hand().collect());
                if land_faces.len() > 1 {
                    // Both faces are lands (pathways): choose.
                    let options = land_faces
                        .iter()
                        .map(|&i| crate::choice::CastModeDesc {
                            index: i as u8,
                            kind: crate::choice::CastModeKind::PlayLandFace(i),
                            cost: baylee_core::mana::ManaCost::ZERO,
                        })
                        .collect();
                    self.pending_plan = Some(PlanKind::PlayLandFace { card });
                    self.pending = Pending::ChooseCastMode {
                        player,
                        object: card,
                        options,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                if let Some(&face) = land_faces.first()
                    && face > 0
                {
                    let def = self
                        .state
                        .object(card)
                        .and_then(|o| o.card)
                        .and_then(|c| self.lookup.card(c.index))
                        .expect("land card known");
                    self.state.switch_face(card, def, face);
                }
                casting::play_land(&mut self.state, player, card)?;
                self.after_action(player);
                Ok(())
            }
            (Pending::Priority { player: p, legal }, PlayerAction::CastSpell { card })
                if *p == player =>
            {
                if !legal.castable.contains(&card) {
                    return Err(EngineError::IllegalAction("spell not castable now"));
                }
                self.start_cast_wizard(player, card)
            }
            (Pending::ChooseCastMode { player: p, .. }, PlayerAction::ChooseMode(index))
                if *p == player =>
            {
                // `take` once. Two `if let Some(…) = self.pending_plan.take()`
                // in a row is one condition and two takes: the first arm
                // *consumes* the plan whatever it holds, so the second could
                // only ever see `None`. The modal-trigger branch below was
                // unreachable for that reason as well as for the one entry 34
                // names, and one fault was hiding the other.
                let plan = self.pending_plan.take();
                // What the option at this position actually names. A modal
                // trigger drops the modes it cannot legally choose
                // (CR 603.3c), so the position answered is not the mode
                // number — and the answer has to be inside the list that was
                // offered, which no arm here used to check.
                let kind = match &self.pending {
                    Pending::ChooseCastMode { options, .. } => options.get(index).map(|o| o.kind),
                    _ => None,
                };
                match plan {
                    // MDFC land-face choice (pathways).
                    Some(PlanKind::PlayLandFace { card }) => {
                        let def = self
                            .state
                            .object(card)
                            .and_then(|o| o.card)
                            .and_then(|c| self.lookup.card(c.index))
                            .expect("land card known");
                        self.state.switch_face(card, def, index);
                        casting::play_land(&mut self.state, player, card)?;
                        self.after_action(player);
                        return Ok(());
                    }
                    // Modal trigger mode choice (CR 603.3c). The mode is
                    // written back onto the queued trigger rather than
                    // stacked here: `collect_triggers` runs again on the way
                    // out of `apply`, sees a mode it does not have to ask
                    // about, and takes the ordinary path — which is what asks
                    // for the *mode's* targets, records `once_per_turn` and
                    // stacks it. Stacking it here skipped all three.
                    Some(PlanKind::ModalTrigger {
                        source,
                        ability_index,
                    }) => {
                        let Some(CastModeKind::Mode(mode)) = kind else {
                            return Err(EngineError::IllegalAction("no such cast mode"));
                        };
                        let Some(front) = self.trigger_queue.front_mut() else {
                            return Err(EngineError::IllegalAction("no trigger awaiting a mode"));
                        };
                        debug_assert!(
                            front.source == source && front.ability_index == ability_index,
                            "the modal plan and the queue's front are the same trigger",
                        );
                        front.chosen_mode = Some(mode as u8);
                        return Ok(());
                    }
                    _ => {}
                }
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                let Some(option) = wizard.options.get(index).map(|o| o.kind) else {
                    return Err(EngineError::IllegalAction("no such cast mode"));
                };
                wizard.option = Some(option);
                // CR 601.2b announces the mode and *then* the value of X, in
                // the same step: choosing how to cast the spell does not
                // answer what X is. Going to `Targets` here skipped the
                // question for every spell with more than one way to be cast
                // — Heliod's Intervention was cast for X = 0 with nobody
                // asked, the multi-option twin of the defect `XValue`'s own
                // comment describes. It falls through to `Targets` when the
                // chosen cost has no X.
                wizard.stage = cast_wizard::WizardStage::XValue;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            (
                Pending::ChooseNumber {
                    player: p,
                    min,
                    max,
                },
                PlayerAction::ChooseNumber(n),
            ) if *p == player => {
                // The choice contract: the answer must stay inside the
                // offered range — an unchecked X overflows costs, life
                // payments, and token counts downstream.
                if n < *min || n > *max {
                    return Err(EngineError::IllegalAction(
                        "number outside the offered range",
                    ));
                }
                // Two things ask for a number, and only one of them is the
                // cast wizard. An activation announcing the X of a counter
                // cost (CR 601.2b) has no wizard at all, so the plan is what
                // tells them apart and it has to be read *before* the
                // `expect` below — which is the whole reason the branch is
                // here rather than after it.
                if let Some(PlanKind::ChooseActivationX {
                    source,
                    ability_index,
                }) = self.pending_plan.take()
                {
                    self.activation_x = Some(n);
                    return self.start_activation(player, source, ability_index, SmallVec::new());
                }
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                wizard.x = n;
                wizard.stage = cast_wizard::WizardStage::Targets;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            (Pending::ChoosePlayer { player: p, options }, PlayerAction::ChoosePlayer(chosen))
                if *p == player =>
            {
                if !options.contains(&chosen) {
                    return Err(EngineError::IllegalAction("player not among the options"));
                }
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(
                        r.awaiting,
                        Some(resolve::AwaitingOp::ControlRotation { .. })
                    )
                }) {
                    let mut res = self.resolution.take().expect("rotation suspended");
                    match resolve::resume_control_rotation(&mut self.state, &mut res, chosen) {
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => self.finish_resolution(&res),
                    }
                    return Ok(());
                }
                // Loyalty ability target player.
                if let Some(PlanKind::LoyaltyPlayer {
                    source,
                    ability_index,
                }) = self.pending_plan.take()
                {
                    self.loyalty_player_choice = Some(chosen);
                    self.finish_loyalty_activation(player, source, ability_index, SmallVec::new());
                    return Ok(());
                }
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                wizard.chosen_player = Some(chosen);
                wizard.stage = cast_wizard::WizardStage::SecondTargets;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            (
                Pending::Priority { player: p, legal },
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
            ) if *p == player => {
                if !legal.abilities.contains(&(source, ability_index)) {
                    return Err(EngineError::IllegalAction("ability not activatable"));
                }
                // A fresh press starts with nothing answered. The field is
                // accumulated across several `apply` calls, so the one place
                // it can be cleared without losing an answer is the moment a
                // new activation begins — an activation refused halfway
                // through its questions would otherwise hand what it had
                // collected to the next one.
                self.activation_cost_choices.clear();
                self.activation_x = None;
                self.activation_second_targets = None;
                self.activation_targets_answered = false;
                self.start_activation(player, source, ability_index, SmallVec::new())
            }
            (Pending::Priority { player: p, legal }, PlayerAction::Suspend { card })
                if *p == player =>
            {
                if !legal.suspendable.contains(&card) {
                    return Err(EngineError::IllegalAction("card cannot be suspended"));
                }
                let (counters, cost) = self
                    .state
                    .object(card)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))
                    .and_then(|def| {
                        def.abilities.iter().find_map(|a| match a {
                            AbilityDef::Suspend { counters, cost } => Some((*counters, *cost)),
                            _ => None,
                        })
                    })
                    .ok_or(EngineError::IllegalAction("not a suspend card"))?;
                // Suspending costs the printed suspend cost (CR 702.62).
                // Through `pay_with`, because the offer asks `can_pay_mana`,
                // which reads Mycosynth Lattice: a bare `mana_pay::pay` here
                // would refuse a suspend that this seat's every mana is
                // allowed to pay for, which is the offer disagreeing with the
                // answer on a second axis.
                let wild = casting::mana_is_wild(&self.state);
                if !casting::pay_with(
                    wild,
                    &mut self.state.players[player.get() as usize].mana_pool,
                    &cost,
                ) {
                    return Err(EngineError::IllegalAction("cannot pay the suspend cost"));
                }
                let owner = self.state.object(card).map_or(player, |o| o.owner);
                {
                    let obj = self.state.object_mut(card).expect("validated");
                    obj.riders.push(crate::object::Rider::Suspend);
                    obj.counters
                        .add(baylee_cards_dsl::CounterKind::Time, u16::from(counters));
                }
                self.state.move_object(
                    card,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                )?;
                self.after_action(player);
                Ok(())
            }
            (
                Pending::ChooseTargets {
                    player: p,
                    options,
                    player_options,
                    min,
                    max,
                    ..
                },
                PlayerAction::ChooseTargets { objects, players },
            ) if *p == player => {
                let total = objects.len() + players.len();
                // The same target can't be chosen twice for one instance of
                // the word "target" (CR 115.3), and one question here is one
                // instance — a second instance is asked on its own. A
                // repeated seat was counted twice and stored once (the spell
                // carries a `SeatSet`), and a repeated object was counted
                // twice and *kept* twice, so a spell taking two targets could
                // be cast naming one.
                //
                // The convoke question arrives as this variant too, and there
                // the refusal rests on a different rule for the same answer:
                // each creature pays for one mana by being tapped (CR
                // 702.51a), and only an untapped permanent can be tapped (CR
                // 701.26a). `convoke_taps.len()` is what reduces the cost, so
                // `[elf, elf]` bought two mana with one tap.
                if total < *min as usize
                    || total > *max as usize
                    || names_one_twice(&players)
                    || names_one_twice(&objects)
                    || !objects.iter().all(|o| options.contains(o))
                    || !players.iter().all(|p| player_options.contains(p))
                {
                    return Err(EngineError::IllegalAction("invalid target selection"));
                }
                // Wizard path: a cast in progress is asking, and *which*
                // question it asked is the stage it is standing in.
                //
                // These were two `if`s in the other order, and the first one
                // asked only whether a wizard was active — so it answered
                // every question the wizard could ask, the convoke branch
                // below it was unreachable, and `convoke_taps` was written by
                // nothing. Answering the convoke question therefore filed the
                // tapped permanents as the spell's *targets* and put the
                // wizard back at `Kicker`, which asked the kicker question
                // again, over an unchanged board, forever: a self-play game
                // stalled on turn 34 casting one Spirit Water Revival.
                if let Some(mut wizard) = self.cast_wizard.take() {
                    if wizard.stage == cast_wizard::WizardStage::Convoke {
                        wizard.convoke_taps = objects.into_iter().collect();
                        wizard.stage = cast_wizard::WizardStage::Done;
                    } else if wizard.stage == cast_wizard::WizardStage::SecondTargets {
                        // The second instance of "target" is objects only in
                        // every shape that prints one, so no seat is kept.
                        wizard.second_targets = objects.into_iter().collect();
                        wizard.stage = cast_wizard::WizardStage::Kicker;
                    } else {
                        wizard.targets = objects.into_iter().collect();
                        wizard.target_players = players.into_iter().collect();
                        wizard.stage = cast_wizard::WizardStage::SecondTargets;
                    }
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                // Resolution path: a resolving effect asked for targets rather
                // than a cast or an activation — redirecting a spell, or
                // pointing a fresh copy somewhere new. There is no plan to
                // consume; the suspended resolution is the continuation, the
                // same way `ChooseCards` answers a search.
                if self.pending_plan.is_none()
                    && let Some(mut res) = self.resolution.take()
                {
                    match resolve::resume(&mut self.state, &mut res, &objects) {
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => {
                            self.finish_resolution(&res);
                        }
                    }
                    return Ok(());
                }
                let plan = self.pending_plan.take().expect("target plan set");
                let targets: SmallVec<[ObjectId; 2]> = objects.into_iter().collect();
                match plan {
                    // Answered with a number and taken off the plan there, so
                    // a target choice can never be carrying one. Named rather
                    // than left to a wildcard, because the next plan that is
                    // answered somewhere else should have to say so here.
                    PlanKind::ChooseActivationX { .. } => {
                        unreachable!("activation-number plans are answered via ChooseNumber")
                    }
                    PlanKind::ActivateAbilitySecondTargets {
                        source,
                        ability_index,
                        targets: first,
                        target_players,
                    } => {
                        self.activation_second_targets = Some(targets.into_iter().collect());
                        self.activation_target_players = target_players;
                        self.start_activation(player, source, ability_index, first)?;
                    }
                    PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    } => {
                        // Loyalty abilities complete via their own finish path
                        // (no guard, no re-payment).
                        if matches!(
                            self.state
                                .object(source)
                                .and_then(|o| {
                                    let face = o.face_index as usize;
                                    o.card
                                        .and_then(|c| self.lookup.card(c.index))
                                        .map(|def| def.abilities_for_face(face))
                                })
                                .and_then(|abilities| abilities.get(ability_index as usize)),
                            Some(AbilityDef::Loyalty { .. })
                        ) {
                            self.finish_loyalty_activation(player, source, ability_index, targets);
                        } else {
                            // Only on this arm. A loyalty ability shares the
                            // plan and finishes elsewhere, so setting the
                            // field for one would leave it standing for
                            // whichever activation came next.
                            self.activation_target_players.clone_from(&players);
                            self.activation_targets_answered = true;
                            self.start_activation(player, source, ability_index, targets)?;
                        }
                    }
                    // Set only beside a `Pending::ChooseCards`, and answered
                    // in that arm. Named rather than swept into a `_` so a
                    // new `PlanKind` is still a compile error here, which is
                    // how this arm came to be written at all.
                    PlanKind::PayActivationCost { .. } => {
                        return Err(EngineError::IllegalAction(
                            "an activation cost is not a target choice",
                        ));
                    }
                    // Same reason, one step further out: the untap step's
                    // determination is not targeting at all (CR 115.1), and
                    // the step it belongs to grants nobody priority to cast
                    // anything that could be.
                    PlanKind::UntapChoice => {
                        return Err(EngineError::IllegalAction(
                            "the untap determination is not a target choice",
                        ));
                    }
                    PlanKind::Trigger {
                        source,
                        ability_index,
                        mode,
                    } => {
                        // Consume the queued trigger before stacking it — and
                        // keep it, because it is carrying the ability list its
                        // index points into when the source has stopped
                        // answering with it (CR 603.10a). This is the third of
                        // the three doors to the stack, and the one where the
                        // trigger has been out of reach across a player's
                        // answer.
                        let queued = self.trigger_queue.pop_front();
                        if self.breaking_loop {
                            // The house rule broke an endless loop: the
                            // trigger feeding it is answered but never
                            // reaches the stack. A trigger whose target was
                            // already chosen has to be dropped here as well
                            // as in `collect_triggers` -- the choice is what
                            // put it beyond that gate.
                            return Ok(());
                        }
                        let controller = self.state.object(source).map_or(player, |o| o.controller);
                        if let Some(t) = queued.as_ref() {
                            self.hand_over_trigger_abilities(t);
                        }
                        self.push_ability_to_stack(controller, source, ability_index, targets);
                        self.set_top_mode(mode);
                        // Player targets ride beside the object ones. The
                        // ability is on the stack now, so the seats are
                        // written onto it directly rather than threaded
                        // through a signature every other caller passes
                        // empty. Both fields are written because they answer
                        // two different questions: `target_players` is the
                        // set that was targeted ("any target" may hold
                        // several), `chosen_player` is the single seat
                        // `PlayerRel::Chosen` reads back, which exists only
                        // when exactly one was named.
                        if !players.is_empty()
                            && let Some(top) =
                                self.state.zones.list(ZoneLocation::Stack).last().copied()
                            && let Some(obj) = self.state.object_mut(top)
                        {
                            obj.target_players = players.iter().copied().collect();
                            if let [only] = players[..] {
                                obj.chosen_player = Some(only);
                            }
                        }
                    }
                    PlanKind::EntryTap { .. } => {
                        unreachable!("entry-tap plans are answered via YesNo")
                    }
                    PlanKind::DelayedPay { .. } | PlanKind::DelayedPaySacrifice { .. } => {
                        unreachable!("delayed-pay plans are answered via YesNo")
                    }
                    PlanKind::LoyaltyPlayer { .. } => {
                        unreachable!("loyalty-player plans are answered via ChoosePlayer")
                    }
                    PlanKind::DrawOffer { .. } => {
                        unreachable!("draw offers are answered via YesNo")
                    }
                    PlanKind::ModalTrigger { .. } => {
                        unreachable!("modal-trigger plans are answered via ChooseMode")
                    }
                    PlanKind::CopyOnEnter {
                        object,
                        before_entry,
                    } => {
                        // The move first, and unconditionally: declining is a
                        // legal answer (`min: 0`) and a permanent that was
                        // not copied still enters. Ordering matters the other
                        // way too — `apply_copy_choice` writes the copied
                        // base onto a permanent, and it has to be one.
                        if before_entry {
                            let _ = self.state.move_object(
                                object,
                                crate::zone::ZoneLocation::Battlefield,
                                crate::zone::ZonePosition::Top,
                                crate::event::Cause::Spell,
                            );
                        }
                        if let Some(&target) = targets.first() {
                            self.apply_copy_choice(object, target);
                        }
                    }
                    PlanKind::EntryReveal { .. } => {
                        unreachable!("entry-reveal plans are answered beside Pending::ChooseCards")
                    }
                    PlanKind::SyntheticTriggerTarget { trigger } => {
                        // No pop, unlike `PlanKind::Trigger` above. The
                        // ordinary targeted path publishes its question and
                        // returns *before* `collect_triggers` reaches the pop
                        // at the end of its loop body; the synthetic one is
                        // asked after it, so this entry is already off the
                        // queue and popping again would take the trigger
                        // behind it.
                        self.push_synthetic_trigger_with_targets(&trigger, targets);
                    }
                    PlanKind::ChooseSubtype { .. } => {
                        unreachable!("subtype plans are answered via ChooseSubtype")
                    }
                    PlanKind::ChooseColor { .. } | PlanKind::IntrinsicMana { .. } => {
                        unreachable!("color plans are answered via ChooseColor")
                    }
                    PlanKind::PlayLandFace { .. } => {
                        unreachable!("land-face plans are answered via ChooseMode")
                    }
                    PlanKind::CommanderZone { .. } => {
                        unreachable!("command-zone plans are answered via YesNo")
                    }
                    PlanKind::Miracle { .. } => {
                        unreachable!("miracle plans are answered via YesNo")
                    }
                }
                Ok(())
            }
            (
                Pending::ChooseSubtype { player: p, options },
                PlayerAction::ChooseSubtype(subtype),
            ) if *p == player => {
                if !options.contains(&subtype) {
                    return Err(EngineError::IllegalAction("not a creature type"));
                }
                let Some(PlanKind::ChooseSubtype { object }) = self.pending_plan.take() else {
                    return Err(EngineError::IllegalAction("no subtype choice pending"));
                };
                if let Some(obj) = self.state.object_mut(object) {
                    obj.chosen_subtype = Some(subtype);
                    // "This creature is the chosen type in addition to its
                    // other types" (Roaming Throne) — creatures gain the
                    // chosen subtype in their base characteristics.
                    if obj
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                    {
                        obj.base_mut().subtypes.insert(subtype);
                    }
                }
                // The choice is an input to the *layer system*, not only to
                // this object: `Filter::MatchesChosenTypeOfSource` reads it,
                // so Steely Resolve's "creatures of the chosen type have
                // shroud" matches a different set of permanents the instant
                // the type is named. The generation compare that guards the
                // refresh tracks the effect **table**, which did not change
                // here — the static was registered when the enchantment
                // entered, one question earlier — so without this every
                // creature on the board keeps the projection it had before
                // anybody chose, and the card does nothing at all.
                self.state.invalidate_projections();
                Ok(())
            }
            // Two questions wear one `Pending`. This arm is the entering
            // permanent's — "as this enters, choose a color" — and it is
            // guarded on the plan rather than ordered by luck: the arm below
            // takes the suspended `Resolution`, and there is none while a
            // permanent is entering.
            (Pending::ChooseColor { player: p, options }, PlayerAction::ChooseColor(color))
                if *p == player
                    && matches!(self.pending_plan, Some(PlanKind::IntrinsicMana { .. })) =>
            {
                if !options.contains(&color) {
                    return Err(EngineError::IllegalAction("color not allowed"));
                }
                let Some(PlanKind::IntrinsicMana { source }) = self.pending_plan.take() else {
                    unreachable!("guarded above");
                };
                // The land was tapped when the question was published, so
                // what is left is the mana — through the same door the
                // one-type land goes through.
                casting::add_intrinsic_mana(&mut self.state, player, source, color);
                self.after_action(player);
                Ok(())
            }
            (Pending::ChooseColor { player: p, options }, PlayerAction::ChooseColor(color))
                if *p == player
                    && matches!(self.pending_plan, Some(PlanKind::ChooseColor { .. })) =>
            {
                if !options.contains(&color) {
                    return Err(EngineError::IllegalAction("color not allowed"));
                }
                let Some(PlanKind::ChooseColor { object }) = self.pending_plan.take() else {
                    unreachable!("guarded above");
                };
                if let Some(obj) = self.state.object_mut(object) {
                    obj.chosen_color = Some(color);
                }
                Ok(())
            }
            (Pending::ChooseColor { player: p, options }, PlayerAction::ChooseColor(color))
                if *p == player =>
            {
                if !options.contains(&color) {
                    return Err(EngineError::IllegalAction("color not allowed"));
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_with_color(&mut self.state, &mut res, color) {
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => {
                        self.finish_resolution(&res);
                    }
                }
                Ok(())
            }
            (
                Pending::Arrange {
                    player: p,
                    cards,
                    piles: specs,
                    ..
                },
                PlayerAction::Arrange { piles },
            ) if *p == player => {
                // Every offered card exactly once, each pile within its
                // bounds — anything else would duplicate or vanish cards.
                if let Some(fault) = crate::choice::arrangement_fault(cards, specs, &piles) {
                    return Err(EngineError::IllegalAction(fault));
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_arranged(&mut self.state, &mut res, &piles) {
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => {
                        self.finish_resolution(&res);
                    }
                }
                Ok(())
            }
            (Pending::YesNo { player: p, .. }, PlayerAction::YesNo(answer)) if *p == player => {
                // A draw offer: unanimous or nothing (CR 104.4i).
                if matches!(self.pending_plan, Some(PlanKind::DrawOffer { .. })) {
                    let Some(PlanKind::DrawOffer {
                        proposer,
                        mut remaining,
                        resume,
                    }) = self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    if !answer {
                        // Refused: hand back the decision the offer interrupted.
                        self.pending = *resume;
                        self.awaiting_answer = true;
                        return Ok(());
                    }
                    if remaining.is_empty() {
                        self.agreed_draw = true;
                        self.awaiting_answer = false;
                        self.run_until_choice();
                        return Ok(());
                    }
                    let next = remaining.remove(0);
                    self.pending_plan = Some(PlanKind::DrawOffer {
                        proposer,
                        remaining,
                        resume,
                    });
                    self.pending = Pending::YesNo {
                        player: next,
                        prompt: crate::choice::YesNoPrompt::DrawOffer { proposer },
                        source: None,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                // Delayed pay-or-lose (Pact of Negation).
                if matches!(self.pending_plan, Some(PlanKind::DelayedPay { .. })) {
                    let Some(PlanKind::DelayedPay { cost }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    // `pay` mutates the pool — never hide the call behind
                    // `debug_assert!`, which is not evaluated in release.
                    let paid = answer
                        && mana_pay::pay(
                            &mut self.state.players[player.get() as usize].mana_pool,
                            &cost,
                        );
                    debug_assert!(!answer || paid, "pact cost was offered as payable");
                    if !paid {
                        sba::eliminate_player(&mut self.state, player, LossReason::Effect);
                    }
                    return Ok(());
                }
                // Echo: pay the echo cost or sacrifice the permanent.
                if matches!(
                    self.pending_plan,
                    Some(PlanKind::DelayedPaySacrifice { .. })
                ) {
                    let Some(PlanKind::DelayedPaySacrifice { cost, card }) =
                        self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    let paid = answer
                        && mana_pay::pay(
                            &mut self.state.players[player.get() as usize].mana_pool,
                            &cost,
                        );
                    debug_assert!(!answer || paid, "echo cost was offered as payable");
                    if !paid {
                        let owner = self.state.object(card).map_or(player, |o| o.owner);
                        if let Some(obj) = self.state.object_mut(card) {
                            obj.kind = ObjectKind::Card;
                        }
                        let _ = self.state.move_object(
                            card,
                            ZoneLocation::Graveyard(owner),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                    return Ok(());
                }
                // An optional clause inside a resolving ability ("you may
                // gain life equal to …").
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(r.awaiting, Some(crate::resolve::AwaitingOp::MayDo { .. }))
                }) {
                    let mut res = self.resolution.take().expect("resolution suspended");
                    match resolve::resume_may_do(&mut self.state, &mut res, answer) {
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => {
                            self.finish_resolution(&res);
                        }
                    }
                    return Ok(());
                }
                // Tax choice (Rhystic Study & co.).
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(
                        r.awaiting,
                        Some(crate::resolve::AwaitingOp::PlayerMayPay { .. })
                    )
                }) {
                    // CR 605.3a: a player who says they will pay may make
                    // the mana now. If the pool already covers it there is
                    // nothing to open — and if it does not, a window is only
                    // worth opening when there is something in it to press,
                    // because a player with no untapped source would
                    // otherwise be handed a question whose only answer is
                    // the one they just gave.
                    let asked = self.resolution.as_ref().and_then(|r| match r.awaiting {
                        Some(crate::resolve::AwaitingOp::PlayerMayPay { player, mana, .. }) => {
                            Some((player, mana))
                        }
                        _ => None,
                    });
                    if let Some((payer, mana)) = asked
                        && answer
                        && payer == player
                    {
                        let pool = self.state.players[player.get() as usize].mana_pool.total();
                        if pool < u32::from(mana) {
                            // Narrowed before any window exists, so the
                            // resolution is never lifted out of its slot for a
                            // window that then turns out not to be worth
                            // opening — a state that cannot be entered needs
                            // no way back out of it.
                            let mut legal = self.compute_legal(player);
                            self.narrow_to_mana(&mut legal);
                            if legal.has_mana_source() {
                                let suspended = self
                                    .resolution
                                    .take()
                                    .expect("the arm above matched on it being suspended");
                                self.mana_window = Some(PaymentWindow {
                                    player,
                                    suspended: Box::new(suspended),
                                });
                                self.pending = Pending::Priority {
                                    player,
                                    legal: Box::new(legal),
                                };
                                self.awaiting_answer = true;
                                return Ok(());
                            }
                            // Nothing to press: no window is opened at all, and
                            // the payment fails the way it always has.
                        }
                    }
                    let mut res = self.resolution.take().expect("resolution suspended");
                    let answer = answer && self.can_settle_tax(&res);
                    match resolve::resume_tax_choice(&mut self.state, &mut res, answer) {
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => {
                            self.finish_resolution(&res);
                        }
                    }
                    return Ok(());
                }
                // A commander offered its way home (CR 903.9a). Saying no
                // is a real answer — the card stays where it is, and the
                // pass that asked has already recorded that it did, so the
                // question does not come round again for this arrival.
                if matches!(self.pending_plan, Some(PlanKind::CommanderZone { .. })) {
                    let Some(PlanKind::CommanderZone { card }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    if answer {
                        // The *owner's* command zone (CR 903.9a): a
                        // commander stolen and then killed goes home to
                        // whoever brought it, not to whoever took it.
                        let owner = self.state.object(card).map_or(player, |o| o.owner);
                        if let Some(obj) = self.state.object_mut(card) {
                            obj.kind = ObjectKind::Card;
                        }
                        self.state.move_object(
                            card,
                            ZoneLocation::Command(owner),
                            ZonePosition::Top,
                            Cause::StateBased,
                        )?;
                    }
                    return Ok(());
                }
                // Miracle offer: yes starts the miracle cast wizard.
                if matches!(self.pending_plan, Some(PlanKind::Miracle { .. })) {
                    let Some(PlanKind::Miracle { card }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    if answer {
                        return self.start_miracle_cast(player, card);
                    }
                    return Ok(());
                }
                // Shockland entry choice: pay life or enter tapped.
                if matches!(self.pending_plan, Some(PlanKind::EntryTap { .. })) {
                    let Some(PlanKind::EntryTap { object, amount }) = self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    if answer {
                        let p_ref = &mut self.state.players[player.get() as usize];
                        let old = p_ref.life;
                        p_ref.life -= i32::from(amount);
                        let new = p_ref.life;
                        self.state.journal.record(GameEvent::LifeChanged {
                            player,
                            old,
                            new,
                            cause: Cause::Cost,
                        });
                    } else {
                        self.state.set_tapped(object, true);
                    }
                    return Ok(());
                }
                // Wizard path: kicker yes/no.
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::Kicker)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.kicked = answer;
                    wizard.stage = cast_wizard::WizardStage::PitchChoice;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_yes_no(&mut self.state, &mut res, answer) {
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => {
                        self.finish_resolution(&res);
                    }
                }
                Ok(())
            }
            (
                Pending::ChooseCards {
                    player: p,
                    options,
                    min,
                    max,
                    ..
                },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                // Every option is a distinct object, so a repeat is never a
                // second choice — it is one card counted twice. Delve reads
                // `delve_exiles.len()` and exiles each card once, so `[c, c]`
                // bought two generic mana with one card (CR 702.66a); a cost
                // question reads its answers the same way.
                if objects.len() < *min as usize
                    || objects.len() > *max as usize
                    || names_one_twice(&objects)
                    || !objects.iter().all(|o| options.contains(o))
                {
                    return Err(EngineError::IllegalAction("invalid card selection"));
                }
                // Wizard path: pitch cards (exile-from-hand costs).
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::PitchChoice)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.pitch = objects.into_iter().collect();
                    wizard.stage = cast_wizard::WizardStage::Delve;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                // Wizard path: delve cards (exile-from-graveyard, {1} each).
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::Delve)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.delve_exiles = objects.into_iter().collect();
                    wizard.stage = cast_wizard::WizardStage::Convoke;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                // Activation-cost path: the answer to "sacrifice a creature"
                // or "discard a card". It goes back into the same
                // `start_activation` the target answer re-enters, one
                // question further down CR 601.2's checklist, carrying the
                // halves that function takes out of the engine on entry.
                match self.pending_plan.take() {
                    Some(PlanKind::PayActivationCost {
                        source,
                        ability_index,
                        targets,
                        target_players,
                    }) => {
                        self.activation_cost_choices.extend(objects);
                        self.activation_target_players = target_players;
                        return self.start_activation(player, source, ability_index, targets);
                    }
                    // The untap step's determination (CR 502.3). The answer
                    // names what stays tapped, and the step carries on from
                    // "then they untap them all simultaneously" — never
                    // from the top, where phasing and the day/night check
                    // have already happened.
                    Some(PlanKind::UntapChoice) => {
                        self.finish_untap_step(&objects);
                        return Ok(());
                    }
                    // A reveal land's entry clause. CR 701.20a shows the
                    // card to every player and CR 701.20b leaves it in hand,
                    // so nothing moves: the journal entry *is* the reveal,
                    // and without it the card would have been shown to
                    // nobody while the land still came down untapped.
                    //
                    // Naming nothing is how "you may" is declined (`min: 0`),
                    // and that is the branch the printed "if you don't"
                    // charges for.
                    Some(PlanKind::EntryReveal { object }) => {
                        if objects.is_empty() {
                            self.state.set_tapped(object, true);
                        } else {
                            self.state.journal.record(GameEvent::Revealed {
                                player,
                                cards: objects,
                            });
                        }
                        return Ok(());
                    }
                    other => self.pending_plan = other,
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume(&mut self.state, &mut res, &objects) {
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => {
                        self.finish_resolution(&res);
                    }
                }
                Ok(())
            }
            (
                Pending::Priority { player: p, legal },
                PlayerAction::ActivateManaAbility { source },
            ) if *p == player => {
                if !legal.mana_abilities.contains(&source) {
                    return Err(EngineError::IllegalAction("mana ability not activatable"));
                }
                // `mana_abilities` has two producers, and only one of them is
                // the CR 305.6 shortcut this action was written for. The other
                // is a mana ability a continuous effect *granted*, which lives
                // in `legal.abilities` at the synthetic index and is activated
                // like any other ability.
                //
                // Both were offered here and only the first could be taken:
                // `activate_mana` asks `intrinsic_mana`, a granted ability has
                // none, and the answer came back "illegal action for your
                // seat" — for a source the engine had just listed. Every
                // caller reads the list the same way (`HeuristicAgent` sends
                // `ActivateManaAbility { source: legal.mana_abilities[0] }`
                // outright), so the list has to mean one thing: everything in
                // it is activatable by this action.
                //
                // Intrinsic first, because a land with a granted ability on
                // top of its own basic type still taps for its own colour
                // unless the player names the other ability by index.
                if casting::can_activate_mana(&self.state, player, source) {
                    // CR 305.6 gives the land one mana ability per basic
                    // type, so a land with several is a question. It is
                    // asked *here* and not inside `activate_mana`, which is
                    // a state function with no way to publish a pending —
                    // and it is asked after the tap, because the tap is the
                    // cost and the colour is the effect, exactly as a
                    // printed "add one mana of any color" pays first and
                    // asks second.
                    let colors = casting::intrinsic_mana_offer(&self.state, &self.lookup, source);
                    if colors.len() > 1 {
                        self.state.set_tapped(source, true);
                        self.state.journal.record(GameEvent::ObjectTapped {
                            object: source,
                            cause: Cause::Cost,
                        });
                        self.pending_plan = Some(PlanKind::IntrinsicMana { source });
                        self.pending = Pending::ChooseColor {
                            player,
                            options: colors,
                        };
                        self.awaiting_answer = true;
                        return Ok(());
                    }
                    let [only] = colors.as_slice() else {
                        return Err(EngineError::IllegalAction("mana ability not activatable"));
                    };
                    casting::add_intrinsic_mana(&mut self.state, player, source, *only);
                    self.after_action(player);
                    return Ok(());
                }
                self.start_activation(
                    player,
                    source,
                    crate::choice::GRANTED_ABILITY,
                    SmallVec::new(),
                )
            }
            (
                Pending::ChooseAttackers { player: p, .. },
                PlayerAction::DeclareAttackers { attackers },
            ) if *p == player => self.declare_attackers(player, attackers),
            (
                Pending::ChooseBlockers { player: p, .. },
                PlayerAction::DeclareBlockers { blockers },
            ) if *p == player => self.declare_blockers(player, blockers),
            (
                Pending::DiscardChoice { player: p, count },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                // The same hole as the mulligan's: one card named twice
                // passes the count and leaves the hand over its maximum
                // (CR 514.1).
                if objects.len() != *count as usize || names_one_twice(&objects) {
                    return Err(EngineError::IllegalAction(
                        "must discard exactly the required number",
                    ));
                }
                for card in &objects {
                    if !self.in_hand(player, *card) {
                        return Err(EngineError::IllegalAction("card not in hand"));
                    }
                }
                for card in objects {
                    self.state.journal.record(GameEvent::Discarded {
                        object: card,
                        player,
                    });
                    self.state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::TurnBased,
                    )?;
                }
                self.end_cleanup();
                Ok(())
            }
            (
                Pending::LegendChoice { player: p, options },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != 1 || !options.contains(&objects[0]) {
                    return Err(EngineError::IllegalAction(
                        "choose exactly one legendary permanent to keep",
                    ));
                }
                let options = options.clone();
                sba::apply_legend_choice(&mut self.state, player, objects[0], &options);
                Ok(())
            }
            _ => Err(EngineError::MismatchedAction),
        }
    }

    pub(crate) fn in_hand(&self, player: PlayerId, card: ObjectId) -> bool {
        self.state
            .object(card)
            .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player))
    }

    /// Whether the pool now covers the payment the suspended resolution is
    /// asking for.
    ///
    /// Asked before `resume_tax_choice` rather than left to it, because that
    /// function asserts what it was told: it takes `paid` as a promise the
    /// caller has already checked, and answering "yes" for a player who
    /// cannot actually pay trips its `debug_assert!` — in debug only, which
    /// is the shape of bug this workspace runs its test suite in release to
    /// catch.
    fn can_settle_tax(&self, res: &crate::resolve::Resolution) -> bool {
        match res.awaiting {
            Some(crate::resolve::AwaitingOp::PlayerMayPay { player, mana, .. }) => {
                self.state.players[player.get() as usize].mana_pool.total() >= u32::from(mana)
            }
            _ => false,
        }
    }

    /// Ends a payment window and settles the payment it was opened for.
    ///
    /// The player is taken at their word only as far as their pool goes: a
    /// window they leave short pays nothing and takes the effect's other
    /// branch, which is the same outcome as declining and is what the card
    /// prints.
    fn close_mana_window(&mut self) {
        // Total rather than asserted. The only caller is the pass arm, which
        // has already matched on this window standing open, and a window with
        // no resolution in it is now unrepresentable — so there is nothing
        // here left to be wrong about. The `expect` this replaces was not
        // decoration: it is what caught #167, where a mana ability that asked
        // a colour had taken the slot this resolution was waiting in.
        let Some(window) = self.mana_window.take() else {
            return;
        };
        let mut res = *window.suspended;
        let paid = self.can_settle_tax(&res);
        match resolve::resume_tax_choice(&mut self.state, &mut res, paid) {
            resolve::Flow::Wait(pending) => {
                self.resolution = Some(res);
                self.pending = pending;
                self.awaiting_answer = true;
            }
            resolve::Flow::Complete => {
                self.finish_resolution(&res);
            }
        }
    }

    pub(crate) fn after_action(&mut self, player: PlayerId) {
        // After any non-pass action, priority returns to the acting player
        // (CR 117.3c) and the pass counter resets.
        //
        // It is *recorded* and not published, because CR 117.3c is not the
        // only rule that owes this player something. Before anybody receives
        // priority the game performs its state-based actions (CR 117.5) and
        // puts the abilities that have triggered on the stack (CR 603.3b),
        // and the continuous effects the action created have to be in force
        // for both. All of that is the machine's work and none of it had a
        // chance to happen: building the question here set `awaiting_answer`,
        // which is the first line `run_machine` returns on, so the machine
        // stayed out until the *next* action arrived. `priority_round` picks
        // this up at step 5 instead, which is where every other priority in
        // the game is handed over.
        self.passes = 0;
        self.priority_holder = Some(player);
        self.regrant_priority = Some(player);
    }

    pub(crate) fn mulligan_bottom_count(&self, taken: u8) -> u8 {
        taken.saturating_sub(u8::from(self.house_rules.mulligan_free_first))
    }

    pub(crate) fn advance_mulligan(&mut self) {
        self.mulligan_player += 1;
        if self.mulligan_player >= self.state.players.len() {
            self.begin_turn(true);
        } else {
            let player = PlayerId::new(self.mulligan_player as u8);
            self.pending = Pending::Mulligan {
                player,
                taken: 0,
                next_is_free: self.house_rules.mulligan_free_first,
            };
            self.awaiting_answer = true;
        }
    }

    /// The automatic progression machine: SBAs, stack resolution, and
    /// step/turn transitions until a decision is required.
    pub(crate) fn declare_attackers(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, baylee_core::ids::Defender)>,
    ) -> Result<(), EngineError> {
        // Checked against the same list the request offered, so a client
        // can never name a defender the engine did not put on the table:
        // an opponent's planeswalker that has since left, a player who has
        // lost, or the attacking player themself.
        let legal = combat::defender_options(&self.state, player);
        let mut seen = Vec::with_capacity(attackers.len());
        for (creature, defending) in &attackers {
            if !combat::can_attack(&self.state, player, *creature) {
                return Err(EngineError::IllegalAction("creature cannot attack"));
            }
            if !legal.contains(defending) {
                return Err(EngineError::IllegalAction("invalid defender"));
            }
            if seen.contains(creature) {
                return Err(EngineError::IllegalAction("duplicate attacker"));
            }
            seen.push(*creature);
        }
        for (creature, defending) in attackers {
            let vigilance = self.state.object(creature).is_some_and(|o| {
                o.characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::VIGILANCE)
            });
            if !vigilance {
                self.state.set_tapped(creature, true);
                self.state.journal.record(GameEvent::ObjectTapped {
                    object: creature,
                    cause: Cause::TurnBased,
                });
            }
            self.state.combat.attackers.push(AttackerInfo {
                creature,
                defending,
                blocked: false,
            });
            self.state.journal.record(GameEvent::BecameAttacker {
                object: creature,
                defending,
            });
        }
        // `Filter::Attacking` is read by the layer system (Orcish Oriflamme's
        // "attacking creatures you control get +1/+0"), and which permanents
        // match it just changed without the effect table moving — the same
        // shape as a tap, and the same door.
        self.state.board_state_changed();
        self.combat_declared = CombatDeclared::Attackers;
        self.passes = 0;
        self.priority_holder = None;
        Ok(())
    }

    pub(crate) fn declare_blockers(
        &mut self,
        defending: PlayerId,
        blockers: Vec<(ObjectId, ObjectId)>,
    ) -> Result<(), EngineError> {
        let mut seen = Vec::with_capacity(blockers.len());
        for (blocker, attacker) in &blockers {
            if !self
                .state
                .combat
                .attackers
                .iter()
                .any(|a| a.creature == *attacker)
            {
                return Err(EngineError::IllegalAction("no such attacker"));
            }
            if !combat::can_block(&self.state, defending, *blocker, *attacker) {
                return Err(EngineError::IllegalAction("creature cannot block"));
            }
            if seen.contains(blocker) {
                return Err(EngineError::IllegalAction("duplicate blocker"));
            }
            seen.push(*blocker);
        }
        // Menace: needs two blockers per attacker (CR 702.111b), checked
        // against the whole declaration because that is where CR 509.1b puts
        // it — and because it cannot be checked anywhere else. This loop was
        // written with the rest of the rule and was unreachable until #156:
        // `combat::can_block` asked `blockers_of(attacker)` in the per-pair
        // loop above, which runs to completion before the first
        // `declare_block`, so every menace pair was refused one line earlier
        // and no declaration ever arrived here with a count to take. It is
        // now the only place menace is enforced.
        //
        // Zero is legal and one is not: "can't be blocked except by two or
        // more creatures" says nothing about a creature nobody blocks.
        for attacker in &self.state.combat.attackers {
            let has_menace = self.state.object(attacker.creature).is_some_and(|o| {
                o.characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::MENACE)
            });
            if has_menace {
                let count = blockers
                    .iter()
                    .filter(|(_, a)| *a == attacker.creature)
                    .count();
                if count == 1 {
                    return Err(EngineError::IllegalAction("menace requires two blockers"));
                }
            }
        }
        for (blocker, attacker) in blockers {
            self.state.combat.declare_block(blocker, attacker);
            self.state.journal.record(GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            });
        }
        self.combat_declared = CombatDeclared::Blockers;
        self.passes = 0;
        self.priority_holder = None;
        Ok(())
    }

    // --------------------------------------------------------- turn steps
}

/// Whether an answer names one thing twice.
///
/// Every door that takes a list asks it before anything moves, so a refused
/// answer leaves the state untouched. Quadratic, because an answer is a
/// handful of ids.
fn names_one_twice<T: PartialEq>(xs: &[T]) -> bool {
    xs.iter().enumerate().any(|(at, x)| xs[..at].contains(x))
}
