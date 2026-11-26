//! A recorded game picked up again where its record ends: the client's own
//! table after the client restarted (an update, `docs/client.md`
//! §"Restarting into an update").
//!
//! [`replay`](crate::record::replay) builds a bare engine; a table needs the
//! whole session around it — the log every seat was told, its question
//! counters, what policies answered — so this applies each recorded input
//! through the same steps a live answer takes ([`Session::replayed`]),
//! minus the recorder, which is handed the record's position at the end so
//! it goes on writing where the record stops. A hosted game is resumed the
//! same way after its server restarted
//! (`docs/proposals/games-survive-restarts.md`), so what the table says
//! beside the inputs is made again too: chair changes, lost and returned
//! connections, hand sharing, and where the log had been sent.

use super::{Session, clock_answer};
use crate::record::{self, ChairChange, ConnectionChange, Line, Recorder, ReplayError, Source};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::HouseAnswer;

impl<D: baylee_engine::engine::LoopDetection, R: crate::session::SessionRegistry> Session<D, R> {
    /// The game `record` holds, rebuilt input by input and checked against
    /// every digest the record wrote, as a recorded session that goes on
    /// writing after the record's last line.
    ///
    /// The house's answers are replayed from the record, not asked again:
    /// an agent asked anew could answer differently (its stream restarts
    /// here), and the engine would leave the record. What the house plays
    /// from here on is its own.
    /// A challenge instead has a fixed policy: every recorded house answer is
    /// recomputed and compared, restoring the agent's memory before play resumes.
    ///
    /// The game's description ([`Session::describe`]) is not in the record,
    /// which names no one: the caller gives it again.
    ///
    /// Checked as [`record::replay`] checks: every engine hash on the build
    /// that played the game, and on another build the public table where
    /// the engine hash moved.
    ///
    /// # Errors
    /// As [`record::replay`]: a line that does not parse, an input the
    /// engine refuses, or state the record's digests do not hold.
    pub fn resume_with_detection(record: &[u8]) -> Result<Self, ReplayError> {
        let mut lines = record::lines_of(record);
        let head = record::header_of(&mut lines)?;
        let mut checker = record::Checker::for_build(&head.build);
        let mut session = Self::with_detection(&head.preset).ok_or(ReplayError::Unbuildable)?;
        checker.check(&session.engine, None, head.hash, head.seen.as_deref())?;
        let mut next = 0;
        let mut ended = false;
        for line in lines {
            match line? {
                Line::Header { .. } => return Err(ReplayError::NoHeader),
                Line::Input {
                    n,
                    at,
                    seat,
                    by,
                    action,
                    hash,
                    seen,
                    sealed,
                } => {
                    session.seal_as_recorded(sealed);
                    session.tell_time(at);
                    if !session.replayed(PlayerId::new(seat), by, action) {
                        return Err(ReplayError::Refused { n });
                    }
                    checker.check(&session.engine, Some(n), hash, seen.as_deref())?;
                    next = n + 1;
                }
                Line::End { n, .. } => {
                    if !matches!(session.engine.pending(), Pending::GameOver(_)) {
                        return Err(ReplayError::NotOver { n });
                    }
                    ended = true;
                    next = n + 1;
                }
                said => next = session.said_again(&said)?,
            }
        }
        session.earn_what_was_told();
        session.record = Some(Recorder::resumed(next, ended));
        Ok(session)
    }

    /// What the table said beside the inputs, said again in its place: a
    /// chair change, a lost or returned connection, a seat's word about its
    /// hand. The number of the line after it.
    fn said_again(&mut self, line: &Line) -> Result<u64, ReplayError> {
        let (n, taken) = match *line {
            Line::Chair {
                n,
                at,
                seat,
                change,
                sealed,
            } => {
                self.seal_as_recorded(sealed);
                self.tell_time(at);
                (n, self.chair_again(PlayerId::new(seat), change))
            }
            Line::Connection {
                n,
                at,
                seat,
                change,
                sealed,
            } => {
                self.seal_as_recorded(sealed);
                self.tell_time(at);
                let seat = PlayerId::new(seat);
                let taken = match change {
                    ConnectionChange::Lost { wait_secs } => self.connection_lost(seat, wait_secs),
                    ConnectionChange::Back => {
                        self.connection_back(seat);
                        true
                    }
                };
                (n, taken)
            }
            // Taken when it was said, so taken again here: a setting the
            // table refuses now is not this game's.
            Line::Setting {
                n,
                at,
                seat,
                setting,
                sealed,
            } => {
                self.seal_as_recorded(sealed);
                self.tell_time(at);
                (n, self.seat_setting(PlayerId::new(seat), setting).is_ok())
            }
            // What a seat's client declared holds for the record alone.
            Line::DeclaredMind { n, .. } => (n, true),
            Line::Header { .. } | Line::Input { .. } | Line::End { .. } => {
                return Err(ReplayError::NoHeader);
            }
        };
        if taken {
            Ok(n + 1)
        } else {
            Err(ReplayError::Refused { n })
        }
    }

    /// Seals the log as far as it had been sent when the line was written,
    /// so what follows folds into a repeat only where it folded live. A
    /// line from before the record noted it leaves the log as it is.
    fn seal_as_recorded(&mut self, sealed: Option<u32>) {
        if let Some(sealed) = sealed {
            self.log.seal(sealed as usize);
        }
    }

    /// A recorded change of who answers a chair, made again. `false` when
    /// it does not take, which a chair that changed live always did.
    fn chair_again(&mut self, seat: PlayerId, change: ChairChange) -> bool {
        match change {
            ChairChange::TakenOver => self.take_over(seat),
            ChairChange::Released => self.release(seat),
            ChairChange::StoodIn => self.stand_in(seat),
            ChairChange::HandedBack => self.hand_back(seat),
        }
    }

    /// One recorded input, applied as the live door that produced it
    /// applied it ([`Session::answer`] for a seat or its clock,
    /// `apply_house_action` for the house), without recording it again.
    /// `false` when the engine refuses it.
    pub(super) fn replayed(&mut self, player: PlayerId, by: Source, action: PlayerAction) -> bool {
        // A record naming a seat the table lacks is not this game: refused
        // here, before the session indexes anything by it.
        if usize::from(player.get()) >= self.seats.len() {
            return false;
        }
        let moves_the_game = !action.is_automation_setting();
        let deciding = self.deciding();
        let clock =
            (by == Source::Clock).then(|| clock_answer(self.engine.pending_for(player), &action));
        let asked = if self.is_challenge() {
            let Some(asked) = self.challenge_replay_apply(player, by, &action) else {
                return false;
            };
            asked
        } else {
            let asked = self.answering(player, &action);
            if self.engine.apply(player, action).is_err() {
                return false;
            }
            asked
        };
        self.challenge_answered(player, by);
        if moves_the_game
            && by == Source::Seat
            && let Some(window) = self.policy_acts.get_mut(player.get() as usize)
        {
            window.clear();
        }
        self.log_answer(player, &asked, deciding, clock);
        self.seq += 1;
        if moves_the_game {
            self.moved(player, deciding);
            if let Some(slot) = self.house_answered.get_mut(player.get() as usize) {
                *slot = match by {
                    Source::Clock => Some(HouseAnswer::Clock),
                    Source::StandIn => Some(HouseAnswer::StandIn),
                    Source::Seat | Source::House => None,
                };
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::preset::{GamePreset, SeatController};

    fn table(seed: u64) -> GamePreset {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/acceptance-decks.txt"
        ))
        .expect("acceptance deck file");
        let allytifact =
            baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact loads");
        let victory =
            baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory loads");
        let mut preset = baylee_cards::decks::preset_for(seed, &allytifact, &victory);
        preset.seats[0].controller = SeatController::Open;
        preset
    }

    /// Plays `answers` of the player's answers against the house and
    /// returns the session and everything it recorded.
    fn played(seed: u64, answers: u32) -> (Session, Vec<u8>) {
        let me = PlayerId::new(0);
        let mut session =
            Session::new_recorded(&table(seed), record::this_build()).expect("the table builds");
        session.describe("local".into(), vec!["You".into(), "House".into()]);
        session.tell_time(5_000);
        session.pump();
        let mut record = session.take_record();
        for i in 0..answers {
            if matches!(session.pending(), Pending::GameOver(_)) {
                break;
            }
            session.tell_time(5_000 + u64::from(i));
            let action = session.house_action(me).expect("the player is asked");
            session.act(me, action).expect("the answer stands");
            record.extend(session.take_record());
        }
        (session, record)
    }

    /// The point of the whole thing: the resumed table is the table, its
    /// log is the log every seat was told, it asks the same question, and
    /// the record goes on from where it stopped so that the whole of it
    /// still replays.
    #[test]
    fn a_resumed_game_is_the_game_it_was_and_its_record_goes_on() {
        let me = PlayerId::new(0);
        let (mut live, record) = played(11, 25);
        let mut resumed = Session::resume_recorded(&record).expect("the record resumes");
        resumed.describe("local".into(), vec!["You".into(), "House".into()]);
        assert_eq!(resumed.snapshot_hash(), live.snapshot_hash());
        assert_eq!(
            format!("{:?}", resumed.pending()),
            format!("{:?}", live.pending())
        );
        assert_eq!(resumed.seq(), live.seq(), "the same number of frames");
        assert_eq!(resumed.decision_seq(), live.decision_seq());
        assert_eq!(
            resumed.log.told(me, 0, resumed.log.len()),
            live.log.told(me, 0, live.log.len()),
            "the log the player reads after the restart is the one they read before"
        );

        // Both go on alike, and the resumed record is the old one continued.
        let mut whole = record.clone();
        for _ in 0..5 {
            if matches!(live.pending(), Pending::GameOver(_)) {
                break;
            }
            let action = live.house_action(me).expect("asked");
            live.act(me, action.clone()).expect("stands");
            resumed
                .act(me, action)
                .expect("stands after the restart too");
            whole.extend(resumed.take_record());
        }
        let replayed = record::replay(&whole).expect("old and new record are one record");
        assert_eq!(replayed.engine.snapshot_hash(), resumed.snapshot_hash());
    }

    /// Two people at one table, with what a hosted game adds beside the
    /// inputs: a lost connection that came back, a chair the house held and
    /// handed back, a clock's answer. Every seat of the resumed table is
    /// shown what it was shown before: its view and question, its whole log,
    /// its roster and print table, and who the table waits for. A seat shown
    /// less would see a different game after a server restart; a seat shown
    /// more would see what was hidden from it.
    #[test]
    fn every_seat_of_a_resumed_table_is_shown_what_it_was_shown() {
        use crate::session::SeatKind;
        let (a, b) = (PlayerId::new(0), PlayerId::new(1));
        let mut preset = table(21);
        preset.seats[1].controller = SeatController::Open;
        let names = || vec!["Ada".to_owned(), "Bo".to_owned()];
        let mut live =
            Session::new_recorded(&preset, record::this_build()).expect("the table builds");
        live.describe("g".into(), names());
        let mut now = 5_000;
        live.tell_time(now);
        let _ = live.pump();
        let mut record = live.take_record();
        for step in 0..60_u32 {
            now += 1_000;
            live.tell_time(now);
            match step {
                5 => assert!(live.connection_lost(b, Some(180))),
                9 => assert!(live.stand_in(b)),
                15 => assert!(live.hand_back(b)),
                18 => assert!(live.connection_lost(a, None)),
                19 => live.connection_back(a),
                _ => {}
            }
            let _ = live.pump();
            record.extend(live.take_record());
            if matches!(live.pending(), Pending::GameOver(_)) {
                break;
            }
            let Some(seat) = live.awaiting_seat() else {
                continue;
            };
            if matches!(live.seat_kind(seat), Some(SeatKind::StandIn(_))) {
                continue;
            }
            if step == 22
                && let Some(asked) = live.asked_at(seat)
                && let Some(answered) = live.answer_by_clock(seat, asked)
            {
                answered.expect("the clock answers");
            } else {
                let action = live.house_action(seat).expect("the seat is asked");
                live.act(seat, action).expect("the answer stands");
            }
            record.extend(live.take_record());
        }
        record.extend(live.take_record());

        let mut resumed = Session::resume_recorded(&record).expect("the record resumes");
        resumed.describe("g".into(), names());
        resumed.tell_time(now);
        assert_eq!(resumed.snapshot_hash(), live.snapshot_hash());
        assert_eq!(resumed.decision_seq(), live.decision_seq());
        assert_eq!(resumed.seq(), live.seq(), "the same number of frames");
        assert_eq!(
            format!("{:?}", resumed.lost_seats()),
            format!("{:?}", live.lost_seats()),
            "the table waits for the same players"
        );
        for seat in [a, b] {
            assert_eq!(
                resumed.log.told(seat, 0, resumed.log.len()),
                live.log.told(seat, 0, live.log.len()),
                "seat {seat:?} reads the same log"
            );
            assert_eq!(
                format!("{:?}", resumed.view_for(seat)),
                format!("{:?}", live.view_for(seat)),
                "seat {seat:?} sees the same table and question"
            );
            let (now, then) = (resumed.game_static(seat), live.game_static(seat));
            assert_eq!(now.seats, then.seats, "seat {seat:?} has the same roster");
            // Earned as frames went out live, and from the log and the view
            // after a resume: never a printing the seat was not shown, and
            // every one its log and its table name.
            for (at, (now, then)) in now.prints.iter().zip(&then.prints).enumerate() {
                assert!(
                    now.is_none() || now == then,
                    "seat {seat:?} was never shown printing {at}"
                );
            }
            let told = baylee_view::LogTail {
                from: 0,
                entries: resumed.log.told(seat, 0, resumed.log.len()),
            };
            let view = resumed.seat_view(seat);
            for print in told.prints().chain(view.prints()) {
                assert!(
                    now.prints
                        .get(print.get() as usize)
                        .is_some_and(Option::is_some),
                    "seat {seat:?} can draw printing {print:?} of its log and table"
                );
            }
        }
    }

    /// A record that lost a step is not resumed into a different game: the
    /// hashes say so (red if a line goes missing on the way to the disk).
    #[test]
    fn a_record_missing_a_step_does_not_resume() {
        let (_, record) = played(12, 10);
        let lines: Vec<&[u8]> = record.split_inclusive(|b| *b == b'\n').collect();
        let input = lines
            .iter()
            .position(|l| l.starts_with(br#"{"kind":"input""#))
            .expect("an input");
        let mut torn = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if i != input + 2 {
                torn.extend_from_slice(line);
            }
        }
        assert!(matches!(
            Session::resume_recorded(&torn),
            Err(ReplayError::Diverged { .. } | ReplayError::Refused { .. })
        ));
        assert!(matches!(
            Session::resume_recorded(b"not a record\n"),
            Err(ReplayError::Unreadable { .. })
        ));
    }

    /// On the build that played the game a resume is held to every engine
    /// hash: one changed hash is a divergence even where the public table
    /// beside it still agrees. The same record from another build resumes by
    /// that table.
    #[test]
    fn a_changed_hash_resumes_only_where_another_build_wrote_it() {
        let (live, record) = played(13, 8);
        let mut written = record::tests::lines(&record);
        let at = written
            .iter()
            .rposition(|l| matches!(l, Line::Input { .. }))
            .expect("an input");
        if let Line::Input { hash, .. } = &mut written[at] {
            *hash = record::hex(0);
        }
        assert!(matches!(
            Session::resume_recorded(&record::tests::encode(&written)),
            Err(ReplayError::Diverged { .. })
        ));
        if let Line::Header { build, .. } = &mut written[0] {
            *build = "0.0.0+build.0 (another)".into();
        }
        let resumed = Session::resume_recorded(&record::tests::encode(&written))
            .expect("another build's record resumes by its public table");
        assert_eq!(resumed.snapshot_hash(), live.snapshot_hash());
    }
}
