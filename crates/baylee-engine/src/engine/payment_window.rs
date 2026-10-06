//! The CR 605.3a payment window: a seat that has said it will pay makes
//! its mana here, one mana ability at a time, and passing settles the
//! payment the window was opened for ([`super::PaymentContinuation`]).

use super::{
    CardLookup, Cause, Engine, EngineError, GameEvent, ObjectId, PaymentContinuation,
    PaymentWindow, Pending, PlayerId, SmallVec, WindowStart, Zone, casting, resolve, sba,
};

impl<L: CardLookup> Engine<L> {
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
        match res.awaiting {
            Some(crate::resolve::AwaitingOp::PlayerMayPay { player, cost, .. }) => {
                self.pool_pays_tax(player, &cost)
            }
            _ => false,
        }
    }

    /// Whether `player`'s pool pays a tax of `cost` the way
    /// `resume_tax_choice` will pay it.
    ///
    /// Asked of the payment and not of the pool's total, because the total
    /// counts mana that says "spend this only on…" (CR 106.6), and a tax is
    /// none of the things it may be spent on. A pool of restricted mana
    /// passed the total, was told it had paid, and tripped the assertion in
    /// `resume_tax_choice` (the refusal sweep, 2026-09-29) — and a seat
    /// holding it was never offered the window to make the mana it lacked.
    ///
    /// And of the payment rather than of the pool's size for a second
    /// reason since the price may have colour in it: two floating red do
    /// not pay Phantasmal Forces' `{U}`.
    pub(super) fn pool_pays_tax(
        &self,
        player: PlayerId,
        cost: &baylee_core::mana::ManaCost,
    ) -> bool {
        casting::affordable(
            &self.state,
            player,
            &self.state.players[player.get() as usize].mana_pool,
            cost,
        )
    }

    /// Ends a payment window and settles the payment it was opened for.
    ///
    /// The player is taken at their word only as far as their pool goes: a
    /// window they leave short pays nothing and takes the effect's other
    /// branch, which is the same outcome as declining and is what the card
    /// prints.
    pub(super) fn close_mana_window(&mut self) -> Result<(), EngineError> {
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
            return Ok(());
        };
        let mut res = match window.suspended {
            PaymentContinuation::LandMana(work) => {
                self.mana_window = Some(PaymentWindow {
                    player: window.player,
                    suspended: PaymentContinuation::LandMana(work),
                });
                return Ok(());
            }
            PaymentContinuation::Tax(res) => *res,
            PaymentContinuation::Activation(payment) => {
                self.finish_activation_payment(window.player, *payment);
                return Ok(());
            }
            PaymentContinuation::GrantedAction { id, .. } => {
                self.state.take_granted_action(window.player, id);
                self.after_action(window.player);
                return Ok(());
            }
            PaymentContinuation::Miracle {
                wizard,
                version,
                opened,
                ..
            } => {
                return self.finish_miracle_payment(&wizard, version, &opened);
            }
            PaymentContinuation::Pact(cost) => {
                if !casting::pay_mana(&mut self.state, window.player, &cost) {
                    let _ = sba::lose_by_effect(&mut self.state, window.player);
                }
                return Ok(());
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
                opened,
            } => {
                let start = (*opened).clone();
                if self
                    .start_paid_cast(window.player, card, version, then_no_more_spells, opened)
                    .is_err()
                {
                    self.give_back_window(window.player, &start);
                }
                return Ok(());
            }
        };
        let paid = self.can_settle_tax(&res);
        if let Some(resolve::AwaitingOp::ManaForDamage { player, amount }) = res.awaiting {
            res.awaiting = Some(resolve::AwaitingOp::DamagePayment { player, amount });
            self.pending = Pending::ChooseNumber {
                player,
                min: 0,
                max: casting::spendable_units(&self.state, player, casting::SpendFor::Other),
                reason: crate::choice::NumberPrompt::ManaPayment {
                    preventable_damage: amount,
                },
            };
            self.resolution = Some(res);
            self.awaiting_answer = true;
            return Ok(());
        }
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
        Ok(())
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
                _ => return false,
            }
        }
        for object in tapped {
            if self
                .state
                .object(object)
                .is_some_and(|o| o.zone == Zone::Battlefield)
                && self.state.set_tapped(object, false)
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
