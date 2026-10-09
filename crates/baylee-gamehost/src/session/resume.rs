//! A recorded game picked up again where its record ends: the client's own
//! table after the client restarted (an update, `docs/client.md`
//! §"Restarting into an update").
//!
//! [`replay`](crate::record::replay) builds a bare engine; a table needs the
//! whole session around it — the log every seat was told, its question
//! counters, what policies answered — so this applies each recorded input
//! through the same steps a live answer takes ([`Session::replayed`]),
//! minus the recorder, which is handed the record's position at the end so
//! it goes on writing where the record stops.

use super::{Session, clock_answer};
use crate::record::{self, Line, Recorder, ReplayError, Source};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::HouseAnswer;

impl Session {
    /// The game `record` holds, rebuilt input by input and checked against
    /// every hash the record wrote, as a recorded session that goes on
    /// writing after the record's last line.
    ///
    /// The house's answers are replayed from the record, not asked again:
    /// an agent asked anew could answer differently (its stream restarts
    /// here), and the engine would leave the record. What the house plays
    /// from here on is its own.
    ///
    /// The game's description ([`Session::describe`]) is not in the record,
    /// which names no one: the caller gives it again.
    ///
    /// # Errors
    /// As [`record::replay`]: a line that does not parse, an input the
    /// engine refuses, a hash that differs.
    pub fn resume_recorded(record: &[u8]) -> Result<Self, ReplayError> {
        let mut lines = record::lines_of(record);
        let (preset, hash) = record::header_of(&mut lines)?;
        let mut session = Self::new(&preset).ok_or(ReplayError::Unbuildable)?;
        record::check_hash(&session.engine, None, hash)?;
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
                } => {
                    session.tell_time(at);
                    if !session.replayed(PlayerId::new(seat), by, action) {
                        return Err(ReplayError::Refused { n });
                    }
                    record::check_hash(&session.engine, Some(n), hash)?;
                    next = n + 1;
                }
                Line::Chair { n, .. } | Line::DeclaredMind { n, .. } => next = n + 1,
                Line::End { n, .. } => {
                    if !matches!(session.engine.pending(), Pending::GameOver(_)) {
                        return Err(ReplayError::NotOver { n });
                    }
                    ended = true;
                    next = n + 1;
                }
            }
        }
        session.record = Some(Recorder::resumed(next, ended));
        Ok(session)
    }

    /// One recorded input, applied as the live door that produced it
    /// applied it ([`Session::answer`] for a seat or its clock,
    /// `apply_house_action` for the house), without recording it again.
    /// `false` when the engine refuses it.
    fn replayed(&mut self, player: PlayerId, by: Source, action: PlayerAction) -> bool {
        let moves_the_game = !action.is_automation_setting();
        let deciding = self.deciding();
        let asked = self.answering(player, &action);
        let clock =
            (by == Source::Clock).then(|| clock_answer(self.engine.pending_for(player), &action));
        if self.engine.apply(player, action).is_err() {
            return false;
        }
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
        let mut session = Session::new_recorded(&table(seed), "test").expect("the table builds");
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
}
