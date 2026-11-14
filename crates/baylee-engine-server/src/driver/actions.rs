use crate::{
    EngineRunner, Envelope, LoopDetection, PlayerAction, PlayerId, SeatSetting, SessionRegistry,
    error, seat_frame, seat_frames, v1,
};

impl<D: LoopDetection, R: SessionRegistry> EngineRunner<D, R> {
    /// A player-facing frame from one seat.
    pub(crate) fn seat_frame(&mut self, frame: &v1::SeatFrame) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(frame.seat) else {
            return Vec::new();
        };
        let player = PlayerId::new(seat);
        // A frame in no chair's name (a spectator is none) says nothing:
        // a spectator answers nothing, and the rules are never asked.
        if self
            .session
            .as_ref()
            .is_some_and(|session| usize::from(seat) >= session.state().players.len())
        {
            return Vec::new();
        }
        let Ok(inner) = <Envelope as prost::Message>::decode(&frame.envelope[..]) else {
            return Vec::new();
        };
        match inner.msg {
            Some(v1::envelope::Msg::ClockProbe(probe)) => vec![seat_frame(
                seat,
                &Envelope {
                    msg: Some(v1::envelope::Msg::ClockProbe(v1::ClockProbe {
                        client_time_ms: probe.client_time_ms,
                        server_time_ms: self.now,
                    })),
                },
            )],
            Some(v1::envelope::Msg::SeatReady(_)) => self.seat_ready(player),
            Some(v1::envelope::Msg::AiLog(ai_log)) => self.reasoning(player, ai_log),
            // What answers the seat, as its client says, for the record
            // only (`docs/protocol.md` §"Who answers a seat, as it says").
            // Not a move: no view, pump or clock, and taken before the
            // curtain as after. A refused one is dropped without a word:
            // an `Error` on a seat's socket reads as its answer refused,
            // and the client sends only what passes the same check. Its
            // text is never logged.
            Some(v1::envelope::Msg::SeatMind(said)) => {
                if let Some(session) = self.session.as_mut()
                    && let Err(why) = session.declare_mind(player, &said)
                {
                    tracing::warn!(seat, why, "a declared mind was refused");
                }
                Vec::new()
            }
            // Dropped rather than refused before the curtain is up (#256): no
            // seat has been asked anything, so a correct client has nothing
            // to answer, and a refusal would reach it as a failure.
            Some(v1::envelope::Msg::PlayerAction(_)) if self.curtain.is_some() => {
                tracing::debug!(seat, "an action before the curtain went up; dropped");
                Vec::new()
            }
            Some(v1::envelope::Msg::PlayerAction(action_msg)) => {
                let Ok(action) = baylee_core::wire::decode::<PlayerAction>(&action_msg.action)
                else {
                    return Vec::new();
                };
                self.apply(player, action)
            }
            // What it missed, before the curtain, is the table it has not
            // been shown, and still no question.
            Some(v1::envelope::Msg::Resume(_)) if self.curtain.is_some() => {
                let Some(session) = self.session.as_mut() else {
                    return Vec::new();
                };
                if !session.may_retell(player) {
                    return Vec::new();
                }
                session
                    .show(player)
                    .iter()
                    .map(|env| seat_frame(seat, env))
                    .collect()
            }
            // Moves nothing: a seat asking for what it missed must not advance the
            // game, or reconnecting would play an AI seat's turn for it.
            Some(v1::envelope::Msg::Resume(resume)) => {
                let Some(session) = self.session.as_mut() else {
                    return Vec::new();
                };
                session
                    .resume(player, resume.last_seq)
                    .iter()
                    .map(|env| seat_frame(seat, env))
                    .collect()
            }
            // Not a move in the game (#265): the engine never sees it, so it
            // is not `apply`. A refusal is only said: the seat submitted no
            // answer, so it still holds its question and needs none re-sent.
            // Taken before the curtain is up as well: it is no decision and
            // runs no clock, and the views it changes go out as `show` builds
            // them, with no pump and no question.
            Some(v1::envelope::Msg::SeatSetting(setting_msg)) => {
                let Ok(setting) = baylee_core::wire::decode::<SeatSetting>(&setting_msg.setting)
                else {
                    return Vec::new();
                };
                let Some(session) = self.session.as_mut() else {
                    return Vec::new();
                };
                match session.seat_setting(player, setting) {
                    Ok(routed) => self.route(&routed),
                    Err(reason) => self.route(&[(player, error(&reason))]),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Forwards what a seat's mind said (`v1::AiLog`) to every other
    /// attached seat in a debug build, and drops it in a release build
    /// (`docs/protocol.md` §"An AI seat's reasoning").
    ///
    /// The AI log is a debugging and sparring tool, open to the whole table
    /// by the owner's decision: a model's reasoning reads its whole view out
    /// loud, its hand included, so a debug engine hands every seat hidden
    /// information on purpose, and a release engine, the one deployed,
    /// forwards nothing. Never back to the sender, and to no seat without a
    /// socket. The sender is stamped here, so a seat cannot speak in
    /// another's name; nothing else in it is read. Not a move in the game:
    /// no journal, no clock, no pump.
    pub(crate) fn reasoning(&self, from: PlayerId, mut said: v1::AiLog) -> Vec<Envelope> {
        if !cfg!(debug_assertions) || self.session.is_none() {
            return Vec::new();
        }
        said.seat = u32::from(from.get());
        let envelope = Envelope {
            msg: Some(v1::envelope::Msg::AiLog(said)),
        };
        seat_frames(
            self.attached
                .iter()
                .copied()
                .filter(|&seat| seat != from.get()),
            &envelope,
        )
        .collect()
    }

    /// Applies one action and routes everything it produced.
    pub(crate) fn apply(&mut self, player: PlayerId, action: PlayerAction) -> Vec<Envelope> {
        let budget = self.pump_budget;
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        match session.act_slice(player, action, budget) {
            Ok(routed) => {
                let mut out = self.route(&routed);
                out.extend(self.ending());
                out
            }
            Err(reason) => self.refused(player, &reason),
        }
    }

    /// Answers a refused action: why it was refused, and the question again.
    ///
    /// Silence here is what turns a wrong action into a fatal one. The client
    /// clears its `Interaction` the moment it submits (`flush_outbox` in
    /// `baylee-client`), so a seat told nothing is left with no question in
    /// front of it: every later key and click does nothing, and when the
    /// decision clock runs out the house agent plays that seat's turn — which
    /// looks, from the table, like the phase advancing on its own.
    ///
    /// Re-asking is safe because `Session::reask` is read-only: it never
    /// pumps, so it cannot take a turn on an AI seat's behalf. It also sends
    /// the choice only to the seat actually being awaited, so a refusal from a
    /// seat that does not hold the decision gets the error and nothing more.
    /// It sends no log: a refusal lost no frame, and the whole log for each
    /// refused action would be a large answer to a small request.
    pub(crate) fn refused(&mut self, player: PlayerId, reason: &str) -> Vec<Envelope> {
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let mut frames = vec![(player, error(reason))];
        frames.extend(session.reask(player).into_iter().map(|env| (player, env)));
        self.route(&frames)
    }
}
