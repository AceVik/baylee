use super::{
    AbilityDef, AttackerInfo, CardLookup, Cause, CombatDeclared, Engine, EngineError, GameEvent,
    ObjectId, ObjectKind, PaymentContinuation, PaymentWindow, Pending, PlanKind, PlayerAction,
    PlayerId, SmallVec, WindowStart, Zone, ZoneLocation, ZonePosition, cast_wizard, casting,
    combat, resolve, sba,
};
use crate::choice::CastModeKind;

mod declarations;

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
        // Who answers comes first, and a bystander is refused in one way
        // whatever it said. The question's constraints are about what it
        // holds, and a search's options are cards in the searcher's hidden
        // library: checked first, they told a bystander which of its
        // guesses were among them (a card not offered was refused as not
        // offered, one offered as a mismatch, by the seat guard below).
        // Every arm below guards on the seat `asked` names, so this refuses
        // nothing the arms took.
        if self.pending.asked() != Some(player) {
            return Err(EngineError::MismatchedAction);
        }
        // Taking back a cast answers none of the cast's questions and is
        // legal in every one of them (`Engine::cancellable_cast`).
        if matches!(action, PlayerAction::CancelCast) {
            return self.cancel_cast(player);
        }
        // Then the question's own constraints, all of them, before anything
        // else reads the answer: an answer is refused for a reason its
        // question states (`Pending::answer_fault`) and taken otherwise. The
        // checks below this line are the engine's and never the answer's: a
        // continuation that cannot go on is reversed and the answer still
        // taken (`reverse_activation`, `continue_cast_wizard`).
        if let Some(fault) = self.pending.answer_fault(&action) {
            return Err(fault.into());
        }
        if let PlayerAction::ChooseObjects { objects } = &action
            && let Some(res) = &self.resolution
            && !resolve::tap_group::valid_answer(&self.state, res, objects)
        {
            return Err(EngineError::IllegalAction(
                "tap payment must name the complete current untapped set",
            ));
        }
        if matches!(
            self.pending,
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::UseRegenerationShield { .. }
                    | crate::choice::YesNoPrompt::UseShieldCounter { .. }
                    | crate::choice::YesNoPrompt::UseDamageClearingReplacement { .. },
                ..
            }
        ) || (crate::replacement::counter_order::has_pending(&self.state)
            && matches!(
                self.pending,
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::ApplyAdditionalCountersNext { .. },
                    ..
                }
            ))
        {
            let PlayerAction::YesNo(use_shield) = action else {
                unreachable!("validated yes/no")
            };
            match self.pending {
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::UseDamageClearingReplacement { .. },
                    ..
                } => crate::sba::answer_damage_clearing_choice(&mut self.state, use_shield),
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::UseShieldCounter { .. },
                    ..
                } => crate::sba::answer_shield_choice(&mut self.state, use_shield),
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::ApplyAdditionalCountersNext { .. },
                    ..
                } => crate::replacement::counter_order::answer(&mut self.state, use_shield),
                _ => crate::sba::answer_regeneration_choice(&mut self.state, use_shield),
            }
            self.awaiting_answer = false;
            if self.activation_payment.is_some() {
                if !self.offer_counter_order() {
                    self.resume_activation_payment()?;
                }
                return Ok(());
            }
            if let Some(mut res) = self.resolution.take() {
                res.awaiting = None;
                match resolve::run(&mut self.state, &mut res) {
                    resolve::Flow::Complete => self.finish_resolution(&res),
                    resolve::Flow::Enter => self.resolution = Some(res),
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                }
            }
            return Ok(());
        }
        if self.state.entries.waiting() {
            crate::entry::answer(&mut self.state, &action);
            self.continue_entry_queue();
            return Ok(());
        }
        if let PlayerAction::ChooseManaAbility {
            source,
            ability_index,
            ..
        } = action
        {
            return self.answer_land_mana(player, source, ability_index);
        }
        if let PlayerAction::ChooseDamageSource { source, .. } = action {
            let mut res = self.resolution.take().expect("source resolution suspended");
            match resolve::resume_source(&mut self.state, &mut res, Some(source)) {
                resolve::Flow::Enter => {
                    self.resolution = Some(res);
                    self.awaiting_answer = false;
                }
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
                resolve::Flow::Complete => self.finish_resolution(&res),
            }
            return Ok(());
        }
        if matches!(
            self.pending,
            Pending::ChooseDamageEffect { .. } | Pending::AllocatePrevention { .. }
        ) || (matches!(self.pending, Pending::YesNo { .. })
            && (self
                .combat_damage
                .as_ref()
                .is_some_and(crate::damage::DamageWork::is_waiting_yes_no)
                || self.resolution.as_ref().is_some_and(|res| {
                    matches!(
                        &res.awaiting,
                        Some(resolve::AwaitingOp::Damage(work)) if work.is_waiting_yes_no()
                    )
                })))
        {
            if let Some(mut work) = self.combat_damage.take() {
                if let Some(pending) = work.answer(&mut self.state, &action) {
                    self.combat_damage = Some(work);
                    self.pending = pending;
                    self.awaiting_answer = true;
                } else {
                    self.finish_combat_damage(&work);
                }
            } else {
                let mut res = self.resolution.take().expect("damage resolution suspended");
                match resolve::resume_damage(&mut self.state, &mut res, &action) {
                    resolve::Flow::Enter => {
                        self.resolution = Some(res);
                        self.awaiting_answer = false;
                    }
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => self.finish_resolution(&res),
                }
            }
            return Ok(());
        }
        match (&self.pending, action) {
            (Pending::Priority { player: p, .. }, PlayerAction::TakeGrantedAction { id })
                if *p == player =>
            {
                self.start_granted_action(player, id)
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
                self.close_mana_window().map(|_| ())
            }
            (Pending::Priority { player: p, .. }, PlayerAction::PassPriority) if *p == player => {
                self.passes += 1;
                Ok(())
            }
            (Pending::Priority { player: p, .. }, PlayerAction::PlayLand { card })
                if *p == player =>
            {
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
                            index: i as u32,
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
            (Pending::Priority { player: p, .. }, PlayerAction::CastSpell { card })
                if *p == player =>
            {
                // A permission that waives the mana cost is the only way this
                // card is cast from where it lies (Dauthi Voidwalker).
                let in_hand = self
                    .state
                    .object(card)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Hand);
                if !in_hand
                    && casting::play_permission(&self.state, player, card).is_some_and(|p| p.free)
                {
                    return self.start_permitted_free_cast(player, card);
                }
                self.start_cast_wizard(player, card)
            }
            (Pending::ChoosePile { player: p, .. }, PlayerAction::ChooseMode(index))
                if *p == player =>
            {
                let mut res = self.resolution.take().expect("pile choice suspended");
                match resolve::resume_pile(&mut self.state, &mut res, index) {
                    resolve::Flow::Enter => {
                        self.resolution = Some(res);
                        self.awaiting_answer = false;
                    }
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => self.finish_resolution(&res),
                }
                Ok(())
            }
            (Pending::ChooseCastMode { player: p, .. }, PlayerAction::ChooseMode(index))
                if *p == player =>
            {
                if self.resolution.as_ref().is_some_and(|res| {
                    matches!(
                        res.awaiting,
                        Some(
                            resolve::AwaitingOp::Dungeon { .. }
                                | resolve::AwaitingOp::MutatePosition
                                | resolve::AwaitingOp::ChooseEffect { .. }
                                | resolve::AwaitingOp::Colors { .. }
                                | resolve::AwaitingOp::MoveCounter { .. }
                        )
                    )
                }) {
                    let mut res = self.resolution.take().expect("effect choice suspended");
                    match resolve::resume_effect_choice(&mut self.state, &mut res, index) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => self.finish_resolution(&res),
                    }
                    return Ok(());
                }
                // What the option at this position actually names. A modal
                // trigger drops the modes it cannot legally choose
                // (CR 603.3c), so the position answered is not the mode
                // number — and the answer has to be inside the list that was
                // offered, which no arm here used to check.
                //
                // Checked before anything is taken. A position past the end
                // used to be refused after the plan or the cast wizard had
                // been taken out of its slot, and a refusal does not put
                // either back: the question stayed on the table with nothing
                // behind it, and the next answer to it, however legal,
                // panicked on the missing wizard.
                let Some(kind) = (match &self.pending {
                    Pending::ChooseCastMode { options, .. } => options.get(index).map(|o| o.kind),
                    _ => None,
                }) else {
                    return Err(EngineError::IllegalAction("no such cast mode"));
                };
                // `take` once. Two `if let Some(…) = self.pending_plan.take()`
                // in a row is one condition and two takes: the first arm
                // *consumes* the plan whatever it holds, so the second could
                // only ever see `None`. The modal-trigger branch below was
                // unreachable for that reason as well as for the one entry 34
                // names, and one fault was hiding the other.
                let plan = self.pending_plan.take();
                match plan {
                    Some(PlanKind::ModalActivation {
                        source,
                        ability_index,
                    }) => {
                        let CastModeKind::Mode(mode) = kind else {
                            return Err(EngineError::IllegalAction("not an activated mode"));
                        };
                        self.activation_mode = Some(super::abilities::ActivationMode {
                            source: self
                                .state
                                .source_identity(source)
                                .ok_or(EngineError::IllegalAction("activation source left"))?,
                            ability_index,
                            mode: u8::try_from(mode)
                                .map_err(|_| EngineError::IllegalAction("invalid mode"))?,
                        });
                        return self.start_activation(
                            player,
                            source,
                            ability_index,
                            SmallVec::new(),
                        );
                    }
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
                        // Refused with the plan back in its slot, for the
                        // reason the position is checked above.
                        let (CastModeKind::Mode(mode), Some(front)) =
                            (kind, self.trigger_queue.front_mut())
                        else {
                            self.pending_plan = Some(PlanKind::ModalTrigger {
                                source,
                                ability_index,
                            });
                            return Err(EngineError::IllegalAction(
                                "no trigger awaiting this mode",
                            ));
                        };
                        debug_assert!(
                            front.source == source && front.ability_index == ability_index,
                            "the modal plan and the queue's front are the same trigger",
                        );
                        front.chosen_mode = Some(mode as u8);
                        return Ok(());
                    }
                    other => self.pending_plan = other,
                }
                if let Some(wizard) = self.cast_wizard.as_mut()
                    && wizard.stage == cast_wizard::WizardStage::Permission
                {
                    let permission = wizard
                        .permissions
                        .get(index)
                        .copied()
                        .ok_or(EngineError::IllegalAction("no such casting permission"))?;
                    wizard.permission = Some(permission);
                    wizard.stage = cast_wizard::WizardStage::ManaPermission;
                    self.continue_cast_wizard();
                    return Ok(());
                }
                // The wizard's own list is the one the position indexes, and
                // it is read before the wizard leaves its slot.
                let Some(option) = self
                    .cast_wizard
                    .as_ref()
                    .and_then(|w| w.options.get(index).map(|o| o.kind))
                else {
                    return Err(EngineError::IllegalAction("no such cast mode"));
                };
                let Some(mut wizard) = self.cast_wizard.take() else {
                    return Err(EngineError::IllegalAction("no such cast mode"));
                };
                wizard.option = Some(option);
                wizard.kicked = option == crate::choice::CastModeKind::Kicked;
                // CR 601.2b announces the mode and *then* the value of X, in
                // the same step: choosing how to cast the spell does not
                // answer what X is. Going to `Targets` here skipped the
                // question for every spell with more than one way to be cast
                // — Heliod's Intervention was cast for X = 0 with nobody
                // asked, the multi-option twin of the defect `XValue`'s own
                // comment describes. It falls through to `Targets` when the
                // chosen cost has no X.
                wizard.stage = cast_wizard::WizardStage::Permission;
                self.cast_wizard = Some(wizard);
                self.continue_cast_wizard();
                Ok(())
            }
            (Pending::ChooseNumber { player: p, .. }, PlayerAction::ChooseNumber(n))
                if *p == player =>
            {
                if self.resolution.as_ref().is_some_and(|res| {
                    matches!(
                        res.awaiting,
                        Some(
                            resolve::AwaitingOp::ChooseNumber
                                | resolve::AwaitingOp::TapEnergy(_)
                                | resolve::AwaitingOp::Counters { .. }
                                | resolve::AwaitingOp::DamagePayment { .. }
                                | resolve::AwaitingOp::TextReplacement { .. }
                                | resolve::AwaitingOp::RemoveCountersSelfCreateTokens { .. }
                                | resolve::AwaitingOp::ChoosePhaseToSkipThisTurn
                        )
                    )
                }) {
                    let mut res = self.resolution.take().expect("counter choice suspended");
                    match resolve::resume_with_number(&mut self.state, &mut res, n) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => self.finish_resolution(&res),
                    }
                    return Ok(());
                }
                // Inside the offered range, which `answer_fault` checked: an
                // unchecked X overflows costs, life payments, and token
                // counts downstream.
                // Two things ask for a number, and only one of them is the
                // cast wizard. An activation announcing the X of a counter
                // cost (CR 601.2b) has no wizard at all, so the plan is what
                // tells them apart and it has to be read *before* the
                // `expect` below — which is the whole reason the branch is
                // here rather than after it.
                match self.pending_plan.take() {
                    Some(PlanKind::ChooseActivationX {
                        source,
                        ability_index,
                    }) => {
                        self.activation_x = Some(n);
                        if self
                            .start_activation(player, source, ability_index, SmallVec::new())
                            .is_err()
                        {
                            self.reverse_activation(player);
                        }
                        return Ok(());
                    }
                    // One target's share of a division; the next is asked,
                    // or the last takes the rest.
                    Some(PlanKind::DivideDamage {
                        on_stack,
                        targets,
                        mut shares,
                        total,
                    }) => {
                        shares.push(n);
                        self.ask_division(player, on_stack, targets, shares, total);
                        return Ok(());
                    }
                    // One creature's share of a combat damage division that
                    // banding handed to this player (CR 702.22j–k).
                    Some(PlanKind::CombatDamage { owed, shares }) => {
                        self.answer_share(owed, shares, n);
                        return Ok(());
                    }
                    _ => {}
                }
                // Casting numbers are distinguished by the stage: X,
                // replicate payments, and each announced damage share.
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                if wizard.stage == cast_wizard::WizardStage::DivideDamage {
                    wizard.division.push(n);
                } else if wizard.stage == cast_wizard::WizardStage::Replicate {
                    wizard.replicated = u8::try_from(n).unwrap_or(u8::MAX);
                    wizard.stage = cast_wizard::WizardStage::Targets;
                } else {
                    wizard.x = n;
                    wizard.stage = cast_wizard::WizardStage::Kicker;
                    // X is announced before targets are chosen (CR 601.2b,
                    // 601.2c), and a target filter that reads it
                    // (`Filter::CmcExactlyX`) reads it off the card: so the
                    // card carries it from here, not only once it is cast.
                    if let Some(obj) = self.state.object_mut(wizard.card) {
                        obj.x_value = n;
                    }
                }
                self.cast_wizard = Some(wizard);
                self.continue_cast_wizard();
                Ok(())
            }
            (Pending::ChoosePlayer { player: p, .. }, PlayerAction::ChoosePlayer(chosen))
                if *p == player =>
            {
                if self.resolution.as_ref().is_some_and(|res| {
                    matches!(
                        res.awaiting,
                        Some(resolve::AwaitingOp::ChooseCoinRepeatPlayer(_))
                    )
                }) {
                    let mut res = self.resolution.take().expect("coin replacement choice");
                    self.pending =
                        resolve::coin_stakes::choose_player(&self.state, &mut res, chosen);
                    self.resolution = Some(res);
                    self.awaiting_answer = true;
                    return Ok(());
                }
                if let Some(PlanKind::EntryBattleProtector { battle }) = self.pending_plan {
                    let valid = self.state.object(battle.object).is_some_and(|object| {
                        object.version == battle.version
                            && object.zone == Zone::Battlefield
                            && object.controller == player
                            && object
                                .characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::BATTLE)
                            && crate::battle::eligible_protectors(&self.state, object)
                                .contains(&chosen)
                    });
                    if !valid {
                        return Err(EngineError::IllegalAction("invalid entry battle protector"));
                    }
                    crate::battle::set_protector(&mut self.state, battle, chosen);
                    self.pending_plan = None;
                    return Ok(());
                }
                if let Some(PlanKind::BattleProtector { battle }) = self.pending_plan {
                    if !crate::battle::sba::answer(&mut self.state, battle, player, chosen) {
                        return Err(EngineError::IllegalAction("invalid battle protector"));
                    }
                    self.pending_plan = None;
                    return Ok(());
                }
                if let Some(PlanKind::ChooseOpponent { object }) = self.pending_plan {
                    self.pending_plan = None;
                    if let Some(obj) = self.state.object_mut(object) {
                        obj.set_chosen_opponent(Some(chosen));
                    }
                    self.state.invalidate_projections();
                    return Ok(());
                }
                if matches!(
                    self.pending_plan,
                    Some(
                        PlanKind::ActivateAbility { .. }
                            | PlanKind::ActivateAbilitySecondTargets { .. }
                    )
                ) {
                    self.activation_target_choice.opponent = Some(chosen);
                    let (source, ability_index, targets) =
                        match self.pending_plan.take().expect("activation chooser") {
                            PlanKind::ActivateAbility {
                                source,
                                ability_index,
                            } => (source, ability_index, SmallVec::new()),
                            PlanKind::ActivateAbilitySecondTargets {
                                source,
                                ability_index,
                                targets,
                                target_players,
                            } => {
                                self.activation_target_players = target_players;
                                (source, ability_index, targets)
                            }
                            _ => unreachable!("activation chooser plan"),
                        };
                    let controller = self
                        .activation_target_choice
                        .controller
                        .expect("activation controller");
                    if self
                        .start_activation(controller, source, ability_index, targets)
                        .is_err()
                    {
                        self.reverse_activation(controller);
                    }
                    return Ok(());
                }
                // The graveyard an activation's targets come from; the
                // target question follows, narrowed to it.
                if let Some(PlanKind::ChooseActivationGraveyard {
                    source,
                    ability_index,
                }) = self.pending_plan
                {
                    self.pending_plan = None;
                    self.activation_graveyard = Some(chosen);
                    // An offered graveyard is taken; an activation that
                    // cannot go on from it is reversed, as every re-entry of
                    // `start_activation` is (`reverse_activation`).
                    if self
                        .start_activation(player, source, ability_index, SmallVec::new())
                        .is_err()
                    {
                        self.reverse_activation(player);
                    }
                    return Ok(());
                }
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(
                        r.awaiting,
                        Some(
                            resolve::AwaitingOp::ControlRotation { .. }
                                | resolve::AwaitingOp::ControlRecipient { .. }
                        )
                    )
                }) {
                    let mut res = self.resolution.take().expect("rotation suspended");
                    match resolve::resume_control_choice(&mut self.state, &mut res, chosen) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => self.finish_resolution(&res),
                    }
                    return Ok(());
                }
                // Which player a token put onto the battlefield attacking
                // attacks (CR 508.4).
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(r.awaiting, Some(resolve::AwaitingOp::AttackingTokens(_)))
                }) {
                    let mut res = self.resolution.take().expect("attack choice suspended");
                    match resolve::resume_attacking_player(&mut self.state, &mut res, chosen) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => self.finish_resolution(&res),
                    }
                    return Ok(());
                }
                // "Choose a player with the most life or tied" (The Black Gate).
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(r.awaiting, Some(resolve::AwaitingOp::ChoosePlayerFrom))
                }) {
                    let mut res = self.resolution.take().expect("player choice suspended");
                    match resolve::resume_chosen_player(&mut self.state, &mut res, chosen) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => self.finish_resolution(&res),
                    }
                    return Ok(());
                }
                // The opponent who chooses a split search's graveyard cards,
                // or who separates Fact or Fiction's piles.
                if self.resolution.as_ref().is_some_and(|r| {
                    matches!(
                        r.awaiting,
                        Some(
                            resolve::AwaitingOp::PickSplitter { .. }
                                | resolve::AwaitingOp::PickSeparator { .. }
                                | resolve::AwaitingOp::SacrificeOpponent { .. }
                        )
                    )
                }) {
                    let mut res = self.resolution.take().expect("splitter suspended");
                    match resolve::resume_pick_splitter(&mut res, chosen) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
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
                    self.continue_loyalty_activation(
                        player,
                        source,
                        ability_index,
                        SmallVec::new(),
                    );
                    return Ok(());
                }
                let mut wizard = self.cast_wizard.take().expect("wizard active");
                wizard.chosen_player = Some(chosen);
                wizard.stage = cast_wizard::WizardStage::SecondTargets;
                self.cast_wizard = Some(wizard);
                self.continue_cast_wizard();
                Ok(())
            }
            (
                Pending::Priority { player: p, .. },
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
            ) if *p == player => {
                // A fresh press starts with nothing answered. The field is
                // accumulated across several `apply` calls, so the one place
                // it can be cleared without losing an answer is the moment a
                // new activation begins — an activation refused halfway
                // through its questions would otherwise hand what it had
                // collected to the next one.
                self.activation_cost_choices.clear();
                self.activation_x = None;
                self.activation_phyrexian.clear();
                self.activation_second_targets = None;
                self.activation_target_choice = super::targeting::ActivationTargetChoice::default();
                self.activation_mode = None;
                self.activation_target_references = crate::sources::TargetReferences::default();
                self.activation_targets_answered = false;
                self.start_activation(player, source, ability_index, SmallVec::new())
            }
            (Pending::Priority { player: p, .. }, PlayerAction::Foretell { card })
                if *p == player =>
            {
                self.foretell(player, card)
            }
            (Pending::Priority { player: p, .. }, PlayerAction::Suspend { card })
                if *p == player =>
            {
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
                // Through `pay_mana`, because the offer asks `can_pay_mana`,
                // which reads Mycosynth Lattice: a bare `mana_pay::pay` here
                // would refuse a suspend that this seat's every mana is
                // allowed to pay for, which is the offer disagreeing with the
                // answer on a second axis.
                let origin = self
                    .state
                    .source_identity(card)
                    .ok_or(EngineError::IllegalAction("missing cost source"))?;
                if !casting::pay_mana_from(&mut self.state, player, origin, &cost) {
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
                Pending::ChooseTargets { player: p, .. },
                PlayerAction::ChooseTargets { objects, players },
            ) if *p == player => {
                // Checked against the question (`Pending::answer_fault`):
                // the count across objects and seats, membership, and that
                // nothing is named twice. The same target can't be chosen
                // twice for one instance of the word "target" (CR 115.3),
                // and one question here is one instance — a second instance
                // is asked on its own. A repeated seat was counted twice and
                // stored once (the spell carries a `SeatSet`), and a repeated
                // object was counted twice and *kept* twice, so a spell
                // taking two targets could be cast naming one.
                //
                // The convoke question arrives as this variant too, and there
                // the refusal rests on a different rule for the same answer:
                // each creature pays for one mana by being tapped (CR
                // 702.51a), and only an untapped permanent can be tapped (CR
                // 701.26a). `convoke_taps.len()` is what reduces the cost, so
                // `[elf, elf]` bought two mana with one tap.
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
                        wizard.stage = cast_wizard::WizardStage::Sacrifice;
                    } else if wizard.stage == cast_wizard::WizardStage::SecondTargets {
                        wizard.target_references.second = self.state.capture_target_group(&objects);
                        wizard.second_targets = objects.into_iter().collect();
                        wizard.second_target_players = players.into_iter().collect();
                        wizard.stage = cast_wizard::WizardStage::DivideDamage;
                    } else {
                        wizard.target_references.first = self.state.capture_target_group(&objects);
                        wizard.targets = objects.into_iter().collect();
                        wizard.target_players = players.into_iter().collect();
                        wizard.stage = cast_wizard::WizardStage::SecondTargets;
                    }
                    self.cast_wizard = Some(wizard);
                    self.continue_cast_wizard();
                    return Ok(());
                }
                // Resolution path: a resolving effect asked for targets rather
                // than a cast or an activation — redirecting a spell, or
                // pointing a fresh copy somewhere new. There is no plan to
                // consume; the suspended resolution is the continuation, the
                // same way `ChooseCards` answers a search.
                if self.pending_plan.is_none()
                    && let Some(mut res) = self.resolution.take()
                {
                    match resolve::resume_targets(&mut self.state, &mut res, &objects, &players) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
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
                // Every target question is published beside what answers
                // it: a cast in progress, a suspended resolution, or a plan.
                // With none of the three there is nothing for the answer to
                // continue, and refusing it changes nothing.
                let Some(plan) = self.pending_plan.take() else {
                    return Err(EngineError::IllegalAction(
                        "no target choice is waiting for an answer",
                    ));
                };
                let targets: SmallVec<[ObjectId; 2]> = objects.into_iter().collect();
                match plan {
                    // Answered with a number and taken off the plan there, so
                    // a target choice can never be carrying one. Named rather
                    // than left to a wildcard, because the next plan that is
                    // answered somewhere else should have to say so here.
                    PlanKind::ChooseActivationX { .. } => {
                        unreachable!("activation-number plans are answered via ChooseNumber")
                    }
                    PlanKind::ChooseActivationGraveyard { .. } => {
                        unreachable!("a graveyard is answered via ChoosePlayer")
                    }
                    PlanKind::ChoosePhyrexianLife { .. } => {
                        unreachable!("a Phyrexian symbol is answered via YesNo")
                    }
                    PlanKind::ActivateAbilitySecondTargets {
                        source,
                        ability_index,
                        targets: first,
                        target_players,
                    } => {
                        let player = self.activation_target_choice.controller.unwrap_or(player);
                        self.activation_target_references.second =
                            self.state.capture_target_group(&targets);
                        let second = targets.into_iter().collect();
                        if self.loyalty_second_targets(source, ability_index).is_some() {
                            self.finish_loyalty_activation(
                                player,
                                source,
                                ability_index,
                                first,
                                second,
                            );
                        } else {
                            self.activation_second_targets = Some(second);
                            self.activation_target_choice.second_players =
                                players.iter().copied().collect();
                            self.activation_target_players = target_players;
                            if self
                                .start_activation(player, source, ability_index, first)
                                .is_err()
                            {
                                self.reverse_activation(player);
                            }
                        }
                    }
                    PlanKind::ActivateAbility {
                        source,
                        ability_index,
                    } => {
                        let player = self.activation_target_choice.controller.unwrap_or(player);
                        self.activation_target_references.first =
                            self.state.capture_target_group(&targets);
                        // Loyalty abilities complete via their own finish path
                        // (no guard, no re-payment). Asked of the same list
                        // `start_activation` asked when it chose that path,
                        // which for a copied walker is not its card's.
                        if self.loyalty_cost(source, ability_index).is_some() {
                            self.continue_loyalty_activation(
                                player,
                                source,
                                ability_index,
                                targets,
                            );
                        } else {
                            // Only on this arm. A loyalty ability shares the
                            // plan and finishes elsewhere, so setting the
                            // field for one would leave it standing for
                            // whichever activation came next.
                            self.activation_target_players.clone_from(&players);
                            self.activation_targets_answered = true;
                            if self
                                .start_activation(player, source, ability_index, targets)
                                .is_err()
                            {
                                self.reverse_activation(player);
                            }
                        }
                    }
                    // Set only beside a `Pending::ChooseCards`, and answered
                    // in that arm. Named rather than swept into a `_` so a
                    // new `PlanKind` is still a compile error here, which is
                    // how this arm came to be written at all. Refused with
                    // the plan put back, so a refusal leaves the engine as
                    // it found it.
                    PlanKind::PayActivationCost { .. } => {
                        self.pending_plan = Some(plan);
                        return Err(EngineError::IllegalAction(
                            "an activation cost is not a target choice",
                        ));
                    }
                    // Same reason, one step further out: the untap step's
                    // determination is not targeting at all (CR 115.1), and
                    // the step it belongs to grants nobody priority to cast
                    // anything that could be.
                    PlanKind::UntapChoice
                    | PlanKind::UntapLimit { .. }
                    | PlanKind::UntapReplacements(_) => {
                        self.pending_plan = Some(plan);
                        return Err(EngineError::IllegalAction(
                            "the untap determination is not a target choice",
                        ));
                    }
                    PlanKind::Trigger {
                        source,
                        ability_index,
                        mode,
                        per_opponent,
                    } => {
                        // One opponent answered; the next is asked before
                        // anything leaves the queue, and the trigger stacks
                        // with every answer once the last has been given.
                        let targets = match per_opponent {
                            Some(mut asking) => {
                                asking.gathered.extend(targets);
                                let controller =
                                    self.trigger_queue.front().map_or(player, |t| t.controller);
                                let context = self.trigger_queue.front().map_or_else(
                                    || crate::eval::live_context(&self.state, source),
                                    |trigger| crate::text_changes::RuleContext {
                                        source,
                                        text: trigger.text,
                                        resolved_name: None,
                                        reference: None,
                                    },
                                );
                                match self.ask_next_opponent_with_context(
                                    controller,
                                    context,
                                    self.trigger_queue
                                        .front()
                                        .is_some_and(|trigger| self.trigger_wall_only(trigger)),
                                    *asking,
                                    |asking| PlanKind::Trigger {
                                        source,
                                        ability_index,
                                        mode,
                                        per_opponent: Some(asking),
                                    },
                                ) {
                                    None => return Ok(()),
                                    Some(all) => all,
                                }
                            }
                            None => targets,
                        };
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
                        let controller =
                            queued.as_ref().map_or(player, |trigger| trigger.controller);
                        if let Some(t) = queued.as_ref() {
                            self.hand_over_trigger_abilities(t);
                        }
                        let top =
                            self.push_ability_to_stack(controller, source, ability_index, targets);
                        self.set_top_mode(mode);
                        if let Some(t) = &queued {
                            self.bind_top_trigger(t);
                        }
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
                            && let Some(obj) = self.state.object_mut(top)
                        {
                            obj.target_players = players.iter().copied().collect();
                            if let [only] = players[..] {
                                obj.chosen_player = Some(only);
                            }
                        }
                        // A second instance of "target" is asked now that the
                        // first is on the stack object it binds against, and
                        // a division once the targets are known. An empty
                        // optional trigger goes only when its probe resolves
                        // without any effect or follow-up question.
                        if !self.discard_empty_trigger_without_effect(top)
                            && !self.ask_trigger_second_target()
                        {
                            self.ask_trigger_division();
                        }
                    }
                    PlanKind::TriggerSecondTarget { on_stack } => {
                        if let Some(obj) = self.state.object_mut(on_stack) {
                            let req = obj.second_target_req();
                            obj.set_second(targets.into_iter().collect(), req);
                            obj.set_second_players(players.iter().copied().collect());
                        }
                    }
                    PlanKind::EntryCounters { .. } | PlanKind::EntryTap { .. } => {
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
                    PlanKind::ModalTrigger { .. } | PlanKind::ModalActivation { .. } => {
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
                            // A permanent spell's arrival is scanned after
                            // this answer, and the scan puts the copy's
                            // starting loyalty on it. Any other door asked
                            // from inside that scan, after its loyalty step
                            // had read the copier's own values, so the
                            // copied walker's loyalty is put here (CR 306.5b,
                            // CR 614.12).
                            if !before_entry {
                                self.put_starting_loyalty(object);
                            }
                        }
                        if !before_entry {
                            self.initialize_battle_entry(object);
                        }
                    }
                    PlanKind::EntryReveal { .. } => {
                        unreachable!("entry-reveal plans are answered beside Pending::ChooseCards")
                    }
                    PlanKind::SyntheticTriggerTarget {
                        trigger,
                        per_opponent,
                    } => {
                        // One opponent answered; the next is asked, as on the
                        // printed path above.
                        let targets = match per_opponent {
                            Some(mut asking) => {
                                asking.gathered.extend(targets);
                                let plan_t = trigger.clone();
                                match self.ask_next_opponent_with_context(
                                    trigger.controller,
                                    crate::text_changes::RuleContext {
                                        source: trigger.source,
                                        text: trigger.text,
                                        resolved_name: None,
                                        reference: None,
                                    },
                                    self.trigger_wall_only(&trigger),
                                    *asking,
                                    |asking| PlanKind::SyntheticTriggerTarget {
                                        trigger: plan_t,
                                        per_opponent: Some(asking),
                                    },
                                ) {
                                    None => return Ok(()),
                                    Some(all) => all,
                                }
                            }
                            None => targets,
                        };
                        // No pop, unlike `PlanKind::Trigger` above. The
                        // ordinary targeted path publishes its question and
                        // returns *before* `collect_triggers` reaches the pop
                        // at the end of its loop body; the synthetic one is
                        // asked after it, so this entry is already off the
                        // queue and popping again would take the trigger
                        // behind it.
                        self.push_synthetic_trigger_with_targets(&trigger, targets);
                    }
                    PlanKind::ChooseOpponent { .. } => {
                        unreachable!("opponent plans are answered via ChoosePlayer")
                    }
                    PlanKind::BattleProtector { .. } | PlanKind::EntryBattleProtector { .. } => {
                        unreachable!(
                            "mandatory battle protector plans are answered via ChoosePlayer"
                        )
                    }
                    PlanKind::ChooseSubtype { .. } | PlanKind::ChooseTwoBasicLandTypes { .. } => {
                        unreachable!("subtype plans are answered via ChooseSubtype")
                    }
                    PlanKind::ChooseCardName { .. } => {
                        unreachable!("card-name plans are answered via ChooseCardName")
                    }
                    PlanKind::ChooseCardType { .. } => {
                        unreachable!("card-type plans are answered via ChooseCardType")
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
                    PlanKind::Miracle { .. } | PlanKind::Discovered { .. } => {
                        unreachable!("miracle and discover plans are answered via YesNo")
                    }
                    PlanKind::SkipTurn { .. } | PlanKind::SkipDraw { .. } => {
                        unreachable!(
                            "turn skips and draw questions are answered via YesNo or ChooseObjects"
                        )
                    }
                    PlanKind::DivideDamage { .. } => {
                        unreachable!("division plans are answered via ChooseNumber")
                    }
                    // Banding's questions are turn-based actions' own, asked
                    // where nobody holds priority, and neither is targeting
                    // (CR 115.1): refused with the plan put back, as the
                    // untap determination's are.
                    PlanKind::Band { .. }
                    | PlanKind::CombatDamage { .. }
                    | PlanKind::CamouflagePiles { .. } => {
                        self.pending_plan = Some(plan);
                        return Err(EngineError::IllegalAction(
                            "a band, a pile or a damage division is not a target choice",
                        ));
                    }
                }
                Ok(())
            }
            (Pending::ChooseSubtype { player: p, .. }, PlayerAction::ChooseSubtype(subtype))
                if *p == player
                    && self.resolution.as_ref().is_some_and(|res| {
                        matches!(
                            res.awaiting,
                            Some(resolve::AwaitingOp::LandwalkChoice { .. })
                        )
                    }) =>
            {
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_with_subtype(&mut self.state, &mut res, subtype) {
                    resolve::Flow::Enter => {
                        self.resolution = Some(res);
                        self.awaiting_answer = false;
                    }
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                    resolve::Flow::Complete => self.finish_resolution(&res),
                }
                Ok(())
            }
            (Pending::ChooseSubtype { player: p, .. }, PlayerAction::ChooseSubtype(subtype))
                if *p == player =>
            {
                if self.answer_two_land_types(subtype) {
                    return Ok(());
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
            (
                Pending::ChooseCardName {
                    player: p,
                    types,
                    excluded_types,
                },
                PlayerAction::ChooseCardName { card, face },
            ) if *p == player => {
                let printed = self
                    .lookup
                    .card(card)
                    .and_then(|def| def.faces.get(usize::from(face)))
                    .filter(|face| types.is_empty() || face.types.intersects(*types))
                    .filter(|face| !face.types.intersects(*excluded_types));
                let Some((named, name)) =
                    crate::object::PrintedFace::new(card, face).zip(printed.map(|face| face.name))
                else {
                    return Err(EngineError::IllegalAction("not an eligible card name"));
                };
                let resolving_name = self.resolution.as_ref().is_some_and(|res| {
                    matches!(
                        res.awaiting,
                        Some(resolve::AwaitingOp::ChooseCardName { .. })
                    )
                });
                if !resolving_name
                    && !matches!(self.pending_plan, Some(PlanKind::ChooseCardName { .. }))
                {
                    return Err(EngineError::IllegalAction("no card-name choice pending"));
                }
                let name = self.state.names.intern(name);
                if resolving_name {
                    let mut res = self.resolution.take().expect("name choice suspended");
                    match resolve::resume_name(&mut self.state, &mut res, named, name) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
                        resolve::Flow::Wait(pending) => {
                            self.resolution = Some(res);
                            self.pending = pending;
                            self.awaiting_answer = true;
                        }
                        resolve::Flow::Complete => self.finish_resolution(&res),
                    }
                    return Ok(());
                }
                let Some(PlanKind::ChooseCardName { object }) = self.pending_plan.take() else {
                    return Err(EngineError::IllegalAction("no card-name choice pending"));
                };
                if let Some(obj) = self.state.object_mut(object) {
                    obj.remember_name(obj.rules_acquisition(), named, name);
                }
                self.state.invalidate_projections();
                Ok(())
            }
            (
                Pending::ChooseCardType { player: p, options },
                PlayerAction::ChooseCardType(card_type),
            ) if *p == player && options.contains(&card_type) => {
                let Some(PlanKind::ChooseCardType { object }) = self.pending_plan.take() else {
                    return Err(EngineError::IllegalAction("no card-type choice pending"));
                };
                if let Some(obj) = self.state.object_mut(object) {
                    obj.set_chosen_card_type(card_type);
                }
                self.state.invalidate_projections();
                Ok(())
            }
            // Two questions wear one `Pending`. This arm is the entering
            // permanent's — "as this enters, choose a color" — and it is
            // guarded on the plan rather than ordered by luck: the arm below
            // takes the suspended `Resolution`, and there is none while a
            // permanent is entering.
            (Pending::ChooseColor { player: p, .. }, PlayerAction::ChooseColor(color))
                if *p == player
                    && matches!(self.pending_plan, Some(PlanKind::IntrinsicMana { .. })) =>
            {
                let Some(PlanKind::IntrinsicMana { source }) = self.pending_plan.take() else {
                    unreachable!("guarded above");
                };
                // Choosing one of the type-granted abilities precedes its
                // cost. Pay and announce it once, through the one-type door.
                if !casting::pay_intrinsic_mana_price(&mut self.state, player, source) {
                    return Err(EngineError::IllegalAction("cannot pay mana ability cost"));
                }
                if let Some(reference) = self.state.source_identity(source) {
                    self.note_mana_activation(reference);
                }
                casting::add_intrinsic_mana(&mut self.state, player, source, color);
                #[cfg(test)]
                crate::ability_log::intrinsic_mana(&self.state, &self.lookup, source, color);
                self.after_action(player);
                Ok(())
            }
            (Pending::ChooseColor { player: p, .. }, PlayerAction::ChooseColor(color))
                if *p == player
                    && matches!(self.pending_plan, Some(PlanKind::ChooseColor { .. })) =>
            {
                let Some(PlanKind::ChooseColor { object }) = self.pending_plan.take() else {
                    unreachable!("guarded above");
                };
                if let Some(obj) = self.state.object_mut(object) {
                    obj.chosen_color = Some(color);
                }
                Ok(())
            }
            (Pending::ChooseColor { player: p, .. }, PlayerAction::ChooseColor(color))
                if *p == player =>
            {
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_with_color(&mut self.state, &mut res, color) {
                    resolve::Flow::Enter => {
                        self.resolution = Some(res);
                        self.awaiting_answer = false;
                    }
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
            // Every offered card exactly once, each pile within its bounds
            // (`arrangement_fault`, through `answer_fault`): anything else
            // would duplicate or vanish cards.
            (Pending::Arrange { player: p, .. }, PlayerAction::Arrange { piles })
                if *p == player =>
            {
                if self.resolution.is_none() {
                    crate::graveyard_order::answer(&mut self.state, &piles[0]);
                    return Ok(());
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_arranged(&mut self.state, &mut res, &piles) {
                    resolve::Flow::Enter => {
                        self.resolution = Some(res);
                        self.awaiting_answer = false;
                    }
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
                if matches!(self.pending_plan, Some(PlanKind::UntapReplacements(_))) {
                    let Some(PlanKind::UntapReplacements(mut batch)) = self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    batch.answer(answer);
                    self.commit_untap_batch(batch);
                    return Ok(());
                }
                // An activation's Phyrexian symbol: yes is 2 life, no is its
                // mana (CR 107.4f). Nothing is paid yet; the answer is kept
                // and the activation goes on to its next question.
                if let Some(PlanKind::ChoosePhyrexianLife {
                    source,
                    ability_index,
                }) = self.pending_plan
                {
                    self.pending_plan = None;
                    self.activation_phyrexian.push(answer);
                    // Either answer is taken, for the reason the graveyard's
                    // is: the question was asked only where both can pay.
                    if self
                        .start_activation(player, source, ability_index, SmallVec::new())
                        .is_err()
                    {
                        self.reverse_activation(player);
                    }
                    return Ok(());
                }
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
                    let Some(PlanKind::DelayedPay { cost, source }) = self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    if answer {
                        let mut legal = self.compute_legal(player);
                        self.narrow_to_mana(&mut legal);
                        self.mana_window = Some(PaymentWindow {
                            player,
                            suspended: PaymentContinuation::Pact { cost, source },
                        });
                        self.pending = Pending::Priority {
                            player,
                            legal: Box::new(legal),
                        };
                        self.awaiting_answer = true;
                    } else {
                        let _ = sba::lose_by_effect(&mut self.state, player);
                    }
                    return Ok(());
                }
                // Echo: pay the echo cost or sacrifice the permanent.
                if matches!(
                    self.pending_plan,
                    Some(PlanKind::DelayedPaySacrifice { .. })
                ) {
                    let Some(PlanKind::DelayedPaySacrifice {
                        cost,
                        card,
                        version,
                    }) = self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    let paid = answer
                        && casting::pay_mana_from(
                            &mut self.state,
                            player,
                            baylee_core::ids::DamageSourceRef {
                                object: card,
                                version,
                            },
                            &cost,
                        );
                    debug_assert!(!answer || paid, "echo cost was offered as payable");
                    if !paid {
                        if let Some(obj) = self.state.object_mut(card) {
                            obj.kind = ObjectKind::Card;
                        }
                        let _ = self.state.sacrifice_object(card, Cause::Effect);
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
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
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
                        Some(crate::resolve::AwaitingOp::PlayerMayPay { player, cost, .. }) => {
                            Some((player, cost, resolve::payment_for(&self.state, r)))
                        }
                        _ => None,
                    });
                    if let Some((payer, cost, what)) = asked
                        && answer
                        && payer == player
                        && !self.can_pay_mana(player, what, &cost)
                    {
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
                                suspended: PaymentContinuation::Tax(Box::new(suspended)),
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
                    let mut res = self.resolution.take().expect("resolution suspended");
                    let answer = answer && self.can_settle_tax(&res);
                    match resolve::resume_tax_choice(&mut self.state, &mut res, answer) {
                        resolve::Flow::Enter => {
                            self.resolution = Some(res);
                            self.awaiting_answer = false;
                        }
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
                // A turn that would begin, offered to a skip (CR 614.10).
                if matches!(self.pending_plan, Some(PlanKind::SkipTurn { .. })) {
                    let Some(PlanKind::SkipTurn {
                        source,
                        mut declined,
                    }) = self.pending_plan.take()
                    else {
                        unreachable!()
                    };
                    if answer {
                        #[cfg(test)]
                        crate::ability_log::replaced_by(
                            source,
                            baylee_cards_dsl::ReplacementRule::SkipTurnToUntapSelf,
                        );
                        // "Instead of doing [something], do nothing": the
                        // turn is gone, an extra one spent like any other,
                        // and the order goes on from where it would have
                        // left it: a skipped normal turn moves it on, a
                        // skipped extra turn leaves it with the normal turn
                        // it was added after (CR 500.7). The untap waits for
                        // the next turn that actually occurs (CR 614.10b).
                        self.skip_pending_turn();
                        if let Some(version) = self.state.object(source).map(|o| o.version) {
                            self.state.skip_followups.push((source, version));
                        }
                        return Ok(());
                    }
                    declined.push(source);
                    if !self.offer_turn_skip(player, declined) {
                        self.begin_or_skip_turn(player);
                    }
                    return Ok(());
                }
                // A draw in the draw step, offered to a skip outside any
                // resolution (Island Sanctuary): the answer settles that
                // card, and the next waiting draw may be asked in turn.
                if matches!(self.pending_plan, Some(PlanKind::SkipDraw { .. })) {
                    let Some(PlanKind::SkipDraw { source }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    if let Some((next_player, next)) =
                        self.state.draw_offer_answered(source, answer)
                    {
                        self.ask_draw_skip(next_player, next);
                    }
                    return Ok(());
                }
                // Miracle offer: yes starts the miracle cast wizard.
                if matches!(self.pending_plan, Some(PlanKind::Miracle { .. })) {
                    let Some(PlanKind::Miracle { card }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    // "Yes" is taken like every answer a question offers.
                    // A cast it cannot follow through is reversed (CR 601.2,
                    // 732.1) and the card stays in hand: the offer was the
                    // miracle's one chance (CR 702.94a), so that is a "no".
                    // It used to be refused, which a driver that proposes
                    // yes again reads as a question with no answer (the
                    // arena, Metamorphosis Fanatic), and before that it spent
                    // the offer and then refused, which no record could
                    // replay (r001 games 1581, 3288, 3554).
                    if answer {
                        // `start_miracle_cast` touches nothing on its way
                        // to an error, so what is left is the declined offer.
                        let _ = self.start_miracle_cast(player, card);
                    }
                    return Ok(());
                }
                // A discovered card (CR 701.57a): yes casts it without paying
                // its mana cost, and "if you don't cast it, put that card
                // into your hand" — a no, or a cast the wizard refuses after
                // all.
                if matches!(self.pending_plan, Some(PlanKind::Discovered { .. })) {
                    let Some(PlanKind::Discovered { card }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    if answer && self.start_free_cast(player, card).is_ok() {
                        return Ok(());
                    }
                    self.discovered_to_hand(card);
                    return Ok(());
                }
                if matches!(self.pending_plan, Some(PlanKind::EntryCounters { .. })) {
                    let Some(PlanKind::EntryCounters {
                        object,
                        kind,
                        amount,
                    }) = self.pending_plan.take()
                    else {
                        unreachable!("player-counter entry question")
                    };
                    if answer {
                        self.state.set_tapped(object, true);
                        self.state
                            .change_player_counters(player, kind, i32::from(amount));
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
                        self.state
                            .change_life(player, -i32::from(amount), Cause::Cost);
                    } else {
                        self.state.set_tapped(object, true);
                    }
                    return Ok(());
                }
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|wizard| wizard.stage == cast_wizard::WizardStage::ManaPermission)
                {
                    let wizard = self.cast_wizard.as_mut().expect("wizard active");
                    wizard.north_star = if answer {
                        casting::north_star_effect(&self.state, player)
                    } else {
                        None
                    };
                    wizard.stage = cast_wizard::WizardStage::XValue;
                    self.continue_cast_wizard();
                    return Ok(());
                }
                // Wizard path: a Phyrexian symbol, asked at the head of the
                // X stage. Yes is 2 life, no its mana (CR 107.4f); nothing is
                // paid yet, and the stage asks its next question.
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::XValue)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.phyrexian.push(answer);
                    self.cast_wizard = Some(wizard);
                    self.continue_cast_wizard();
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
                    wizard.stage = cast_wizard::WizardStage::Replicate;
                    self.cast_wizard = Some(wizard);
                    self.continue_cast_wizard();
                    return Ok(());
                }
                let mut res = self.resolution.take().expect("resolution suspended");
                match resolve::resume_yes_no(&mut self.state, &mut res, answer) {
                    resolve::Flow::Enter => {
                        self.resolution = Some(res);
                        self.awaiting_answer = false;
                    }
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
            // Count, membership and repeats were checked against the
            // question (`Pending::answer_fault`): every option is a distinct
            // object, so a repeat is one card counted twice — delve reads
            // `delve_exiles.len()` and exiles each card once, so `[c, c]`
            // bought two generic mana with one card (CR 702.66a). So was
            // crew's total (CR 702.122a), which no count of objects can say
            // and which the question states as its `total`.
            (Pending::ChooseCards { player: p, .. }, PlayerAction::ChooseObjects { objects })
                if *p == player =>
            {
                if matches!(self.pending_plan, Some(PlanKind::SkipDraw { .. })) {
                    let Some(PlanKind::SkipDraw { source }) = self.pending_plan.take() else {
                        unreachable!()
                    };
                    if let Some((next_player, next)) =
                        self.state.draw_choice_answered(source, &objects)
                    {
                        self.ask_draw_skip(next_player, next);
                    }
                    return Ok(());
                }
                if let Some(result) = self.answer_effect_play(player, &objects) {
                    return result;
                }
                // Wizard path: pitch cards (exile-from-hand costs).
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::PitchChoice)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.pitch.extend(objects);
                    self.cast_wizard = Some(wizard);
                    // Taken like every other answer the wizard asks, and a
                    // cast that cannot go on from it is reversed (CR 601.2,
                    // 732.1). It was the one wizard answer refused instead,
                    // after the wizard had already been dropped: a refusal
                    // that moved the engine, and a question left standing
                    // with nothing behind it.
                    self.continue_cast_wizard();
                    return Ok(());
                }
                // Wizard path: escape's other cards (CR 702.138a).
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|wizard| wizard.stage == cast_wizard::WizardStage::ReturnCreature)
                {
                    let wizard = self.cast_wizard.as_mut().expect("cast wizard active");
                    wizard.returned = objects.first().copied();
                    wizard.stage = cast_wizard::WizardStage::Behold;
                    self.continue_cast_wizard();
                    return Ok(());
                }
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::Behold)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    if let Some(&chosen) = objects.first() {
                        wizard.beheld = Some(chosen);
                        wizard.behold_paid_mana = false;
                    } else {
                        wizard.beheld = None;
                        wizard.behold_paid_mana = true;
                    }
                    wizard.stage = cast_wizard::WizardStage::Delve;
                    self.cast_wizard = Some(wizard);
                    self.continue_cast_wizard();
                    return Ok(());
                }
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::Escape)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.escape_exiles = objects.into_iter().collect();
                    wizard.stage = cast_wizard::WizardStage::Done;
                    self.cast_wizard = Some(wizard);
                    self.continue_cast_wizard();
                    return Ok(());
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
                    self.continue_cast_wizard();
                    return Ok(());
                }
                // Wizard path: what pays the additional cost's sacrifice. The
                // stage stays where it is and asks about the next part, if
                // the face prints one; it moves on when there is none.
                if self
                    .cast_wizard
                    .as_ref()
                    .is_some_and(|w| w.stage == cast_wizard::WizardStage::Sacrifice)
                {
                    let mut wizard = self.cast_wizard.take().expect("wizard active");
                    wizard.cost_objects.extend(objects);
                    self.cast_wizard = Some(wizard);
                    self.continue_cast_wizard();
                    return Ok(());
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
                        remaining,
                        mut chosen,
                    }) => {
                        let is_single_graveyard = matches!(
                            &self.pending,
                            Pending::ChooseCards {
                                prompt: crate::choice::ChoicePrompt::CostExile,
                                ..
                            }
                        ) && remaining.is_some();
                        if is_single_graveyard {
                            let first_owner = chosen
                                .first()
                                .or_else(|| objects.first())
                                .and_then(|id| self.state.object(*id))
                                .and_then(|o| o.zone_owner);
                            if chosen.iter().chain(objects.iter()).any(|id| {
                                self.state.object(*id).and_then(|o| o.zone_owner) != first_owner
                            }) {
                                self.pending_plan = Some(PlanKind::PayActivationCost {
                                    source,
                                    ability_index,
                                    targets,
                                    target_players,
                                    remaining,
                                    chosen,
                                });
                                return Err(EngineError::IllegalAction(
                                    "cards must be exiled from a single graveyard",
                                ));
                            }
                        }
                        let remaining =
                            remaining.map(|n| n - u32::try_from(objects.len()).unwrap_or(u32::MAX));
                        chosen.extend_from_slice(&objects);
                        if remaining.is_some_and(|n| n > 0) {
                            let mut next = self.pending.clone();
                            if let Pending::ChooseCards {
                                options, min, max, ..
                            } = &mut next
                            {
                                let first_owner = chosen
                                    .first()
                                    .and_then(|id| self.state.object(*id))
                                    .and_then(|o| o.zone_owner);
                                let mut used = objects;
                                used.sort_unstable();
                                options.retain(|id| {
                                    used.binary_search(id).is_err()
                                        && (!is_single_graveyard
                                            || self.state.object(*id).and_then(|o| o.zone_owner)
                                                == first_owner)
                                });
                                *max = u8::try_from(remaining.unwrap_or(0)).unwrap_or(u8::MAX);
                                *min = *max;
                            }
                            self.pending_plan = Some(PlanKind::PayActivationCost {
                                source,
                                ability_index,
                                targets,
                                target_players,
                                remaining,
                                chosen,
                            });
                            self.pending = next;
                            self.awaiting_answer = true;
                            return Ok(());
                        }
                        self.activation_cost_choices.push(chosen);
                        self.activation_target_players = target_players;
                        if self
                            .start_activation(player, source, ability_index, targets)
                            .is_err()
                        {
                            self.reverse_activation(player);
                        }
                        return Ok(());
                    }
                    // The untap step's determination (CR 502.3). The answer
                    // names what stays tapped, and the step carries on with
                    // any untap limit and then "then they untap them all
                    // simultaneously" — never from the top, where phasing
                    // and the day/night check have already happened.
                    Some(PlanKind::UntapChoice) => {
                        self.untap_under_limits(objects, Vec::new());
                        return Ok(());
                    }
                    // What untaps under a limit (CR 502.3): counted against
                    // every limit, and asked again while any has room.
                    Some(PlanKind::UntapLimit { kept, mut chosen }) => {
                        chosen.extend(objects);
                        self.untap_under_limits(kept, chosen);
                        return Ok(());
                    }
                    // The attackers in a band with an attacker with banding
                    // (CR 508.1e, 702.22c).
                    Some(PlanKind::Band { leader }) => {
                        return self.answer_band(player, leader, objects);
                    }
                    // Camouflage's piles, instead of declaring blockers.
                    Some(PlanKind::CamouflagePiles {
                        defending,
                        piles,
                        of,
                    }) => {
                        self.answer_camouflage(defending, piles, of, objects);
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
                    Some(PlanKind::EntryReveal {
                        object,
                        untapped_without_reveal,
                    }) => {
                        if objects.is_empty() {
                            if !untapped_without_reveal {
                                self.state.set_tapped(object, true);
                            }
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
                    resolve::Flow::Enter => {
                        self.resolution = Some(res);
                        self.awaiting_answer = false;
                    }
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
            (Pending::Priority { player: p, .. }, PlayerAction::ActivateManaAbility { source })
                if *p == player =>
            {
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
                // unless the player names the other ability by index. Asked
                // of the offer's own predicate: a dual land under Chromatic
                // Lantern is listed for the grant alone, its shortcut empty
                // because its card prints both colours, and asking only
                // whether the land could be tapped sent it down this branch
                // to be refused.
                let colors =
                    casting::intrinsic_mana_choices(&self.state, &self.lookup, player, source);
                if !colors.is_empty() {
                    // CR 305.6 grants one separate mana ability per basic
                    // land type. This shortcut chooses WHICH ability to
                    // activate before paying its cost, unlike a printed
                    // "add one mana of any color" resolution choice.
                    if colors.len() > 1 {
                        self.pending_plan = Some(PlanKind::IntrinsicMana { source });
                        self.pending = Pending::ChooseColor {
                            player,
                            options: colors,
                        };
                        self.awaiting_answer = true;
                        return Ok(());
                    }
                    if !casting::pay_intrinsic_mana_price(&mut self.state, player, source) {
                        return Err(EngineError::IllegalAction("cannot pay mana ability cost"));
                    }
                    if let Some(reference) = self.state.source_identity(source) {
                        self.note_mana_activation(reference);
                    }
                    // Exactly one colour: nothing to ask.
                    casting::add_intrinsic_mana(&mut self.state, player, source, colors[0]);
                    #[cfg(test)]
                    crate::ability_log::intrinsic_mana(
                        &self.state,
                        &self.lookup,
                        source,
                        colors[0],
                    );
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
            ) if *p == player => self.declare_blockers(player, &blockers),
            (Pending::DiscardChoice { player: p, .. }, PlayerAction::ChooseObjects { objects })
                if *p == player =>
            {
                // The count, and no card named twice, are the question's
                // (`answer_fault`): one card named twice passed the count
                // and left the hand over its maximum (CR 514.1). That the
                // cards are in the hand is the engine's, since the question
                // offers the seat's whole hand without listing it.
                for card in &objects {
                    if !self.in_hand(player, *card) {
                        return Err(EngineError::IllegalAction("card not in hand"));
                    }
                }
                let since = self.state.journal.last_seq();
                for card in objects {
                    self.state.record_discard(card, player, None, true);
                    self.state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::TurnBased,
                    )?;
                    self.state.record_discard_successor(card, player);
                }
                crate::graveyard_order::capture(&mut self.state, since);
                // Then CR 514.2, and the step's check after it (CR 514.3a):
                // a trigger on the discard waits for that check like any
                // other, since triggers are collected from the journal.
                self.cleanup_ends_the_turns_effects();
                Ok(())
            }
            (
                Pending::LegendChoice { player: p, options },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                // Exactly one of the options, which `answer_fault` checked.
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

    /// Where an activation goes when the step its answer re-entered
    /// (`start_activation`) refuses to go on.
    ///
    /// The answer was one of the options its question enumerated, and by the
    /// time `start_activation` refuses, the question's plan has been taken:
    /// handing the refusal back to the caller left the question on the table
    /// with nothing behind it, and the next answer to it panicked the engine
    /// ("target plan set"), or reached for a resolution that was never there.
    /// An activation that can't legally be completed is reversed and the
    /// player who had priority keeps it (CR 733.1, CR 733.2), so that is what
    /// happens: the activation's answers so far are dropped, nothing reaches
    /// the stack, and its activator is asked for priority again. The answer
    /// itself is accepted, because it was a legal one and it did end the
    /// activation, and so a record replays it.
    ///
    /// Nothing known reaches this with a legal board. It is the net under
    /// every refusal `start_activation` has left on its way to the stack.
    pub(super) fn reverse_activation(&mut self, player: PlayerId) {
        self.activation_cost_choices.clear();
        self.activation_x = None;
        self.activation_graveyard = None;
        self.activation_phyrexian.clear();
        self.activation_second_targets = None;
        self.activation_target_choice = super::targeting::ActivationTargetChoice::default();
        self.activation_mode = None;
        self.activation_target_references = crate::sources::TargetReferences::default();
        self.activation_targets_answered = false;
        self.activation_target_players.clear();
        self.pending_plan = None;
        self.regrant_priority = Some(player);
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
        taken.saturating_sub(self.house_rules.free_mulligan_count())
    }

    // --------------------------------------------------------- turn steps
}
