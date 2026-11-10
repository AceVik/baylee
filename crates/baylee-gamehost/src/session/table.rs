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
use baylee_ai::intelligence::DeckIntel;
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_core::preset::{AIProfile, AiHandSharePolicy, HandSharingRule};
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
    /// The table's rule for showing a teammate a hand.
    pub(super) hand_sharing: HandSharingRule,
    /// How the house's chairs show their hands to teammates.
    pub(super) ai_hand_share_policy: AiHandSharePolicy,
}

/// Whether the chair `kind` is read teammates' hands it was shown: a seat
/// answered over a socket or by an external model, and a chair the house
/// plays at a level that plays as a team (`AIProfile::TEAM_PLAY`). A frozen
/// reference is shown none, so a benchmark table plays as it was frozen.
pub(super) fn reads_shared_hands(kind: &SeatKind) -> bool {
    match kind {
        SeatKind::Human | SeatKind::Driven(_) => true,
        SeatKind::Ai(agent) | SeatKind::StandIn(agent) => {
            agent.profile().plays(AIProfile::TEAM_PLAY)
        }
    }
}

/// Whether the chair `kind` is the house playing as a team.
fn house_on_a_team(kind: Option<&SeatKind>) -> bool {
    matches!(kind, Some(SeatKind::Ai(agent)) if agent.profile().plays(AIProfile::TEAM_PLAY))
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
    /// - **No teammate's hand** here. This builder adds none; a hand a
    ///   teammate shows is added by [`Self::show_shared_hands`] for the
    ///   chairs that read one (`house_answer`, `view_for`, `observe_view`)
    ///   and by `Session::show_hands` on the way to a socket, each under the
    ///   table's rule. A clock's answer for a player is given from this view
    ///   alone.
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

    /// The view the house answers in `player`'s chair from: [`Self::agent_view`]
    /// and the hands its teammates show it, where its level plays as a
    /// team ([`Self::show_shared_hands`]).
    pub(super) fn house_view(&self, player: PlayerId, pending: &Pending) -> PlayerView {
        let mut view = self.agent_view(player, pending);
        self.show_shared_hands(&mut view, player);
        view
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

    /// Populates `view.shared_hands` with the teammates' hands `seat` is
    /// shown, continuously or as a one-time snapshot, where its chair reads
    /// them ([`reads_shared_hands`]).
    ///
    /// Read afresh for every view, against everything that makes a share
    /// legal: the table's rule (nothing at all under
    /// [`HandSharingRule::Disabled`], no continuous share but under
    /// [`HandSharingRule::Allowed`]), a team both seats are on, and both
    /// still in a game that is still going. So an opponent's hand is never
    /// added, and a share ends the moment its owner leaves or loses.
    ///
    /// Two house chairs on one side show each other their hands without
    /// asking, as [`AiHandSharePolicy`] has the house share continuously
    /// ([`Self::house_shows_house`]); a person's hand reaches the house only
    /// where the person shared it.
    fn show_shared_hands(&self, view: &mut PlayerView, seat: PlayerId) {
        if self.hand_sharing == HandSharingRule::Disabled {
            return;
        }
        let at = usize::from(seat.get());
        if !self.seats.get(at).is_some_and(reads_shared_hands) {
            return;
        }
        let state = self.engine.state();
        if state.has_left(seat) || matches!(self.engine.pending(), Pending::GameOver(_)) {
            return;
        }
        let Some(team) = self.teams.get(at).copied().flatten() else {
            return;
        };
        let continuous = self.hand_sharing == HandSharingRule::Allowed;
        let mut added = false;
        for (i, _) in self.seats.iter().enumerate() {
            if i == at {
                continue;
            }
            let owner = PlayerId::new(seat_byte(i));
            if self.teams.get(i).copied().flatten() != Some(team) || state.has_left(owner) {
                continue;
            }
            if continuous
                && (self.showing.get(i).is_some_and(|s| s.contains(seat))
                    || self.house_shows_house(i, at))
            {
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

    /// Whether the house chair at `owner` shows its hand to the house chair
    /// at `viewer`, both on one side: where both play as a team and the
    /// house shares continuously. Asking is implied: a house chair asks a
    /// house teammate the moment it could, and is answered as a person who
    /// asked would be (`Session::ai_answer_hand_request`), so the share is
    /// read here rather than kept.
    fn house_shows_house(&self, owner: usize, viewer: usize) -> bool {
        house_on_a_team(self.seats.get(owner))
            && house_on_a_team(self.seats.get(viewer))
            && matches!(
                self.ai_hand_share_policy,
                AiHandSharePolicy::AlwaysContinuous | AiHandSharePolicy::OnRequestContinuous
            )
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
    /// separately (`scouting::request`): it denies a human's stand-in, tells
    /// a fair level its own deck alone, and only a level that peeks more. A
    /// table that says it is fair (`Session::set_fair_scouting`) holds even
    /// that one to its own deck.
    pub(super) fn house_answer(&self, player: PlayerId, pending: &Pending) -> Option<PlayerAction> {
        let (SeatKind::Ai(agent) | SeatKind::StandIn(agent)) =
            self.seats.get(usize::from(player.get()))?
        else {
            return None;
        };
        let view = self.house_view(player, pending);
        let context = self.engine.decision_context();
        let scouting = agent.scouting_request(pending).and_then(|mut request| {
            if self.fair_scouting {
                request = crate::scouting::own_deck_only(request);
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
