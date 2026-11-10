//! What an agent is shown of a table and how the house answers at it, read
//! off one engine and its chairs.
//!
//! One rule wherever a game is played: a hosted [`Session`] and the
//! [`SimulationSession`]s forked from it build every agent's view and every
//! house answer here, so a lookahead plays the house exactly as the table
//! does and shows the searching seat exactly what it would be shown.
//!
//! [`Session`]: super::Session
//! [`SimulationSession`]: super::SimulationSession

use super::{RegistryLookup, SeatKind, seat_byte};
use baylee_ai::HeuristicAgent;
use baylee_ai::intelligence::{DeckIntel, LibraryAccess};
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_view::{HouseAnswer, PlayerView};
use std::collections::BTreeMap;

/// A table as the house and its agents read it, borrowed from whichever
/// session holds it.
pub(super) struct Table<'a> {
    pub(super) engine: &'a Engine<RegistryLookup>,
    pub(super) seats: &'a [SeatKind],
    pub(super) teams: &'a [Option<u8>],
    pub(super) scouting_decks: &'a [DeckIntel],
    pub(super) fair_scouting: bool,
    pub(super) seq: u64,
    pub(super) house_answered: &'a [Option<HouseAnswer>],
    pub(super) showing: &'a [SeatSet],
    pub(super) shown_once: &'a BTreeMap<(u8, u8), Vec<ObjectId>>,
}

impl Table<'_> {
    /// The view the house answers `player`'s question from, whether it plays
    /// the chair or has only been handed one decision by a clock.
    ///
    /// One builder for both, because what an agent may know is one rule.
    /// Two things a socket's view carries never reach it:
    ///
    /// - **No clock.** `decision_remaining_ms` is made of elapsed wall time,
    ///   and machine speed is not an authorized input to a decision: an
    ///   agent that read it would play the same position differently on a
    ///   slow machine, legally and invisibly (#87). A clock's answer is
    ///   where it would bite first, since that view exists because the
    ///   number ran out.
    /// - **No teammate's hand** (#265). Only `Session::show_hands` puts one
    ///   into a view, on the way to a socket, and this never calls it, so a
    ///   chair the house holds for an absent player is played from what it
    ///   was played from before hands could be shown.
    pub(super) fn agent_view(&self, player: PlayerId, pending: &Pending) -> PlayerView {
        let engine = self.engine;
        crate::view::player_view(
            engine.state(),
            player,
            self.seq,
            Some(pending),
            &crate::view::SeatContext {
                awaiting: crate::view::awaiting_for(engine, player),
                controls: &crate::view::player_controls(engine),
                decision_player: pending.asked(),
                controlled_players: engine.controlled_players(player),
                deciding: crate::view::deciding(engine),
                held: engine.automation(player).hold.suppresses(),
                owed: crate::view::owed_payment(engine),
                library_reveal_blocked: engine.library_reveal_blocked(),
                inspected_cards: engine.inspected_cards(),
                decision_remaining_ms: None,
                // What a policy answered is told to the seat's player; it is
                // not an input to a decision.
                policy_acts: &[],
                ballots: engine.ballots(),
            },
            self.house_answered,
        )
    }

    /// `seat`'s question and the view it answers from
    /// ([`Session::view_for`](super::Session::view_for)).
    pub(super) fn view_for(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        let pending = self.engine.pending_for(seat)?.clone();
        let mut view = self.agent_view(seat, &pending);
        self.show_shared_hands(&mut view, seat);
        Some((pending, view))
    }

    /// What `seat` may observe of the table now, asked or not
    /// ([`Session::observe_view`](super::Session::observe_view)).
    pub(super) fn observe_view(&self, seat: PlayerId) -> PlayerView {
        let engine = self.engine;
        let mut view = crate::view::player_view(
            engine.state(),
            seat,
            self.seq,
            engine.information_pending_for(seat),
            &crate::view::SeatContext {
                awaiting: crate::view::awaiting_for(engine, seat),
                controls: &crate::view::player_controls(engine),
                decision_player: crate::view::decision_player_for(engine, seat),
                controlled_players: engine.controlled_players(seat),
                deciding: crate::view::deciding(engine),
                held: engine.automation(seat).hold.suppresses(),
                owed: crate::view::owed_payment(engine),
                library_reveal_blocked: engine.library_reveal_blocked(),
                inspected_cards: engine.inspected_cards(),
                decision_remaining_ms: None,
                policy_acts: &[],
                ballots: engine.ballots(),
            },
            self.house_answered,
        );
        self.show_shared_hands(&mut view, seat);
        view
    }

    /// Populates `view.shared_hands` with any teammate hands shared (continuously
    /// or as a one-time snapshot) with `seat`, provided `seat` is answered over a
    /// socket or driven by an external model (`SeatKind::answers_over_socket`).
    /// House heuristic chairs (`SeatKind::Ai` / `SeatKind::StandIn`) are never
    /// shown voluntary hand shares.
    fn show_shared_hands(&self, view: &mut PlayerView, seat: PlayerId) {
        if self.showing.iter().all(|s| s.is_empty()) && self.shown_once.is_empty() {
            return;
        }
        let at = usize::from(seat.get());
        if !self.seats.get(at).is_some_and(SeatKind::answers_over_socket) {
            return;
        }
        let state = self.engine.state();
        if state.has_left(seat) || matches!(self.engine.pending(), Pending::GameOver(_)) {
            return;
        }
        let Some(team) = self.teams.get(at).copied().flatten() else {
            return;
        };
        let mut added = false;
        for (i, _) in self.seats.iter().enumerate() {
            if i == at {
                continue;
            }
            let owner = PlayerId::new(seat_byte(i));
            if self.teams.get(i).copied().flatten() != Some(team) || state.has_left(owner) {
                continue;
            }
            if self.showing.get(i).is_some_and(|s| s.contains(seat)) {
                crate::view::show_whole_hand(state, view, owner);
                added = true;
            } else if let Some(ids) = self.shown_once.get(&(owner.get(), seat.get())) {
                crate::view::show_hand_snapshot(state, view, owner, ids);
                added = true;
            }
        }
        if added {
            view.shared_hands.sort_by_key(|hand| hand.player);
        }
    }

    /// `view` with the owner's current cast and targets.
    pub(super) fn with_choice_context(&self, seat: PlayerId, mut view: PlayerView) -> PlayerView {
        view.targeting = crate::view::targeting_context(self.engine, seat);
        view.trigger_order = crate::view::trigger_order(self.engine, seat);
        // The cast this seat may cancel: its own, or the one it is making
        // for the player whose turn it controls (CR 724.5).
        let caster = if self.engine.decision_actor() == Some(seat) {
            self.engine.pending().asked().unwrap_or(seat)
        } else {
            seat
        };
        view.casting = self.engine.cancellable_cast(caster);
        view
    }

    /// `view` with the owner's persistent announced payment source.
    pub(super) fn with_payment_context(&self, seat: PlayerId, mut view: PlayerView) -> PlayerView {
        view.announced_cast = self.engine.announced_cast(seat);
        view
    }

    /// `view` with the owner's announcement facts, settled X included.
    pub(super) fn with_announcement_context(
        &self,
        seat: PlayerId,
        mut view: PlayerView,
    ) -> PlayerView {
        if let Some((source, x)) = self.engine.announced_spell_context(seat) {
            view.announced_cast = Some(source);
            view.announced_x = x;
        }
        view
    }

    /// [`Self::view_for`] with choice context.
    pub(super) fn view_for_with_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.view_for(seat)
            .map(|(pending, view)| (pending, self.with_choice_context(seat, view)))
    }

    /// [`Self::observe_view`] with choice context.
    pub(super) fn observe_view_with_context(&self, seat: PlayerId) -> PlayerView {
        self.with_choice_context(seat, self.observe_view(seat))
    }

    /// [`Self::view_for_with_context`] with the announced payment source.
    pub(super) fn view_for_with_payment_context(
        &self,
        seat: PlayerId,
    ) -> Option<(Pending, PlayerView)> {
        self.view_for_with_context(seat)
            .map(|(pending, view)| (pending, self.with_payment_context(seat, view)))
    }

    /// [`Self::observe_view_with_context`] with the announced payment source.
    pub(super) fn observe_view_with_payment_context(&self, seat: PlayerId) -> PlayerView {
        self.with_payment_context(seat, self.observe_view_with_context(seat))
    }

    /// [`Self::view_for_with_payment_context`] with the announcement facts.
    pub(super) fn view_for_with_announcement_context(
        &self,
        seat: PlayerId,
    ) -> Option<(Pending, PlayerView)> {
        self.view_for_with_payment_context(seat)
            .map(|(pending, view)| (pending, self.with_announcement_context(seat, view)))
    }

    /// [`Self::observe_view_with_payment_context`] with the announcement
    /// facts.
    pub(super) fn observe_view_with_announcement_context(&self, seat: PlayerId) -> PlayerView {
        self.with_announcement_context(seat, self.observe_view_with_payment_context(seat))
    }

    /// What the agent sitting in `player`'s chair answers `pending` with, or
    /// `None` when the chair answers over a socket.
    ///
    /// The agent receives the filtered view; scouting checks the live chair
    /// separately (`scouting::request`) and denies a human's stand-in, and a
    /// fair table clamps every request to the agent's own deck.
    pub(super) fn house_answer(&self, player: PlayerId, pending: &Pending) -> Option<PlayerAction> {
        let (SeatKind::Ai(agent) | SeatKind::StandIn(agent)) =
            self.seats.get(usize::from(player.get()))?
        else {
            return None;
        };
        let view = self.agent_view(player, pending);
        let context = self.engine.decision_context();
        let scouting = agent.scouting_request(pending).and_then(|mut request| {
            if self.fair_scouting {
                request.opponents = false;
                request.hands = false;
                request.library = LibraryAccess::None;
            }
            crate::scouting::request(
                self.seats,
                self.scouting_decks,
                self.engine.state(),
                player,
                request,
            )
        });
        Some(scouting.as_ref().map_or_else(
            || agent.act_with_context(&view, pending, &context),
            |report| agent.act_with_scouting(&view, pending, &context, report),
        ))
    }

    /// What a default house agent answers for `player`
    /// ([`Session::house_action`](super::Session::house_action)).
    pub(super) fn house_action(&self, player: PlayerId) -> Option<PlayerAction> {
        let pending = self.engine.pending_for(player)?;
        // Teams included, as `stand_in` builds it: without them every other
        // chair reads as an enemy, the seat's own partner among them.
        let agent = HeuristicAgent::new(AIProfile::default()).with_teams(self.teams.to_vec());
        Some(agent.act(&self.agent_view(player, pending), pending))
    }

    /// What the house answers `pending`, `player`'s standing question,
    /// after the engine refused `refused` for it.
    ///
    /// The answer that does nothing first (a miracle refused for its cost is
    /// declined, CR 601.2), and the default house agent only where the
    /// question has no such answer, or that answer was the one refused.
    pub(super) fn refusal_fallback(
        &self,
        player: PlayerId,
        pending: &Pending,
        refused: &PlayerAction,
    ) -> Option<PlayerAction> {
        baylee_engine::choice::timeout_answer(pending)
            .filter(|answer| answer != refused)
            .or_else(|| self.house_action(player))
    }

    /// The seats that won `result`
    /// ([`Session::winning_seats`](super::Session::winning_seats)).
    pub(super) fn winning_seats(&self, result: baylee_engine::win::GameResult) -> Vec<PlayerId> {
        let Some(victor) = result.winner else {
            return Vec::new();
        };
        (0..self.seats.len())
            .filter_map(|i| u8::try_from(i).ok().map(PlayerId::new))
            .filter(|seat| {
                victor.includes(
                    *seat,
                    self.teams.get(usize::from(seat.get())).copied().flatten(),
                )
            })
            .collect()
    }
}
