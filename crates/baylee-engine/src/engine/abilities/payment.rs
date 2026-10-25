//! Counter-order suspension inside ordinary activation payment.
#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone, Debug, Hash)]
pub(crate) struct CostPayment {
    pub player: PlayerId,
    pub source: ObjectId,
    pub cost: ActivationPrice,
    pub chosen: Vec<Vec<ObjectId>>,
    pub x: u32,
    pub paid: crate::object::PaidRecord,
    pub next: usize,
    pub answer: usize,
    pub mana_paid: bool,
    pub subject_before: Option<baylee_core::ids::DamageSourceRef>,
    pub subject_seq: u64,
    pub graveyard_batch: crate::graveyard_order::PaymentBatch,
}

impl CostPayment {
    pub(super) fn new(
        state: &crate::state::GameState,
        player: PlayerId,
        source: ObjectId,
        cost: ActivationPrice,
        chosen: Vec<Vec<ObjectId>>,
        x: u32,
    ) -> Self {
        Self {
            player,
            source,
            cost,
            chosen,
            x,
            paid: crate::object::PaidRecord::default(),
            next: 0,
            answer: 0,
            mana_paid: false,
            subject_before: state.source_identity(source),
            subject_seq: state.journal.last_seq(),
            graveyard_batch: crate::graveyard_order::PaymentBatch::new(state),
        }
    }
}

/// The original apply checkpoint is retained only when payment actually suspends.
#[derive(Clone)]
pub(crate) struct PaymentRollback {
    checkpoint: Box<super::super::checkpoint::Checkpoint>,
    fingerprint: u64,
}
impl std::fmt::Debug for PaymentRollback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PaymentRollback")
            .field(&self.fingerprint)
            .finish()
    }
}
impl std::hash::Hash for PaymentRollback {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.fingerprint, h);
    }
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct ActivationPayment {
    pub kind: ActivationPaymentKind,
    rollback: Option<PaymentRollback>,
}
impl ActivationPayment {
    pub(super) fn new(kind: ActivationPaymentKind) -> Self {
        Self {
            kind,
            rollback: None,
        }
    }
    pub(crate) fn needs_rollback(&self) -> bool {
        self.rollback.is_none()
    }
    pub(in crate::engine) fn retain_rollback(
        &mut self,
        checkpoint: super::super::checkpoint::Checkpoint,
    ) {
        let fingerprint = checkpoint.payment_fingerprint();
        self.rollback = Some(PaymentRollback {
            checkpoint: Box::new(checkpoint),
            fingerprint,
        });
    }
}

#[derive(Clone, Debug, Hash)]
pub(crate) enum ActivationPaymentKind {
    Ordinary(Box<OrdinaryPayment>),
    Loyalty {
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        wanted: Option<baylee_cards_dsl::TargetReq>,
    },
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct OrdinaryPayment {
    pub work: CostPayment,
    pub completion: ActivationCompletion,
}

#[derive(Clone, Debug, Hash)]
pub(crate) struct ActivationCompletion {
    pub player: PlayerId,
    pub source: ObjectId,
    pub ability_index: u32,
    pub targets: SmallVec<[ObjectId; 2]>,
    pub chosen_players: Vec<PlayerId>,
    pub cost: ActivationPrice,
    pub x: u32,
    pub fixed_mana_cost: baylee_core::mana::ManaCost,
    pub activation_life: i32,
    pub activated_source_version: Option<u32>,
    pub activated_text: crate::text_changes::TextChangeMap,
    pub circled_colors: baylee_core::color::ColorSet,
    pub acquisition: u64,
    pub cycling: bool,
    pub limit: ActivationLimit,
    pub effects: &'static [baylee_cards_dsl::Effect],
    pub mana_ability: bool,
    pub granted: Option<crate::effects::GrantedAbility>,
    pub second: Option<baylee_cards_dsl::TargetReq>,
    pub first: Option<baylee_cards_dsl::TargetReq>,
    pub mode_index: Option<u8>,
}

impl<L: CardLookup> Engine<L> {
    /// Continue only after the pending counter event has actually committed.
    pub(crate) fn resume_activation_payment(&mut self) -> Result<(), EngineError> {
        let Some(payment) = self.activation_payment.take() else {
            return Ok(());
        };
        // A following sacrifice reads the characteristic changed by this counter event.
        self.state.refresh_characteristics();
        self.capture_cost_triggers();
        let ActivationPayment { kind, rollback } = payment;
        match kind {
            ActivationPaymentKind::Ordinary(payment) => {
                let OrdinaryPayment {
                    mut work,
                    completion,
                } = *payment;
                match self.advance_cost_payment(&mut work) {
                    Ok(false) => {
                        self.activation_payment = Some(ActivationPayment {
                            kind: ActivationPaymentKind::Ordinary(Box::new(OrdinaryPayment {
                                work,
                                completion,
                            })),
                            rollback,
                        });
                        self.offer_counter_order();
                        return Ok(());
                    }
                    Ok(true) => {}
                    Err(error) => {
                        let Some(rollback) = rollback else {
                            return Err(error);
                        };
                        // The ordering answer was legal, but the activation cannot
                        // finish. Reverse the whole attempt, unlike an invalid
                        // submitted answer, which preserves its current prompt.
                        self.cancel_counter_payment(rollback, completion.player);
                        return Ok(());
                    }
                }
                self.complete_paid_activation(completion, work.paid);
                Ok(())
            }
            ActivationPaymentKind::Loyalty {
                player,
                source,
                ability_index,
                targets,
                wanted,
            } => self.complete_loyalty_payment(player, source, ability_index, targets, wanted),
        }
    }

    fn cancel_counter_payment(&mut self, rollback: PaymentRollback, player: PlayerId) {
        (*rollback.checkpoint).restore(self);
        self.activation_payment = None;
        self.activation_mode = None;
        self.pending_plan = None;
        self.activation_cost_choices.clear();
        self.activation_target_players.clear();
        self.activation_second_targets = None;
        self.activation_target_references = crate::sources::TargetReferences::default();
        self.activation_target_choice = super::super::targeting::ActivationTargetChoice::default();
        self.activation_targets_answered = false;
        self.activation_x = None;
        self.activation_phyrexian.clear();
        self.activating_abilities = None;
        self.activating_grant = None;
        self.awaiting_answer = false;
        self.after_action(player);
    }

    #[allow(clippy::too_many_lines)] // Completion preserves the activation checklist.
    pub(super) fn complete_paid_activation(
        &mut self,
        completion: ActivationCompletion,
        mut paid: crate::object::PaidRecord,
    ) {
        let ActivationCompletion {
            player,
            source,
            ability_index,
            targets,
            chosen_players,
            cost,
            x,
            fixed_mana_cost,
            activation_life,
            activated_source_version,
            activated_text,
            circled_colors,
            acquisition,
            cycling,
            limit,
            effects,
            mana_ability,
            granted,
            second,
            first,
            mode_index,
        } = completion;
        paid.fixed_mana_cost = fixed_mana_cost;
        // The life the Phyrexian symbols were paid with, beside the rest of
        // the cost (CR 601.2h; CR 119.4 was asked above).
        if activation_life > 0 {
            self.state
                .change_life(player, -activation_life, crate::event::Cause::Cost);
        }
        // "When you cycle this card" is "when you discard this card to pay
        // an activation cost of a cycling ability" (CR 702.29c), and this is
        // the one place that knows both halves: `pay_cost` sees a cost, not
        // whose ability it is.
        if cycling {
            self.state.journal.record(GameEvent::Cycled {
                object: source,
                player,
            });
        }
        // "Activate only once each turn" is spent *here* and not at the
        // offer, because this is the line the rules count: CR 602.2 makes
        // activating an ability putting it on the stack and paying its
        // costs, and every path above this one still ends in a refusal or a
        // question. An activation abandoned over a target choice has
        // announced nothing. "If this ability has been activated four or
        // more times this turn" (Dragon Whelp) reads the same count, so an
        // ability whose effects ask it is counted too; no other is, which
        // keeps a repeatable activation out of the loop signature.
        if matches!(
            limit,
            ActivationLimit::PerTurn(_)
                | ActivationLimit::PerTurnAmount(_)
                | ActivationLimit::Boast
        ) || asks_its_activations(effects)
        {
            *self
                .state
                .ability_fires
                .entry((
                    baylee_core::ids::DamageSourceRef {
                        object: source,
                        version: activated_source_version.unwrap_or_else(|| {
                            self.state
                                .object(source)
                                .expect("activation source")
                                .version
                        }),
                    },
                    ability_index,
                ))
                .or_insert(0) += 1;
        }
        if limit == ActivationLimit::Exhaust {
            // Costs may have moved the source. Do not exhaust a new incarnation.
            if let Some(object) = self.state.object_mut(source)
                && Some(object.version) == activated_source_version
            {
                object
                    .riders
                    .push(crate::object::Rider::ExhaustedAbility(ability_index));
            }
        }
        if mana_ability {
            self.activation_target_choice =
                super::super::targeting::ActivationTargetChoice::default();
            self.record_activation(player, source, activated_source_version, &cost, None);
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
                // CR 605.1a: a mana ability has no target, so it has no
                // second one either — `lints::mana_ability_fault` refuses
                // either list on one.
                second_targets: SmallVec::new(),
                // The number this activation announced, which is what every
                // `Amount::X` in its effects reads: "Add {W} for each storage
                // counter removed this way" is the counters that just came
                // off as a cost.
                x: Some(x),
                chosen_player: None,
                target_players: baylee_core::ids::SeatSet::new(),
                event_object: None,
                targeted: false,
                awaiting: None,
                mana_ability: true,
                countered_source: None,
                target_lki: None,
                subject: resolve::SubjectContext::after_cost(&paid)
                    .with_source_version(activated_source_version)
                    .with_mana_tap_source(cost.parts.contains(&CostPart::TapSelf).then_some(
                        baylee_core::ids::DamageSourceRef {
                            object: source,
                            version: activated_source_version.expect("activated source"),
                        },
                    ))
                    .with_circled_colors(circled_colors)
                    .with_acquisition(acquisition),
                text: activated_text,
                event_mana: None,
                retarget_left: None,
                loot: None,
                draw_watch: None,
                this_way_from: None,
                coin_stake: None,
            };
            let flow = resolve::run(&mut self.state, &mut res);
            #[cfg(test)]
            crate::ability_log::mana_activated(
                &self.state,
                &self.lookup,
                source,
                ability_index,
                matches!(flow, resolve::Flow::Complete),
            );
            match flow {
                resolve::Flow::Complete => {}
                resolve::Flow::Enter => {
                    self.resolution = Some(res);
                    self.awaiting_answer = false;
                }
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                    return;
                }
            }
        } else {
            let ability = if let Some(granted) = granted {
                self.push_granted_ability(player, source, targets, &granted)
            } else {
                self.push_ability_to_stack(player, source, ability_index, targets)
            };
            if let Some(obj) = self.state.object_mut(ability) {
                obj.mode_index = mode_index;
                obj.target_req = first;
            }
            if granted.is_some() && mode_index.is_some() {
                self.synthetic_fx.insert(ability, effects);
                self.state.capture_linked_references(ability, effects);
            }
            if let Some(version) = activated_source_version
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.riders
                    .retain(|r| !matches!(r, crate::object::Rider::AbilitySourceVersion(_)));
                obj.riders
                    .push(crate::object::Rider::AbilitySourceVersion(version));
            }
            // The second instance's answer, taken so that it belongs to this
            // activation and to no other.
            if let Some(second_targets) = self.activation_second_targets.take()
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.set_second(second_targets, second);
                obj.set_second_players(self.activation_target_choice.second_players);
            }
            self.state
                .bind_target_references(ability, &self.activation_target_references);
            self.activation_target_references = crate::sources::TargetReferences::default();
            // The number this activation announced, carried on the ability
            // the way a spell carries its own X (CR 601.2b). No card in the
            // pool prints a counter-X cost on an ability that uses the stack
            // — all seventeen are mana abilities, which resolve without one
            // — so this line has no card behind it and is here because the
            // alternative is an ability that pays for X and resolves with
            // nought. That is a claim about the pool rather than about this
            // function, so it is held by a scan and not by this comment:
            // `offer_tests::every_counter_x_cost_in_the_pool_is_on_a_mana_ability`.
            if x > 0
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.x_value = x;
            }
            // What the cost sacrificed or tapped, carried on the ability for
            // the effect that asks (Birthing Pod's "1 plus the sacrificed
            // creature's mana value", station's "the tapped creature's
            // power"), the way a spell carries its own.
            if paid != crate::object::PaidRecord::default()
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.paid = Some(Box::new(paid));
            }
            self.apply_ability_spend_riders(player, ability);
            // The seats that were targeted, written onto the ability now
            // that there is one — the same two fields the trigger path
            // writes, and for the same reason: `target_players` is the set
            // that was named, `chosen_player` the single seat
            // `PlayerRel::Chosen` reads back at resolution. It cannot wait
            // until `apply` has this call's answer, because `after_action`
            // below stacks whatever a sacrifice cost triggered, and that
            // lands above this ability.
            if !chosen_players.is_empty()
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.target_players = chosen_players.iter().copied().collect();
                if let [only] = chosen_players[..] {
                    obj.chosen_player = Some(only);
                }
            }
            self.record_activation(
                player,
                source,
                activated_source_version,
                &cost,
                Some(ability),
            );
        }
        self.activating_grant = None;
        self.activation_target_choice = super::super::targeting::ActivationTargetChoice::default();
        self.after_action(player);
    }
}
