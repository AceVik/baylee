//! Game construction and the readiness barrier before play begins.
use super::{
    Barrier, EngineRunner, Envelope, GamePreset, LoopDetection, PlayerId, SeatSet, Session,
    SessionRegistry, curtain, ended, error, seat_frames, v1,
};

/// The most a resumed record may unpack to. A two-seat game writes 100–190
/// KiB; the gateway keeps at most 64 MiB of a record compressed.
const MAX_RESUMED_BYTES: u64 = 256 * 1024 * 1024;

/// The session a stored record holds, for a game whose server restarted
/// (`docs/proposals/games-survive-restarts.md`): unpacked within
/// [`MAX_RESUMED_BYTES`], played again by [`Session::resume_for_execution`]
/// on exactly the artifact that played it, and still going. A ranked duel
/// resumes through Central's journal, never here.
fn resumed<D: LoopDetection, R: SessionRegistry>(
    packed: &[u8],
    identity: &baylee_protocol::execution::Identity,
    preset: &GamePreset,
) -> Result<Session<D, R>, &'static str> {
    use std::io::Read as _;
    if preset.house_rules.ranked_duel {
        return Err("a ranked duel is not resumed from its record");
    }
    let mut record = Vec::new();
    flate2::read::MultiGzDecoder::new(packed)
        .take(MAX_RESUMED_BYTES + 1)
        .read_to_end(&mut record)
        .map_err(|_| "the stored record does not unpack")?;
    if record.len() as u64 > MAX_RESUMED_BYTES {
        return Err("the stored record is too large to resume");
    }
    let session = Session::resume_for_execution(&record, identity)
        .map_err(|_| "the stored record does not resume on this engine")?;
    if matches!(
        session.pending(),
        baylee_engine::choice::Pending::GameOver(_)
    ) {
        return Err("the stored record is of a game that is over");
    }
    Ok(session)
}

impl<D: LoopDetection, R: SessionRegistry, T: crate::recording::Transport> EngineRunner<D, R, T> {
    /// Builds the game, or picks it up again from its record
    /// (`GameSetup.resume_record`).
    pub(super) fn setup(&mut self, setup: &v1::GameSetup) -> Vec<Envelope> {
        if self.session.is_some() {
            return Vec::new();
        }
        let Ok(preset) = serde_json::from_slice::<GamePreset>(&setup.preset_json) else {
            return vec![ended(&setup.game_id, "the preset did not decode")];
        };
        let Some(identity) = baylee_protocol::execution::Identity::from_metadata(&setup.execution)
        else {
            return vec![ended(&setup.game_id, "invalid engine execution identity")];
        };
        let resuming = !setup.resume_record.is_empty();
        let built = if resuming {
            resumed::<D, R>(&setup.resume_record, &identity, &preset)
        } else {
            Session::recorded_for_execution(&preset, &identity)
                .ok_or("the preset does not make a game")
        };
        let mut session = match built {
            Ok(session) => session,
            Err(why) => return vec![ended(&setup.game_id, why)],
        };
        // These read one frame at a time; see `Session::tell_views_whole`.
        #[cfg(test)]
        session.tell_views_whole();
        // The levels came by name from the gateway, each with its tier and
        // bound; what this host adds is its own share of them.
        if let Some(host) = &self.foresight {
            session.set_foresight(true);
            session.set_foresight_host(host.clone());
        }
        // Which chairs an account sits in: the house searches only while one
        // of them plays, not for a seat bridge whose host has gone.
        if !setup.seat_accounts.is_empty() {
            session.mark_people(&setup.seat_accounts);
        }
        session.describe(setup.game_id.clone(), setup.seat_names.clone());
        session.mark_guests(setup.seat_guests.clone());
        session.tell_time(self.now);
        self.game_id.clone_from(&setup.game_id);
        self.ranked_duel = preset.house_rules.ranked_duel;
        if resuming {
            // The table was open before its server restarted: no curtain, the
            // record goes on at the next piece, and a player whose connection
            // the record says was lost is still lost here, so their return is
            // told. Every other seat is simply absent until it dials again.
            self.record_seq = setup.resume_pieces;
            self.record_sent_at = self.now;
            self.record_waiting_since = None;
            self.lost = session
                .lost_seats()
                .iter()
                .map(|lost| lost.seat.get())
                .collect();
            self.session = Some(session);
            return Vec::new();
        }
        // The header is waiting from here.
        self.record_sent_at = self.now;
        self.record_waiting_since = Some(self.now);
        // Down until every seat that answers over a socket has drawn its
        // table. An expired preparation fails explicitly. An AI chair is
        // ready from the start and never holds the barrier.
        let waits_for: SeatSet = session.human_seats().into_iter().collect();
        self.curtain = (!waits_for.is_empty()).then_some(Barrier {
            waits_for,
            ready: SeatSet::new(),
        });
        self.session = Some(session);
        Vec::new()
    }

    /// Opens the table once, after the scheduled entrance. Also available
    /// to in-process harnesses that deliberately bypass presentation.
    ///
    /// The house plays its chairs in the pump that follows, which is the
    /// first moment anything is decided, and every attached seat is sent
    /// what that pump produced and then [`curtain`], last, so a client opens
    /// on the table as it stands afterwards. A seat that attaches later is
    /// sent the same at the end of its own batch. The clocks start here too,
    /// because this is the first moment [`EngineRunner::clocks`] names any.
    pub fn raise_curtain(&mut self) -> Vec<Envelope> {
        if self.curtain.take().is_none() {
            return Vec::new();
        }
        self.enter_at = None;
        // The clocks exist from here on, and the views below are the first
        // to carry them.
        self.absorb_clock();
        let budget = self.pump_budget;
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let routed = session.pump_slice(budget);
        let mut out = self.route(&routed);
        out.extend(seat_frames(self.attached.iter().copied(), &curtain()));
        out.extend(self.ending());
        out
    }

    /// Scheduled arrival, at which play and its clocks may begin.
    #[must_use]
    pub fn entrance_deadline(&self) -> Option<u64> {
        self.enter_at
            .map(|at| at.saturating_add(baylee_protocol::TABLE_ENTRANCE_MS))
    }

    /// Called by the transport timer. Early and cancelled timers do nothing.
    pub fn finish_entrance(&mut self) -> Vec<Envelope> {
        if self.entrance_deadline().is_some_and(|at| self.now >= at) {
            self.raise_curtain()
        } else {
            Vec::new()
        }
    }

    /// Preparation failed. Never start decision clocks for an unready seat.
    pub fn preparation_expired(&mut self) -> Vec<Envelope> {
        if self.curtain.is_none() || self.enter_at.is_some() {
            return Vec::new();
        }
        self.ended = true;
        let reason = "The table could not start: not every player finished loading. Return to the lobby and try again.";
        let mut out: Vec<_> = seat_frames(self.attached.iter().copied(), &error(reason)).collect();
        out.push(ended(&self.game_id, reason));
        out
    }

    pub(super) fn loading_status(&self) -> Vec<Envelope> {
        let Some(barrier) = self.curtain else {
            return Vec::new();
        };
        let status = Envelope {
            msg: Some(v1::envelope::Msg::TableLoading(v1::TableLoading {
                ready: barrier.ready.iter().count() as u32,
                total: barrier.waits_for.iter().count() as u32,
                enter_at_ms: self.enter_at.unwrap_or(0),
            })),
        };
        seat_frames(self.attached.iter().copied(), &status).collect()
    }

    /// Count only attached human seats, once. All ready schedules a flight;
    /// it does not pump the game or start a decision clock.
    pub(super) fn seat_ready(&mut self, player: PlayerId) -> Vec<Envelope> {
        let Some(barrier) = self.curtain.as_mut() else {
            return Vec::new();
        };
        if !self.attached.contains(&player.get())
            || !barrier.waits_for.contains(player)
            || barrier.ready.contains(player)
        {
            return Vec::new();
        }
        barrier.ready.insert(player);
        if barrier.met() {
            self.enter_at = Some(
                self.now
                    .saturating_add(baylee_protocol::TABLE_ENTRANCE_LEAD_MS),
            );
        }
        self.loading_status()
    }
}
