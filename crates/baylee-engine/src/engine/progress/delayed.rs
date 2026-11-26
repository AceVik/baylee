//! Carrying out delayed actions: free casts, discoveries, delayed triggers.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Cascade's cast (CR 702.85a), as its resolution ends: the exiled
    /// card is cast without paying its mana cost, or put on the bottom of
    /// its owner's library when it cannot be. Returns whether a question is
    /// out.
    fn cast_free_or_bottom(&mut self, controller: PlayerId, card: ObjectId, version: u32) -> bool {
        let in_exile = |state: &crate::state::GameState| {
            state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
                .map(|o| o.owner)
        };
        if in_exile(&self.state).is_none() {
            return false;
        }
        // A spell nothing can be pointed at cannot be cast
        // (CR 601.2c); asked first, so a refusal leaves no wizard
        // behind to unwind.
        let cast =
            crate::casting::face_has_a_legal_target(&self.state, &self.lookup, controller, card, 0)
                && self.start_free_cast(controller, card).is_ok();
        if !cast && let Some(owner) = in_exile(&self.state) {
            // "…that weren't cast on the bottom of your library."
            let _ = self.state.move_object(
                card,
                ZoneLocation::Library(owner),
                ZonePosition::Bottom,
                Cause::Effect,
            );
        }
        self.awaiting_answer
    }

    /// `Effect::MayCastTarget`'s cast, said yes to: a CR 605.3a payment
    /// window for the card's mana cost, where the caster makes the mana
    /// the cast will be paid with (CR 608.2g lets them). Nothing opens for
    /// a card that has left the zone it was targeted in (CR 400.7) or that
    /// its caster may not begin to cast at all (CR 601.3). Returns whether
    /// a question is out.
    fn open_cast_payment(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        version: u32,
        then_no_more_spells: bool,
        any_mana: bool,
    ) -> bool {
        let Some(cost) = self
            .state
            .object(card)
            .filter(|o| o.version == version)
            .map(|o| o.characteristics().mana_cost)
        else {
            return false;
        };
        let cost = if any_mana {
            cost.with_any_type_component(&cost)
        } else {
            cost
        };
        if !crate::casting::may_begin_casting(&self.state, player) {
            return false;
        }
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        self.mana_window = Some(super::super::PaymentWindow {
            player,
            suspended: super::super::PaymentContinuation::Cast {
                card,
                version,
                cost,
                then_no_more_spells,
                any_mana,
                opened: Box::new(self.window_start(player)),
            },
        });
        self.pending = self.window_priority(player, legal);
        self.awaiting_answer = true;
        true
    }

    /// Offers a discovered card's cast to the player who discovered it
    /// (CR 701.57a), if it is still the card in exile that was found
    /// (`version`) — or puts it into the hand without asking when it cannot
    /// be cast. Returns `true` when the question is out.
    fn offer_discovered(&mut self, player: PlayerId, card: ObjectId, version: u32) -> bool {
        if !self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
        {
            return false;
        }
        if !self.free_cast_possible(player, card) {
            self.discovered_to_hand(card);
            return false;
        }
        self.pending_plan = Some(PlanKind::Discovered { card });
        self.pending = Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::Discover { card },
            source: None,
        };
        self.awaiting_answer = true;
        true
    }

    /// A discovered card that is not cast goes to its owner's hand
    /// (CR 701.57a), if it is still in exile.
    ///
    /// A priority grant already out is asked again: this runs after a free
    /// cast the wizard refused, which resumes the game on its way out, and
    /// the card joins a hand that grant's legal actions did not see.
    pub(crate) fn discovered_to_hand(&mut self, card: ObjectId) {
        let Some(owner) = self
            .state
            .object(card)
            .filter(|o| o.zone == crate::zone::Zone::Exile)
            .map(|o| o.owner)
        else {
            return;
        };
        let _ = self.state.move_object(
            card,
            ZoneLocation::Hand(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
        if let Pending::Priority { player, .. } = self.pending {
            self.pending = Pending::Priority {
                player,
                legal: Box::new(self.compute_legal(player)),
            };
        }
    }

    /// A delayed "transform it": only the permanent it was made for, on the
    /// battlefield as the same object and still showing the same face
    /// (CR 400.7), turns over.
    fn transform_if_unchanged(&mut self, card: ObjectId, version: u32, face: u8) {
        let still_there = self.state.object(card).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield && o.version == version && o.face_index == face
        });
        if still_there {
            if let Some(def) = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            {
                self.state
                    .transform(card, def, 1 - usize::from(face.min(1)));
            } else {
                self.state.transform_token(card, 1 - face.min(1));
            }
        }
    }

    /// Put every due stack trigger into one APNAP batch. Nontrigger delayed
    /// actions retain their relative order and their existing progress path.
    pub(super) fn queue_due_delayed_triggers(&mut self) {
        let count = self.delayed_queue.len();
        for _ in 0..count {
            let Some((controller, action)) = self.delayed_queue.pop_front() else {
                break;
            };
            if matches!(
                action,
                crate::state::DelayedAction::Trigger { .. }
                    | crate::state::DelayedAction::PrintedTrigger { .. }
                    | crate::state::DelayedAction::TriggerAbout { .. }
                    | crate::state::DelayedAction::GeneralTrigger(_)
                    | crate::state::DelayedAction::LinkedCounterCleanup { .. }
                    | crate::state::DelayedAction::ReturnExiledGroup { .. }
                    | crate::state::DelayedAction::FlickerformReturn { .. }
                    | crate::state::DelayedAction::DrawStepLifeLoss { .. }
                    | crate::state::DelayedAction::AddMana { .. }
                    | crate::state::DelayedAction::AddManaAnyColor { .. }
            ) {
                self.delayed_queue.push_front((controller, action));
                let suspended = self.process_delayed();
                debug_assert!(!suspended, "queuing a trigger cannot ask a question");
            } else {
                self.delayed_queue.push_back((controller, action));
            }
        }
    }

    /// Processes one queued delayed action; returns `true` when a pending
    /// choice was produced.
    #[allow(clippy::too_many_lines)] // one arm per delayed action; splitting hides the list
    pub(crate) fn process_delayed(&mut self) -> bool {
        let Some((controller, action)) = self.delayed_queue.pop_front() else {
            return false;
        };
        // This is where a delayed trigger that has come due would be put on
        // the stack, and one controlled by a player who has left the game
        // isn't (CR 800.4d): Swift Spiral's creature stays in exile once
        // its caster has gone. The controller is the one who controlled the
        // spell or ability that made it (CR 603.7d, 603.7e).
        if self.state.has_left(controller) {
            return false;
        }
        match action {
            crate::state::DelayedAction::DrawStepLifeLoss {
                source,
                controller: trigger_controller,
                amount,
                ..
            } => {
                const LOSS: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::LoseLife {
                    amount: baylee_cards_dsl::Amount::EventAmount,
                    target: baylee_cards_dsl::PlayerRel::EventPlayer,
                }];
                self.queue_delayed_trigger(
                    trigger_controller,
                    source.object,
                    LOSS,
                    None,
                    Some(source.version),
                    crate::text_changes::TextChangeMap::IDENTITY,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    trigger.event_player = Some(controller);
                    trigger.event_amount = Some(amount);
                }
                false
            }
            crate::state::DelayedAction::ReturnExiledGroup {
                source,
                objects,
                tapped,
            } => {
                static RETURN: &[baylee_cards_dsl::Effect] =
                    &[baylee_cards_dsl::Effect::ReturnExiledGroupToOwners { tapped: false }];
                static RETURN_TAPPED: &[baylee_cards_dsl::Effect] =
                    &[baylee_cards_dsl::Effect::ReturnExiledGroupToOwners { tapped: true }];
                self.queue_delayed_trigger(
                    controller,
                    source.object,
                    if tapped { RETURN_TAPPED } else { RETURN },
                    None,
                    Some(source.version),
                    crate::text_changes::TextChangeMap::IDENTITY,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    trigger.related_objects = Some(std::sync::Arc::new(objects));
                }
                false
            }
            crate::state::DelayedAction::FlickerformReturn {
                source,
                host,
                auras,
            } => {
                static FLICKERFORM_RETURN: &[baylee_cards_dsl::Effect] =
                    &[baylee_cards_dsl::Effect::FlickerformReturnFromRelated];
                self.queue_delayed_trigger(
                    controller,
                    source.object,
                    FLICKERFORM_RETURN,
                    None,
                    Some(source.version),
                    crate::text_changes::TextChangeMap::IDENTITY,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    let mut related = Vec::with_capacity(1 + auras.len());
                    related.push(host);
                    related.extend(auras);
                    trigger.related_objects = Some(std::sync::Arc::new(related));
                }
                false
            }
            crate::state::DelayedAction::RestrictAttacksForTurn { .. }
            | crate::state::DelayedAction::ReturnToHandDuringUntap { .. } => {
                // Consumed only by the untap turn-based action, never queued.
                debug_assert!(false, "untap instruction reached delayed-trigger queue");
                false
            }
            crate::state::DelayedAction::CastFromExileWithoutPaying { card, version } => {
                let Some(object) = self
                    .state
                    .object(card)
                    .filter(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
                else {
                    return false;
                };
                let owner = object.owner;
                let from_suspend = object.riders.contains(&crate::object::Rider::Suspend);
                let _ = self.start_exile_free_cast(owner, card, from_suspend);
                self.awaiting_answer
            }
            crate::state::DelayedAction::CastFreeOrBottom { card, version } => {
                self.cast_free_or_bottom(controller, card, version)
            }
            crate::state::DelayedAction::CastPaying {
                card,
                version,
                then_no_more_spells,
                any_mana,
            } => self.open_cast_payment(controller, card, version, then_no_more_spells, any_mana),
            crate::state::DelayedAction::CastWithMadness { card, version } => {
                if self.start_madness_cast(controller, card, version).is_err() {
                    self.state.madness_to_graveyard(card, version);
                }
                self.awaiting_answer
            }
            crate::state::DelayedAction::Transform {
                card,
                version,
                face,
            } => {
                self.transform_if_unchanged(card, version, face);
                false
            }
            crate::state::DelayedAction::Sacrifice { card, version } => {
                let owner = self.state.object(card).and_then(|o| {
                    (o.zone == crate::zone::Zone::Battlefield
                        && o.version == version
                        && o.controller == controller
                        && self.state.can_be_sacrificed(card, Some(controller)))
                    .then_some(o.owner)
                });
                if owner.is_some() {
                    let _ = self
                        .state
                        .sacrifice_object(card, crate::event::Cause::Effect);
                }
                false
            }
            crate::state::DelayedAction::CastDiscovered { card, version } => {
                self.offer_discovered(controller, card, version)
            }
            crate::state::DelayedAction::ReturnToBattlefield { card, version } => {
                if self
                    .state
                    .object(card)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
                {
                    // End-step blink returns (Eerie Interlude, Swift
                    // Spiral): under the OWNER's control.
                    let owner = self.state.object(card).map(|o| o.owner);
                    if let Some(obj) = self.state.object_mut(card)
                        && let Some(owner) = owner
                    {
                        obj.set_controller(owner);
                    }
                    // The delayed trigger's controller puts it there.
                    self.state
                        .entries
                        .begin_event(self.state.journal.last_seq(), Some(controller));
                    let _ = self.state.move_object(
                        card,
                        ZoneLocation::Battlefield,
                        ZonePosition::Top,
                        crate::event::Cause::Effect,
                    );
                }
                false
            }
            // Mana Drain's "At the beginning of your next main phase, add":
            // a delayed triggered ability (CR 603.7) whose trigger condition
            // is a phase beginning rather than a mana activation or mana
            // production (CR 605.1b), so it is not a mana ability and goes on
            // the stack before resolving into the controller's pool.
            crate::state::DelayedAction::AddMana {
                source,
                source_version,
                color,
                amount,
            } => {
                self.queue_delayed_trigger(
                    controller,
                    source,
                    delayed_mana_effects(color),
                    None,
                    (source != ObjectId::NO_SOURCE).then_some(source_version),
                    crate::text_changes::TextChangeMap::IDENTITY,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    trigger.event_amount = Some(u32::from(amount));
                }
                false
            }
            crate::state::DelayedAction::AddManaAnyColor {
                source,
                source_version,
                amount,
            } => {
                const PLASM_MANA: &[baylee_cards_dsl::Effect] =
                    &[baylee_cards_dsl::Effect::AddMana {
                        source: baylee_cards_dsl::effect::ManaSource::Choice(
                            baylee_cards_dsl::ALL_MANA_COLORS,
                        ),
                        amount: baylee_cards_dsl::Amount::EventAmount,
                        distribution: baylee_cards_dsl::effect::ManaDistribution::AnyCombination,
                        restriction: None,
                    }];
                self.queue_delayed_trigger(
                    controller,
                    source,
                    PLASM_MANA,
                    None,
                    (source != ObjectId::NO_SOURCE).then_some(source_version),
                    crate::text_changes::TextChangeMap::IDENTITY,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    trigger.event_amount = Some(u32::from(amount));
                }
                false
            }
            // A delayed triggered ability that uses the stack, come due at a
            // step: it joins the trigger queue and is put on the stack from
            // there, as any trigger is. Earthbend's watch is read off the
            // journal (`trigger::watch_triggers`) and never reaches this
            // queue; no step-timed one exists yet.
            crate::state::DelayedAction::LinkedCounterCleanup {
                source,
                version,
                effects,
            } => {
                self.queue_delayed_trigger(
                    controller,
                    source,
                    effects,
                    None,
                    Some(version),
                    crate::text_changes::TextChangeMap::IDENTITY,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    trigger.counter_source_version = Some(version);
                }
                false
            }
            crate::state::DelayedAction::Trigger {
                source,
                source_version,
                effects,
                text,
            } => {
                self.queue_delayed_trigger(
                    controller,
                    source,
                    effects,
                    None,
                    Some(source_version),
                    text,
                );
                false
            }
            crate::state::DelayedAction::PrintedTrigger {
                source,
                printed,
                ability,
            } => {
                self.queue_printed_trigger(controller, source, printed, ability);
                false
            }
            // "That creature" while it is still that object (CR 603.7c):
            // one that has left the battlefield, even to come back, is a
            // new object and the trigger is about nothing (CR 400.7).
            crate::state::DelayedAction::TriggerAbout {
                source,
                source_version,
                effects,
                text,
                object,
                version,
                subject,
            } => {
                // The trigger is about the object as it was (CR 603.7c): one
                // that has left its zone since is named with the identity it
                // had, so "that creature" affects nothing (the identity is no
                // longer current) while what the effect reads about it, such
                // as the creatures a Wall blocked, is its last information
                // (CR 608.2h).
                let still = self
                    .state
                    .object(object)
                    .is_some_and(|o| o.version == version);
                self.queue_delayed_trigger(
                    controller,
                    source,
                    effects,
                    Some(object),
                    Some(source_version),
                    text,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    let power = if still {
                        self.state
                            .object(object)
                            .and_then(|o| o.characteristics().power)
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    trigger.event_object_identity = Some((version, power));
                    trigger.event_subject = subject;
                }
                false
            }
            crate::state::DelayedAction::GeneralTrigger(spec) => {
                let rem = spec.remembered.unwrap_or_default();
                let identity = rem.identity.map(|(ver, old_power)| {
                    let live_power = rem.object.and_then(|obj| {
                        self.state
                            .object(obj)
                            .filter(|o| o.version == ver)
                            .and_then(|o| o.characteristics().power)
                    });
                    (ver, live_power.unwrap_or(old_power))
                });
                let event_ref = rem
                    .object
                    .zip(identity.map(|(v, _)| v))
                    .map(|(object, version)| baylee_core::ids::DamageSourceRef { object, version });
                if !crate::eval::intervening_if_for_incarnation(
                    &self.state,
                    spec.condition.copied(),
                    controller,
                    spec.source,
                    Some(spec.source_version),
                    None,
                    event_ref,
                ) {
                    return false;
                }
                self.queue_delayed_trigger(
                    controller,
                    spec.source,
                    spec.effects,
                    rem.object,
                    Some(spec.source_version),
                    spec.text,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    if !spec.related_objects.is_empty() {
                        trigger.related_objects =
                            Some(std::sync::Arc::new(spec.related_objects.clone()));
                    }
                    trigger.synthetic_target = spec.targets;
                    trigger.synthetic_condition = spec.condition;
                    trigger.event_object_identity = identity;
                    trigger.event_departure = rem.departure;
                    trigger.event_mana_value = rem.mana_value;
                    trigger.event_player = rem.player;
                    trigger.event_amount = rem.amount;
                    trigger.event_subject = rem.subject;
                }
                false
            }
        }
    }

    /// A delayed triggered ability come due at a step joins the trigger
    /// queue, and is put on the stack from there (CR 603.7).
    fn queue_delayed_trigger(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        effects: &'static [baylee_cards_dsl::Effect],
        event_object: Option<ObjectId>,
        source_version: Option<u32>,
        text: crate::text_changes::TextChangeMap,
    ) {
        self.trigger_queue
            .push_back(crate::trigger::PendingTrigger {
                text,
                source_version,
                related_objects: None,
                found_objects: None,
                grantor: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: None,
                counter_source_version: None,
                event_amount: None,
                event_player: None,
                event_mana: None,
                event_mana_value: None,
                event_tap_snapshot: None,
                event_departure: None,
                source,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                abilities: None,
                controller,
                timestamp: self.state.object(source).map_or(0, |o| o.timestamp),
                event_object,
                implicit_target: None,
                synthetic_effects: Some(effects),
                once_per_turn: false,
                synthetic_target: None,
                synthetic_condition: None,
                chosen_mode: None,
                in_chosen_order: false,
            });
    }

    /// The trigger a delayed ability its card prints goes on the stack as:
    /// that ability of that list, with no synthetic body. Read off `printed`
    /// and not the source, which a copy of a spell no longer has.
    fn queue_printed_trigger(
        &mut self,
        controller: PlayerId,
        source: baylee_core::ids::DamageSourceRef,
        printed: crate::object::PrintedFace,
        ability: u32,
    ) {
        let Some(def) = self.lookup.card(printed.card()) else {
            return;
        };
        let list = crate::object::AbilityList::from_static(
            def.abilities_for_face(usize::from(printed.face())),
            Some(printed),
            None,
        );
        self.queue_delayed_trigger(
            controller,
            source.object,
            &[],
            None,
            Some(source.version),
            crate::text_changes::TextChangeMap::IDENTITY,
        );
        if let Some(trigger) = self.trigger_queue.back_mut() {
            trigger.synthetic_effects = None;
            trigger.ability_index = ability;
            trigger.abilities = Some(list);
        }
    }

    /// Echo as it resolves (CR 702.30a): "sacrifice it unless you pay
    /// [cost]", which is the tax `Effect::PlayerMayPayManaOr` asks, in the
    /// window a yes opens (CR 605.3a), for its controller.
    ///
    /// A player asked to pay may make the mana first, so the question is put
    /// while the pool pays it or anything could make mana for it. Only a
    /// player with neither is past asking, and the ability sacrifices the
    /// permanent outright. It used to do that to any pool that did not
    /// already pay it, beside untapped lands.
    pub(super) fn demand_echo(
        &self,
        controller: PlayerId,
        card: ObjectId,
        cost: baylee_core::mana::ManaCost,
    ) -> baylee_cards_dsl::Effect {
        use baylee_cards_dsl::{Effect, PlayerRel};
        let what = self.state.source_identity(card).map_or(
            super::super::casting::SpendFor::Other,
            super::super::casting::SpendFor::OtherFrom,
        );
        if !self.can_pay_mana(controller, what, &cost)
            && self.mana_window_legal(controller).is_none()
        {
            return Effect::SacrificeSelf;
        }
        Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost,
            effect: &Effect::SacrificeSelf,
        }
    }
}
/// Synthetic resolution program for a delayed mana trigger (`DelayedAction::AddMana`).
const fn delayed_mana_effects(
    color: baylee_core::mana::ManaColor,
) -> &'static [baylee_cards_dsl::Effect] {
    use baylee_cards_dsl::{Amount, Effect, PlayerRel};
    use baylee_core::mana::ManaColor;
    const WHITE: &[Effect] = &[Effect::AddManaAmountFor {
        who: PlayerRel::You,
        color: ManaColor::White,
        amount: Amount::EventAmount,
    }];
    const BLUE: &[Effect] = &[Effect::AddManaAmountFor {
        who: PlayerRel::You,
        color: ManaColor::Blue,
        amount: Amount::EventAmount,
    }];
    const BLACK: &[Effect] = &[Effect::AddManaAmountFor {
        who: PlayerRel::You,
        color: ManaColor::Black,
        amount: Amount::EventAmount,
    }];
    const RED: &[Effect] = &[Effect::AddManaAmountFor {
        who: PlayerRel::You,
        color: ManaColor::Red,
        amount: Amount::EventAmount,
    }];
    const GREEN: &[Effect] = &[Effect::AddManaAmountFor {
        who: PlayerRel::You,
        color: ManaColor::Green,
        amount: Amount::EventAmount,
    }];
    const COLORLESS: &[Effect] = &[Effect::AddManaAmountFor {
        who: PlayerRel::You,
        color: ManaColor::Colorless,
        amount: Amount::EventAmount,
    }];
    match color {
        ManaColor::White => WHITE,
        ManaColor::Blue => BLUE,
        ManaColor::Black => BLACK,
        ManaColor::Red => RED,
        ManaColor::Green => GREEN,
        ManaColor::Colorless => COLORLESS,
    }
}

#[cfg(test)]
#[path = "delayed/announcement_tests.rs"]
mod announcement_tests;

/// The engine's own steps, named for the card test crates (`test-support`) and
/// their kit: the steps stay the engine's, these only call them.
#[cfg(any(test, feature = "test-support"))]
impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    #[doc(hidden)]
    pub fn process_delayed_for_tests(&mut self) -> bool {
        self.process_delayed()
    }
}
