use super::{Asked, POLICY_WINDOW, SeatKind, Session, choice_envelope};
use crate::record::Source;
use baylee_core::ids::{PlayerId, SeatSet};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::event::{GameEvent, PolicyAnswer as EnginePolicyAnswer};
use baylee_engine::zone::ZoneLocation;
use baylee_protocol::v1::Envelope;
use baylee_view::{ClockAnswer, HouseAnswer, LogEvent, PolicyAct, PolicyAnswer};

impl Session {
    /// Whether the house still owes an answer after a bounded pump. A local
    /// host must keep pumping instead of waiting for input from a human who
    /// has not received a question yet.
    #[must_use]
    pub fn needs_pump(&self) -> bool {
        self.engine.awaited().iter().any(|seat| {
            !self
                .seats
                .get(seat.get() as usize)
                .is_some_and(SeatKind::answers_over_socket)
                && self.engine.pending_for(seat).is_some()
        })
    }

    /// Drains AI-controlled pendings, then returns per-seat envelopes:
    /// a fresh hidden-information view for every human seat plus a
    /// choice request for every seat being asked (or game over for
    /// everyone). Capped so an all-AI game can never hang the server.
    pub fn pump(&mut self) -> Vec<(PlayerId, Envelope)> {
        self.pump_at_most(4096)
    }

    /// [`Session::pump`], answering at most `answers` house questions before
    /// it returns.
    ///
    /// For a host that plays whole games with nobody at the table (the
    /// self-play runner in `baylee-train`): it reads the wall clock between
    /// calls, so it can stop a game whose every answer has become slow
    /// instead of waiting out a full `pump`. A call that runs out of answers
    /// while the house still owes another step returns no envelopes; if the
    /// `answers`-th step hands the decision to a socket seat or ends the
    /// game, its views and choice request are returned immediately.
    pub fn pump_at_most(&mut self, answers: usize) -> Vec<(PlayerId, Envelope)> {
        let mut out = Vec::new();
        let mut answered = 0;
        loop {
            let awaited = self.engine.awaited();
            if awaited.is_empty() {
                let pending = self.engine.pending().clone();
                if let Pending::GameOver(_) = &pending {
                    for seat in self.human_seats() {
                        let envelopes = self.view_envelopes(seat);
                        out.extend(envelopes.into_iter().map(|env| (seat, env)));
                        out.push((seat, choice_envelope(self.seq, &pending)));
                    }
                    self.tell_spectators(&mut out);
                }
                return out;
            }
            // The house answers first, in seat order: during the opening
            // mulligans several seats are asked at once, and a human who is
            // still deciding must not hold up an AI chair's keep.
            let house_seat = awaited.iter().find(|seat| {
                let socket = self
                    .seats
                    .get(seat.get() as usize)
                    .is_some_and(SeatKind::answers_over_socket);
                !socket && self.engine.pending_for(*seat).is_some()
            });
            let Some(player) = house_seat else {
                for seat in self.human_seats() {
                    let envelopes = self.view_envelopes(seat);
                    out.extend(envelopes.into_iter().map(|env| (seat, env)));
                }
                for seat in awaited.iter() {
                    if let Some(pending) = self.engine.pending_for(seat) {
                        out.push((seat, choice_envelope(self.seq, pending)));
                    }
                }
                self.tell_spectators(&mut out);
                return out;
            };
            if answered >= answers {
                return out;
            }
            let Some(pending) = self.engine.pending_for(player).cloned() else {
                return out;
            };
            let action = match &self.seats[player.get() as usize] {
                // Both receive a filtered view. Scouting checks the live
                // seat kind separately and denies a human's stand-in.
                SeatKind::Ai(agent) | SeatKind::StandIn(agent) => {
                    let view = self.agent_view(player, &pending);
                    let context = self.engine.decision_context();
                    let scouting = agent.scouting_request(&pending).and_then(|mut request| {
                        if self.fair_scouting {
                            request.opponents = false;
                            request.hands = false;
                            request.library = baylee_ai::intelligence::LibraryAccess::None;
                        }
                        crate::scouting::request(
                            &self.seats,
                            &self.scouting_decks,
                            self.engine.state(),
                            player,
                            request,
                        )
                    });
                    scouting.as_ref().map_or_else(
                        || agent.act_with_context(&view, &pending, &context),
                        |report| agent.act_with_scouting(&view, &pending, &context, report),
                    )
                }
                // Both answer over a socket, so `pump` returned above.
                SeatKind::Human | SeatKind::Driven(_) => unreachable!(),
            };
            let by = self.seats[player.get() as usize]
                .is_away()
                .then_some(HouseAnswer::StandIn);
            let deciding = self.deciding();
            let moves_the_game = self.apply_house_action(player, action);
            self.seq += 1;
            if moves_the_game {
                self.moved(player, deciding);
                self.house_answered[player.get() as usize] = by;
            }
            answered += 1;
        }
    }

    /// Runs at most `answers` house steps and always emits incremental view
    /// and log envelopes for every socket seat and spectator, even when the
    /// house still owes further steps ([`Session::needs_pump`]).
    ///
    /// While `needs_pump()` remains true, no [`v1::ChoiceRequest`] is sent for
    /// a seat whose turn to answer has not arrived yet (except during
    /// simultaneous opening decisions where a human seat already has a pending
    /// question, which `pump_at_most` answers only after the AI seats have
    /// finished theirs). This lets an engine server or background worker slice
    /// long AI turns across event-loop ticks, streaming live [`v1::StateDelta`]
    /// frames and answering control frames without ever blocking the link.
    pub fn pump_slice(&mut self, answers: usize) -> Vec<(PlayerId, Envelope)> {
        let out = self.pump_at_most(answers);
        if out.is_empty() && answers > 0 && self.needs_pump() {
            return self.progress_envelopes();
        }
        out
    }

    /// Intermediate per-seat and spectator views and log lines while the house
    /// is still stepping between human decisions.
    pub fn progress_envelopes(&mut self) -> Vec<(PlayerId, Envelope)> {
        let mut out = Vec::new();
        for seat in self.human_seats() {
            let envelopes = self.view_envelopes(seat);
            out.extend(envelopes.into_iter().map(|env| (seat, env)));
        }
        self.tell_spectators(&mut out);
        out
    }

    /// An invalid agent proposal must not leave an untimed seat stalled (#180).
    pub(super) fn apply_house_action(&mut self, player: PlayerId, action: PlayerAction) -> bool {
        let moves = !action.is_automation_setting();
        let deciding = self.deciding();
        let asked = self.answering(player, &action);
        let by = if self.seats[player.get() as usize].is_away() {
            Source::StandIn
        } else {
            Source::House
        };
        let refused = action.clone();
        let kept = self.record.is_some().then(|| action.clone());
        self.last_attempt = Some((player, by, action.clone()));
        let applied = self.engine.apply(player, action);
        self.last_attempt = None;
        let error = match applied {
            Ok(()) => {
                if let Some(action) = kept {
                    self.recorded(player, by, action);
                }
                self.log_answer(player, &asked, deciding, None);
                return moves;
            }
            Err(error) => error,
        };
        // A refused answer leaves the engine as it was, so the question the
        // proposal was refused on is still the one standing, and asking the
        // house again would only propose the same answer to it. The answer
        // that does nothing comes first (a miracle refused for its cost is
        // declined, CR 601.2), and the house only where the question has no
        // such answer. This used to re-ask the house outright, which was
        // right only while a refusal could move the game: a refused miracle
        // spent its offer, the house was then asked about the priority that
        // followed, and that answer went into the record in place of an
        // answer to the miracle — a record no replay could follow.
        //
        // A seat with no question standing was asked nothing before the
        // proposal either (the refusal moved nothing), so nobody waits on it
        // and nothing stalls. This used to `expect` a question here, and a
        // concession for a chair whose duel the concession before it had
        // ended aborted the engine process and the game with it
        // (22.10.2026). The host's journal says so now; the game and its
        // record stay as they were.
        let Some(pending) = self.engine.pending_for(player) else {
            tracing::error!(
                seat = player.get(),
                action = ?refused,
                %error,
                game_over = matches!(self.engine.pending(), Pending::GameOver(_)),
                "the house answered for a seat that was asked nothing; the answer is dropped",
            );
            return false;
        };
        let fallback = baylee_engine::choice::timeout_answer(pending)
            .filter(|answer| *answer != refused)
            .or_else(|| self.house_action(player))
            .expect("refused AI action left no decision");
        let asked = self.answering(player, &fallback);
        let kept = self.record.is_some().then(|| fallback.clone());
        self.last_attempt = Some((player, by, fallback.clone()));
        self.engine.apply(player, fallback).expect(
            "both AI proposal and recovery were refused; refusing to silently stall the table",
        );
        self.last_attempt = None;
        if let Some(action) = kept {
            self.recorded(player, by, action);
        }
        self.log_answer(player, &asked, deciding, None);
        true
    }

    /// What the log needs to know of `player`'s answer before it is spent.
    pub(super) fn answering(&self, player: PlayerId, action: &PlayerAction) -> Asked {
        Asked {
            hand: self
                .engine
                .state()
                .zones
                .list(ZoneLocation::Hand(player))
                .len(),
            took: matches!(action, PlayerAction::MulliganTake),
            bottomed: match action {
                PlayerAction::ChooseObjects { objects } => objects.len(),
                _ => 0,
            },
        }
    }

    /// Writes into the log what `player`'s answer did: what a clock answered
    /// in their place, a mulligan or a keep, and then everything the journal
    /// recorded while it was applied.
    ///
    /// `deciding` is [`Session::deciding`] before the answer. A seat that
    /// leaves it by answering, and has not left the game, has kept: its hand
    /// as it answered, less what the answer put on the bottom.
    pub(super) fn log_answer(
        &mut self,
        player: PlayerId,
        asked: &Asked,
        deciding: SeatSet,
        clock: Option<ClockAnswer>,
    ) {
        if let Some(answer) = clock {
            self.log.note(LogEvent::TimedOut { player, answer });
        }
        if asked.took {
            self.log.note(LogEvent::Mulliganed { player });
        } else if deciding.contains(player)
            && !self.deciding().contains(player)
            && !self.engine.state().has_left(player)
        {
            let cards = asked.hand.saturating_sub(asked.bottomed);
            self.log.note(LogEvent::Kept {
                player,
                cards: u8::try_from(cards).unwrap_or(u8::MAX),
            });
        }
        self.log.consume(self.engine.state());
        self.read_policy_acts();
    }

    /// Takes what each seat's policies answered for it off the journal and
    /// into its window (#234).
    fn read_policy_acts(&mut self) {
        let state = self.engine.state();
        let entries = state.journal.entries();
        for entry in entries.get(self.policy_read..).unwrap_or_default() {
            let GameEvent::AutoAnswered {
                player,
                ability,
                object,
                answer,
            } = entry.event
            else {
                continue;
            };
            let seat = player.get() as usize;
            let (Some(window), Some(counted)) = (
                self.policy_acts.get_mut(seat),
                self.policy_counted.get_mut(seat),
            ) else {
                continue;
            };
            *counted += 1;
            if window.len() == POLICY_WINDOW {
                window.remove(0);
            }
            window.push(PolicyAct {
                number: *counted,
                ability: crate::log::policy_ability(state, object, ability, player),
                answer: match answer {
                    EnginePolicyAnswer::Passed => PolicyAnswer::Passed,
                    EnginePolicyAnswer::Yes => PolicyAnswer::Yes,
                    EnginePolicyAnswer::No => PolicyAnswer::No,
                },
            });
        }
        self.policy_read = entries.len();
    }
}
