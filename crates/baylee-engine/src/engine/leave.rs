//! A player leaving the game while a question is on the table (CR 800.4a).
//!
//! A concession is legal at any moment and from any seat (CR 104.3a), so
//! it often comes from a seat that is not the one being asked. What happens
//! to the question depends on where it lives.
//!
//! - **Priority, and the step machine's own questions** (attackers,
//!   blockers, the cleanup discard): the machine runs and asks them again
//!   from the board the leaver left behind. A priority held by anybody else
//!   stays with them (#275).
//! - **A question held in a slot above the machine**: a spell being cast
//!   ([`Engine::cast_wizard`]), an activation or a trigger waiting for its
//!   targets ([`Engine::pending_plan`]), a resolution waiting for a choice
//!   ([`Engine::resolution`]), or a payment window opened over one
//!   ([`Engine::mana_window`]). Nobody has priority in any of these
//!   (CR 117.2e; CR 601.2i gives the caster priority only once the spell
//!   *is* cast), so the machine must not run in between. None of them can
//!   be asked again from the board either. Running the machine once dropped
//!   the cast and handed priority to the next seat, and started a
//!   resolving spell over from its first instruction, so Brainstorm drew
//!   six (#277). The question stays, less what named the leaver or what
//!   they owned.
//! - **The leaver's own question in such a slot**: a cast, an activation or
//!   a trigger of theirs that has not reached the stack goes with them
//!   (CR 800.4a; a trigger is never put on the stack, CR 800.4d). A
//!   resolution goes on without them, whoever controls it (CR 608.2m), and
//!   what it asked of them is neither chosen nor paid (CR 800.4f).

use super::{Engine, PlanKind, resolve, sba};
use crate::choice::{Pending, PlayerAction};
use crate::event::LossReason;
use crate::resolve::{AwaitingOp, Flow, Resolution};
use crate::state::CardLookup;
use baylee_core::ids::{ObjectId, PlayerId};

impl<L: CardLookup> Engine<L> {
    /// `player` concedes (CR 104.3a).
    pub(crate) fn concede(&mut self, player: PlayerId) {
        if !self.holds_a_question() {
            self.concede_to_the_machine(player);
            return;
        }
        let asked = self.pending.asked();
        // A pass the leaver made earlier in the round is not a pass by a
        // player still in the game (CR 117.4), as in #275's path. The round
        // is only paused under this question: a refused draw offer hands it
        // back exactly as it was.
        if let Some(holder) = self.priority_holder
            && holder != player
            && self.passed_before(player, holder)
        {
            self.passes -= 1;
        }
        sba::eliminate_player(&mut self.state, player, LossReason::Conceded);
        // CR 104.2a: the game is over the moment the last opponent leaves,
        // whatever was being asked.
        if let Some(result) = self.game_result() {
            self.end_game(result);
            return;
        }
        if asked == Some(player) {
            self.drop_the_leavers_question(player);
        } else if !self.forget_the_departed() {
            self.cannot_comply();
        }
    }

    /// Whether the question on the table lives in a slot above the machine,
    /// which running the machine would drop rather than ask again.
    ///
    /// A priority counts only inside a payment window. Anywhere else it
    /// belongs to the round, which #275's path answers.
    fn holds_a_question(&self) -> bool {
        match self.pending {
            Pending::Priority { .. } => self.mana_window.is_some(),
            _ => {
                self.cast_wizard.is_some()
                    || self.pending_plan.is_some()
                    || self.resolution.is_some()
            }
        }
    }

    /// Leaving while the round or the step machine is asking. The machine
    /// runs after it: the game may be over, and the board the leaver's
    /// objects left behind is owed its state-based actions before anybody is
    /// asked anything (CR 117.5).
    fn concede_to_the_machine(&mut self, player: PlayerId) {
        // A priority the leaver held passes to the next player still in the
        // game, which the round does on its own (CR 800.4a). One held by
        // anybody else stays with them (#275): it is asked again, with what
        // is legal now. A pass the leaver had already made is no longer one
        // of the passes in succession the round waits for, because it is
        // not a pass by a player still in the game (CR 117.4).
        if let Pending::Priority { player: holder, .. } = self.pending
            && holder != player
        {
            if self.passed_before(player, holder) {
                self.passes -= 1;
            }
            self.regrant_priority = Some(holder);
        }
        sba::eliminate_player(&mut self.state, player, LossReason::Conceded);
        self.awaiting_answer = false;
        self.run_until_choice();
    }

    /// Takes out of the question on the table every object that has left
    /// the game and every player who has. Returns `false` when a target
    /// choice is left with fewer options than it must name.
    ///
    /// A choice of cards keeps what is left and asks for no more than that:
    /// a player can't choose what is impossible (CR 608.2d).
    pub(super) fn forget_the_departed(&mut self) -> bool {
        let state = &self.state;
        let here = |id: &ObjectId| state.object(*id).is_some();
        let playing = |p: &PlayerId| !state.players[usize::from(p.get())].has_lost();
        if let Some(PlanKind::DrawOffer { remaining, .. }) = &mut self.pending_plan {
            remaining.retain(playing);
        }
        match &mut self.pending {
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                ..
            } => {
                options.retain(here);
                player_options.retain(playing);
                return options.len() + player_options.len() >= usize::from(*min);
            }
            Pending::ChooseCards {
                options, min, max, ..
            } => {
                options.retain(here);
                let n = u8::try_from(options.len()).unwrap_or(u8::MAX);
                *min = (*min).min(n);
                *max = (*max).min(n);
            }
            Pending::LegendChoice { options, .. } => options.retain(here),
            Pending::ChoosePlayer { options, .. } => options.retain(playing),
            Pending::Arrange { cards, piles, .. } => {
                cards.retain(here);
                // Every card still goes somewhere, and no pile asks for more
                // than is left once the piles before it have their least.
                let total = u32::try_from(cards.len()).unwrap_or(u32::MAX);
                let mut left = total;
                for pile in piles {
                    pile.min = pile.min.min(left);
                    pile.max = pile.max.min(total);
                    left -= pile.min;
                }
            }
            _ => {}
        }
        true
    }

    /// A target choice that has lost the options it needed.
    fn cannot_comply(&mut self) {
        // A spell being cast: the casting is illegal, and the game returns
        // to the moment before it was proposed (CR 601.2). The wizard's own
        // failure path does exactly that when its target stage finds too
        // few, and gives the caster the priority they held.
        if self.cast_wizard.is_some() {
            let _ = self.advance_cast_wizard();
            return;
        }
        match self.pending_plan.take() {
            // An activation, likewise (CR 602.2b). Its targets come before
            // its costs, so nothing has been paid.
            Some(
                PlanKind::ActivateAbility { .. } | PlanKind::ActivateAbilitySecondTargets { .. },
            ) => {
                let activator = self.pending.asked().expect("a target choice asks somebody");
                self.pending = Pending::Priority {
                    player: activator,
                    legal: Box::new(self.compute_legal(activator)),
                };
            }
            // A triggered ability with no legal choice is removed from the
            // stack (CR 603.3d). A queued one is still the queue's front, and
            // the machine asks for its targets again and removes it itself; a
            // synthetic one is carried by its plan, already off the queue, and
            // goes with the plan.
            Some(PlanKind::Trigger { .. } | PlanKind::SyntheticTriggerTarget { .. }) => {
                self.awaiting_answer = false;
                self.run_until_choice();
            }
            // A resolution's own target question (new targets for a spell,
            // CR 115.7) asks for what is left, as a choice of cards does.
            other => {
                self.pending_plan = other;
                if let Pending::ChooseTargets {
                    options,
                    player_options,
                    min,
                    ..
                } = &mut self.pending
                {
                    let n = options.len() + player_options.len();
                    *min = u8::try_from(n).unwrap_or(u8::MAX).min(*min);
                }
            }
        }
    }

    /// The leaver was the one being asked.
    fn drop_the_leavers_question(&mut self, player: PlayerId) {
        // A draw needs the agreement of the players in the game (CR 104.4i),
        // and they are no longer one of them: the offer goes on to the next.
        if let Some(PlanKind::DrawOffer { remaining, .. }) = &mut self.pending_plan {
            let state = &self.state;
            remaining.retain(|p| !state.players[usize::from(p.get())].has_lost());
            self.awaiting_answer = false;
            let _ = self.apply_inner(player, PlayerAction::YesNo(true));
            self.run_until_choice();
            return;
        }
        // Their own spell being cast, or their activation or trigger waiting
        // on an answer, has not reached the stack, and goes with them. A
        // trigger waiting on its mode or its targets is the queue's front,
        // and is never put on the stack (CR 800.4d); a synthetic one is
        // carried by the plan itself.
        let plan = self.pending_plan.take();
        if let Some(PlanKind::Trigger { .. } | PlanKind::ModalTrigger { .. }) = plan {
            self.trigger_queue.pop_front();
        }
        let their_own = plan.is_some() | self.cast_wizard.take().is_some();
        // A payment window asks only its payer, and they pay nothing
        // (CR 800.4f). A colour one of their mana abilities was asking in
        // the meantime goes with them (#167 put it in the resolution slot).
        if let Some(window) = self.mana_window.take() {
            self.resolution = None;
            let mut res = *window.suspended;
            let flow = resolve::resume_tax_choice(&mut self.state, &mut res, false);
            self.go_on_with(res, flow);
            return;
        }
        // A resolution goes on without them, their own included: a spell
        // that leaves the stack once it has started to resolve still
        // resolves fully (CR 608.2m), and what it does to a player who has
        // left reads their last known information (CR 800.4i). What it
        // asked of them is neither chosen nor paid (CR 800.4f).
        if !their_own && let Some(res) = self.resolution.take() {
            self.decline_for_the_departed(res);
            return;
        }
        self.awaiting_answer = false;
        self.run_until_choice();
    }

    /// Resumes `res` past the question its departed chooser was asked, as
    /// if they had chosen nothing and paid nothing.
    ///
    /// Everything they owned has left the game, so a choice among their
    /// cards or permanents has nothing left to name, and the chains that ask
    /// player after player skip one with nothing to pick. An optional
    /// clause is not taken, and cards looked at stay where they are. A
    /// question with no empty answer (a colour, a number, a player) ends the
    /// resolution there: nobody else can make it for them.
    fn decline_for_the_departed(&mut self, mut res: Resolution) {
        let state = &mut self.state;
        let flow = match (&self.pending, &res.awaiting) {
            (_, None) => Flow::Complete,
            (Pending::YesNo { .. }, Some(AwaitingOp::PlayerMayPay { .. })) => {
                resolve::resume_tax_choice(state, &mut res, false)
            }
            (Pending::YesNo { .. }, Some(AwaitingOp::MayDo { .. })) => {
                resolve::resume_may_do(state, &mut res, false)
            }
            (Pending::YesNo { .. }, _) => resolve::resume_yes_no(state, &mut res, false),
            (Pending::ChooseCards { .. }, _) => resolve::resume(state, &mut res, &[]),
            (Pending::ChooseTargets { .. }, _) => {
                resolve::resume_targets(state, &mut res, &[], &[])
            }
            (Pending::Arrange { piles, .. }, _) => {
                resolve::resume_arranged(state, &mut res, &vec![Vec::new(); piles.len()])
            }
            _ => Flow::Complete,
        };
        self.go_on_with(res, flow);
    }

    /// Where a resumed resolution goes next: its next question, or the end.
    fn go_on_with(&mut self, res: Resolution, flow: Flow) {
        self.awaiting_answer = false;
        match flow {
            Flow::Wait(pending) => {
                self.resolution = Some(res);
                self.pending = pending;
                self.awaiting_answer = true;
            }
            Flow::Complete => self.finish_resolution(&res),
        }
        self.run_until_choice();
    }
}
