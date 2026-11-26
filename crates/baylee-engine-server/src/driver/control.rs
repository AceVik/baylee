use crate::{
    Clock, EngineRunner, Envelope, LoopDetection, Session, SessionRegistry, ended, frames,
    seat_frames, v1,
};

impl<D: LoopDetection, R: SessionRegistry, T: crate::recording::Transport> EngineRunner<D, R, T> {
    /// Applies one frame from the gateway and returns what to send back.
    ///
    /// `remaining` is how much of each **currently armed** deadline is left,
    /// read from the caller's clocks a moment ago, and empty if the caller
    /// has nothing armed. It is a parameter rather than something the
    /// runner reads for itself because the `Instant` lives with whoever runs
    /// the timer, and it is a parameter rather than an optional setter
    /// because a caller that forgot it would produce views whose countdown
    /// silently restarts on every frame.
    ///
    /// It matters for exactly one shape, and that shape is the point of the
    /// ticket: a seat that **reconnects** in the middle of a question is sent
    /// a fresh snapshot, and must be shown the twenty seconds it has left
    /// rather than the ten minutes it started with. Every other frame either
    /// moves the game — in which case the question is new and the allowance
    /// is whole — or is answered the same way by both.
    pub fn handle(&mut self, envelope: Envelope, remaining: &[(Clock, u32)]) -> Vec<Envelope> {
        // A held game answers another flush and nothing else: no seat, no
        // setting, no close moves it until its link closes.
        if self.held {
            return match envelope.msg {
                Some(v1::envelope::Msg::FlushRecord(flush)) => self.flush_record(flush.nonce, true),
                _ => Vec::new(),
            };
        }
        self.set_readings(remaining);
        self.absorb_clock();
        match envelope.msg {
            Some(v1::envelope::Msg::GameSetup(setup)) => self.setup(&setup),
            Some(v1::envelope::Msg::SeatAttached(attached)) => self.attach(attached),
            Some(v1::envelope::Msg::SeatDetached(detached)) => self.detach(detached),
            Some(v1::envelope::Msg::SeatFrame(frame)) => self.seat_frame(&frame),
            Some(v1::envelope::Msg::SpectatorsChanged(changed)) => self.spectators_changed(changed),
            Some(v1::envelope::Msg::FlushRecord(flush)) => {
                self.flush_record(flush.nonce, flush.hold)
            }
            Some(v1::envelope::Msg::CloseGame(close)) => self.close(&close.reason),
            _ => Vec::new(),
        }
    }

    /// An admin ended the game from the console (`docs/protocol.md`
    /// §"Acting on a game"): every attached seat and the spectators are
    /// told why ([`v1::GameClosed`]), the rest of the record goes as its
    /// last piece, and then `GameEnded` with no winners. The rules are not
    /// asked: nobody won or lost, and the record ends where the game was
    /// closed. A game already over is left alone.
    pub(crate) fn close(&mut self, reason: &str) -> Vec<Envelope> {
        self.closed(reason, "closed by an admin")
    }

    /// [`Self::close`], with what `GameEnded` gives as the reason.
    pub(crate) fn closed(&mut self, reason: &str, ended_why: &str) -> Vec<Envelope> {
        if self.ended {
            return Vec::new();
        }
        self.ended = true;
        let closed = Envelope {
            msg: Some(v1::envelope::Msg::GameClosed(v1::GameClosed {
                reason: reason.to_owned(),
            })),
        };
        let mut out: Vec<Envelope> = seat_frames(self.attached.iter().copied(), &closed).collect();
        if self.spectators > 0 {
            out.push(frames::spectator_frame(&closed));
        }
        if let Some(data) = self.session.as_mut().map(Session::take_record) {
            out.extend(self.record_chunk(data, true));
        }
        let mut ending = ended(&self.game_id, ended_why);
        if let Some(v1::envelope::Msg::GameEnded(ended)) = ending.msg.as_mut() {
            let outcome = if matches!(
                ended_why,
                "challenge abandoned" | "challenge connection expired"
            ) {
                v1::challenge_result::Outcome::Abandoned
            } else {
                v1::challenge_result::Outcome::Void
            };
            ended.challenge = self.challenge_result(outcome);
            if outcome == v1::challenge_result::Outcome::Abandoned {
                ended.turn = self
                    .session
                    .as_ref()
                    .map_or(1, |session| session.state().turn.number.max(1));
            }
        }
        out.push(ending);
        out
    }
}
