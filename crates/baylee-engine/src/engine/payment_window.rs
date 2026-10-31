//! The CR 605.3a payment window: a seat that has said it will pay makes
//! its mana here, one mana ability at a time, and passing settles the
//! payment the window was opened for ([`super::PaymentContinuation`]).

use super::{
    CardLookup, Cause, Engine, EngineError, GameEvent, ObjectId, PaymentContinuation,
    PaymentWindow, Pending, PlayerId, SmallVec, WindowStart, Zone, casting, resolve, sba,
};

impl<L: CardLookup> Engine<L> {
    /// `PlayerAction::PayWindow`: the window's mana abilities and their
    /// answers, then its settlement, as one action.
    ///
    /// Every step goes through the door it would take alone
    /// ([`Self::take_answer`]), so nothing about a mana ability is decided
    /// twice. The all-or-nothing half is [`Self::apply`]'s: it took a
    /// checkpoint before this ran and restores it on any refusal returned
    /// here, the journal cut back with it. So a refusal needs only to be
    /// returned, at whatever step it is found.
    pub(super) fn pay_window(
        &mut self,
        actor: PlayerId,
        player: PlayerId,
        steps: Vec<crate::choice::PlayerAction>,
    ) -> Result<(), EngineError> {
        let in_own_window = |engine: &Self| {
            engine
                .mana_window
                .as_ref()
                .is_some_and(|w| w.player == player)
                && matches!(engine.pending, Pending::Priority { player: p, .. } if p == player)
        };
        if !in_own_window(self) {
            return Err(EngineError::IllegalAction(
                "no payment window is open for this seat",
            ));
        }
        if self
            .mana_window
            .as_ref()
            .is_some_and(|w| matches!(w.suspended, PaymentContinuation::LandMana(_)))
        {
            return Err(EngineError::IllegalAction(
                "this window owes no payment to settle",
            ));
        }
        for step in steps {
            if !step.is_payment_step() {
                return Err(EngineError::IllegalAction(
                    "a payment carries only mana abilities and their answers",
                ));
            }
            self.take_answer(actor, step)?;
            if self.mana_window.as_ref().is_none_or(|w| w.player != player) {
                return Err(EngineError::IllegalAction(
                    "a payment step closed its own window",
                ));
            }
        }
        if !in_own_window(self) {
            return Err(EngineError::IllegalAction(
                "a payment step left its question unanswered",
            ));
        }
        self.awaiting_answer = false;
        if self.close_mana_window()? != WindowClose::Paid {
            return Err(EngineError::IllegalAction(
                "the payment does not cover what is owed",
            ));
        }
        self.run_until_choice();
        if !self.library_announcement_open() {
            self.library_action_tops = None;
        }
        Ok(())
    }

    /// Opens an arbitrary payment's mana opportunity before automation or
    /// a host can observe the suspended numeric choice.
    pub(super) fn open_variable_mana_window(&mut self) {
        let Some(resolve::AwaitingOp::ManaForDamage { player, .. }) =
            self.resolution.as_ref().and_then(|r| r.awaiting.as_ref())
        else {
            return;
        };
        let player = *player;
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        let suspended = self.resolution.take().expect("payment suspended");
        self.mana_window = Some(PaymentWindow {
            player,
            suspended: PaymentContinuation::Tax(Box::new(suspended)),
        });
        self.pending = Pending::Priority {
            player,
            legal: Box::new(legal),
        };
        self.awaiting_answer = true;
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
    pub(super) fn can_settle_tax(&self, res: &crate::resolve::Resolution) -> bool {
        match &res.awaiting {
            Some(crate::resolve::AwaitingOp::PlayerMayPay {
                player,
                cost,
                outcome,
            }) => {
                let life = match outcome {
                    resolve::ManaPaymentOutcome::ManaAndLife { life, .. } => *life,
                    _ => 0,
                };
                resolve::mana_life_payment::life_payable(&self.state, *player, life)
                    && self.can_pay_mana(*player, resolve::payment_for(&self.state, res), cost)
            }
            _ => false,
        }
    }

    /// Whether `player`'s pool pays a tax of `cost` the way
    /// `resume_tax_choice` will pay it.
    ///
    /// Asked of the payment and not of the pool's total, because the total
    /// includes units whose restriction may forbid a tax (CR 106.6).
    /// Powerstone mana is permitted; creature-spell-only mana is not. A pool of restricted mana
    /// passed the total, was told it had paid, and tripped the assertion in
    /// `resume_tax_choice` (the refusal sweep, 2026-09-29) — and a seat
    /// holding it was never offered the window to make the mana it lacked.
    ///
    /// And of the payment rather than of the pool's size for a second
    /// reason since the price may have colour in it: two floating red do
    /// not pay Phantasmal Forces' `{U}`.
    #[cfg(test)]
    pub(super) fn pool_pays_tax(
        &self,
        player: PlayerId,
        cost: &baylee_core::mana::ManaCost,
    ) -> bool {
        self.can_pay_mana(player, casting::SpendFor::Other, cost)
    }

    /// Ends a payment window and settles the payment it was opened for, and
    /// says how it settled ([`WindowClose`]).
    ///
    /// The player is taken at their word only as far as their pool goes: a
    /// window they leave short pays nothing and takes the effect's other
    /// branch, which is the same outcome as declining and is what the card
    /// prints. Passing takes that outcome; `PlayerAction::PayWindow` refuses
    /// it and is undone whole.
    pub(super) fn close_mana_window(&mut self) -> Result<WindowClose, EngineError> {
        // "The player plays that card if able" (Word of Command): while the
        // commanded card's price can still be paid and the pool does not
        // pay it yet, closing the window gives up a card the player is able
        // to play — an illegal attempt (CR 732.1), refused.
        if self.commanded_payment_feasible() == Some(true) && !self.commanded_pool_completes() {
            return Err(EngineError::IllegalAction(
                "the commanded card can still be paid",
            ));
        }
        // Total rather than asserted. The only caller is the pass arm, which
        // has already matched on this window standing open, and a window with
        // no resolution in it is now unrepresentable — so there is nothing
        // here left to be wrong about. The `expect` this replaces was not
        // decoration: it is what caught #167, where a mana ability that asked
        // a colour had taken the slot this resolution was waiting in.
        let Some(window) = self.mana_window.take() else {
            return Ok(WindowClose::Open);
        };
        let res = match window.suspended {
            PaymentContinuation::LandMana(work) => {
                self.mana_window = Some(PaymentWindow {
                    player: window.player,
                    suspended: PaymentContinuation::LandMana(work),
                });
                return Ok(WindowClose::Open);
            }
            PaymentContinuation::Tax(res) => *res,
            PaymentContinuation::Activation(payment) => {
                let paid = self.finish_activation_payment(window.player, *payment);
                return Ok(WindowClose::settled(paid));
            }
            PaymentContinuation::GrantedAction { id, .. } => {
                let paid = self.state.take_granted_action(window.player, id);
                self.after_action(window.player);
                return Ok(WindowClose::settled(paid));
            }
            PaymentContinuation::Miracle {
                wizard,
                version,
                opened,
                ..
            } => {
                let paid = self.finish_miracle_payment(&wizard, version, &opened)?;
                return Ok(WindowClose::settled(paid));
            }
            PaymentContinuation::Pact { cost, source } => {
                let what = source.map_or(casting::SpendFor::Other, casting::SpendFor::OtherFrom);
                let paid =
                    casting::pay_mana_for(&mut self.state, window.player, what, &cost).is_some();
                if !paid {
                    let _ = sba::lose_by_effect(&mut self.state, window.player);
                }
                return Ok(WindowClose::settled(paid));
            }
            PaymentContinuation::AttackTax {
                attackers, opened, ..
            } => {
                let paid = self.settle_attack_tax(window.player, attackers, &opened);
                return Ok(WindowClose::settled(paid));
            }
            PaymentContinuation::BlockTax {
                defending,
                blockers,
                opened,
                ..
            } => {
                let paid = self.settle_block_tax(defending, &blockers, &opened);
                return Ok(WindowClose::settled(paid));
            }
            // The cast pays out of the pool as any cast does; one it cannot
            // pay is not made, and the card stays where it is (CR 601.2h
            // reverses a casting that cannot be paid), and what the window
            // made for it is given back (CR 732.1).
            PaymentContinuation::Cast {
                card,
                version,
                cost: _,
                then_no_more_spells,
                any_mana,
                opened,
            } => {
                let start = (*opened).clone();
                let paid = self
                    .start_paid_cast(
                        window.player,
                        card,
                        version,
                        then_no_more_spells,
                        any_mana,
                        opened,
                    )
                    .is_ok();
                if !paid {
                    self.give_back_window(window.player, &start);
                }
                return Ok(WindowClose::settled(paid));
            }
        };
        let paid = self.can_settle_tax(&res);
        Ok(self.settle_tax_resolution(res, paid))
    }

    fn settle_tax_resolution(&mut self, mut res: resolve::Resolution, paid: bool) -> WindowClose {
        if let Some(resolve::AwaitingOp::ManaForDamage { player, amount }) = res.awaiting {
            self.prompt_damage_payment(res, player, amount);
            return WindowClose::Paid;
        }
        match resolve::resume_tax_choice(&mut self.state, &mut res, paid) {
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
        WindowClose::settled(paid)
    }

    fn prompt_damage_payment(
        &mut self,
        mut res: resolve::Resolution,
        player: PlayerId,
        amount: u32,
    ) {
        res.awaiting = Some(resolve::AwaitingOp::DamagePayment { player, amount });
        self.pending = Pending::ChooseNumber {
            player,
            min: 0,
            max: casting::spendable_units(
                &self.state,
                player,
                resolve::payment_for(&self.state, &res),
            ),
            reason: crate::choice::NumberPrompt::ManaPayment {
                preventable_damage: amount,
            },
        };
        self.resolution = Some(res);
        self.awaiting_answer = true;
    }

    /// Where a window `player` is about to pay in begins
    /// ([`WindowStart`]).
    pub(super) fn window_start(&self, player: PlayerId) -> WindowStart {
        WindowStart {
            journal: self.state.journal.len(),
            pool: self.state.players[usize::from(player.get())]
                .mana_pool
                .clone(),
            constrained: self.state.constrained_payments.clone(),
            triggers: self.trigger_queue.len(),
        }
    }

    /// Gives back what a payment window made when it closes without the
    /// cast it was opened for: the play was never completed, and each
    /// player may reverse the legal mana abilities they activated while
    /// making it (CR 732.1). Returns whether it did.
    ///
    /// The owner wants the reversal, so it is taken for the player. What it
    /// puts back is exactly what a mana ability that taps for mana does:
    /// every permanent tapped as a cost since the window opened is untapped,
    /// the pool and the generated-mana obligations are what they were, and
    /// the triggered abilities the taps queued are dropped, because no
    /// ability triggers from an action that is undone (CR 732.1). Every
    /// mana ability of the window is reversed together, so none of them
    /// paid for one that stays, which is the one reversal 732.1 forbids.
    ///
    /// A window in which anything else happened is left as it stands: a
    /// sacrificed Lotus Petal cannot be put back as the object it was
    /// (CR 400.7), life paid or damage dealt by a mana ability is not a tap,
    /// and 732.1 forbids reversing anything that touched a library. CR 732.1
    /// lets the player reverse, it does not make them, so leaving those is
    /// a legal answer and not a half-reversal: nothing is given back at all.
    pub(super) fn give_back_window(&mut self, player: PlayerId, opened: &WindowStart) -> bool {
        let Some(made) = self.state.journal.entries().get(opened.journal..) else {
            return false;
        };
        let mut tapped: SmallVec<[ObjectId; 8]> = SmallVec::new();
        for entry in made {
            match entry.event {
                GameEvent::ObjectTapped {
                    object,
                    cause: Cause::Cost,
                } => tapped.push(object),
                GameEvent::ManaProduced { player: into, .. } if into == player => {}
                GameEvent::AbilityActivated {
                    player: activator,
                    mana_ability: true,
                    ..
                } if activator == player => {}
                _ => return false,
            }
        }
        for object in tapped {
            if self
                .state
                .object(object)
                .is_some_and(|o| o.zone == Zone::Battlefield)
                && self.state.restore_tapped(object, false)
            {
                self.state.journal.record(GameEvent::ObjectUntapped {
                    object,
                    cause: Cause::Cost,
                });
            }
        }
        self.state.players[usize::from(player.get())].mana_pool = opened.pool.clone();
        self.state
            .constrained_payments
            .clone_from(&opened.constrained);
        self.trigger_queue.truncate(opened.triggers);
        true
    }
}

/// How a payment window settled as it closed ([`Engine::close_mana_window`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum WindowClose {
    /// The payment it was opened for was made.
    Paid,
    /// It closed short: the effect's other branch, a reversed cast or
    /// activation, a Pact's loss.
    Short,
    /// It is still open (a land-mana instruction's window, which passing
    /// does not end).
    Open,
}

impl WindowClose {
    const fn settled(paid: bool) -> Self {
        if paid { Self::Paid } else { Self::Short }
    }
}
