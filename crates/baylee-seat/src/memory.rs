//! What the seat remembers of the table: what a client keeps, and nothing a
//! client does not.
//!
//! The roster and print table ([`GameStatic`]), the newest view, the seat's
//! log, and how far along the game it has been told (for `ResumeGame`). A
//! frame whose view is older than the one held is not read for its view,
//! but its log tail is: several frames may repeat a view and a `seq`, and
//! the log is appended by `from`, never by frame (`docs/protocol.md`
//! §"The game log").
//!
//! The log is kept in the client's own book ([`LogBook`]), each tail read
//! beside the view that came with it, as the client reads it: the book a
//! report's game log is written from ([`crate::reports::game_log`]), line
//! for line what a person's client at this seat would attach.

use baylee_client_core::gamelog::LogBook;
use baylee_engine::choice::Pending;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, LogEntry, LogTail, PlayerView};
use prost::Message as _;

/// What one frame from the table turned out to be.
#[derive(Debug)]
pub enum Heard {
    /// The roster and print table, the first time or again after a chair
    /// changed hands.
    Statics,
    /// A view (and any log lines with it).
    View,
    /// A question, with the sequence number it came with and its bytes
    /// exactly as sent: two askings of one question are told apart from two
    /// questions that look alike by what came between them, and the bytes
    /// are how "alike" is measured.
    Question {
        /// The frame's sequence number.
        seq: u64,
        /// The question.
        pending: Box<Pending>,
        /// The question as the table encoded it.
        raw: Vec<u8>,
    },
    /// The table opened: questions may follow.
    Curtain,
    /// The table refused something this seat sent, in words.
    Refused(String),
    /// The table's answer to a clock probe.
    ClockProbe {
        /// What this seat stamped the probe with.
        client_time_ms: u64,
    },
    /// The table does not speak this bridge's protocol.
    Incompatible(String),
    /// A frame that could not be read, and why.
    Unreadable(String),
    /// Anything else (a loading count, a heartbeat): nothing to do.
    Other,
}

/// The seat's memory of the table.
#[derive(Debug, Default)]
pub struct TableMemory {
    statics: Option<GameStatic>,
    view: Option<PlayerView>,
    log: LogBook,
    handed: usize,
    gaps: u32,
    last_seq: u64,
    curtain: bool,
}

impl TableMemory {
    /// Reads one frame from the table.
    pub fn hear(&mut self, bytes: &[u8]) -> Heard {
        let envelope = match Envelope::decode(bytes) {
            Ok(envelope) => envelope,
            Err(e) => return Heard::Unreadable(format!("a frame that is not an envelope: {e}")),
        };
        let Some(msg) = envelope.msg else {
            return Heard::Other;
        };
        match msg {
            v1::envelope::Msg::GameStatic(msg) => {
                if msg.view_version != baylee_view::VIEW_VERSION {
                    return Heard::Incompatible(format!(
                        "this bridge reads game views version {}, the table speaks {}",
                        baylee_view::VIEW_VERSION,
                        msg.view_version
                    ));
                }
                match serde_json::from_slice::<GameStatic>(&msg.static_json) {
                    Ok(statics) => {
                        self.statics = Some(statics);
                        Heard::Statics
                    }
                    Err(e) => Heard::Unreadable(format!("unreadable game setup: {e}")),
                }
            }
            v1::envelope::Msg::StateDelta(delta) => {
                self.last_seq = self.last_seq.max(delta.seq);
                let lines = !delta.log_json.is_empty();
                // The view is the game and the lines are its commentary; a
                // frame's view is not wrong because its lines are, and the
                // whole log comes again on resume.
                let tail = if lines {
                    let tail = serde_json::from_slice::<LogTail>(&delta.log_json).ok();
                    if tail.is_none() {
                        self.gaps += 1;
                    }
                    tail
                } else {
                    None
                };
                // More lines of the view held: read beside it, and the view
                // is not read again.
                if lines
                    && delta.seq != 0
                    && let Some(held) = self.view.as_ref().filter(|held| delta.seq <= held.seq)
                {
                    if let Some(tail) = &tail {
                        self.log.append(tail, held);
                    }
                    return Heard::View;
                }
                match serde_json::from_slice::<PlayerView>(&delta.view_json) {
                    Ok(view) => {
                        if let Some(tail) = &tail {
                            self.log.append(tail, &view);
                        }
                        if self.view.as_ref().is_none_or(|held| view.seq >= held.seq) {
                            self.view = Some(view);
                        }
                        Heard::View
                    }
                    Err(e) => {
                        // Its lines are read beside the view held; with
                        // none, they come again on resume.
                        match (&tail, self.view.as_ref()) {
                            (Some(tail), Some(held)) => {
                                self.log.append(tail, held);
                            }
                            (Some(_), None) => self.gaps += 1,
                            (None, _) => {}
                        }
                        Heard::Unreadable(format!("unreadable game state: {e}"))
                    }
                }
            }
            v1::envelope::Msg::ChoiceRequest(request) => {
                self.last_seq = self.last_seq.max(request.seq);
                match serde_json::from_slice::<Pending>(&request.pending_json) {
                    Ok(pending) => Heard::Question {
                        seq: request.seq,
                        pending: Box::new(pending),
                        raw: request.pending_json,
                    },
                    Err(e) => Heard::Unreadable(format!("unreadable question: {e}")),
                }
            }
            v1::envelope::Msg::Curtain(_) => {
                self.curtain = true;
                Heard::Curtain
            }
            v1::envelope::Msg::Error(error) => Heard::Refused(error.message),
            v1::envelope::Msg::ClockProbe(probe) => Heard::ClockProbe {
                client_time_ms: probe.client_time_ms,
            },
            v1::envelope::Msg::HelloAck(ack) if !ack.compatible => Heard::Incompatible(ack.message),
            _ => Heard::Other,
        }
    }

    /// Every log line not yet handed to the mind, marked handed.
    ///
    /// The book appends a tail by its `from` ([`LogBook::append`]): lines
    /// already held are skipped, and a tail that starts past the end is a
    /// gap, counted and not appended (its lines would be filed under the
    /// wrong numbers). The table tells the whole log again after a
    /// reconnect, which closes any gap.
    pub fn hand_over_log(&mut self) -> LogTail {
        let entries = self.log.entries();
        // A book that replaced lines it held hands them over again from
        // where it parted (a table never tells one seat two histories).
        let from = self.handed.min(entries.len());
        self.handed = entries.len();
        LogTail {
            from: u32::try_from(from).unwrap_or(u32::MAX),
            entries: entries[from..].to_vec(),
        }
    }

    /// The roster and print table, once told.
    #[must_use]
    pub const fn statics(&self) -> Option<&GameStatic> {
        self.statics.as_ref()
    }

    /// The newest view.
    #[must_use]
    pub const fn view(&self) -> Option<&PlayerView> {
        self.view.as_ref()
    }

    /// The seat's whole log so far.
    #[must_use]
    pub fn log(&self) -> &[LogEntry] {
        self.log.entries()
    }

    /// The seat's whole log so far, in the client's book: what a report's
    /// log is written from.
    #[must_use]
    pub const fn book(&self) -> &LogBook {
        &self.log
    }

    /// How many log tails could not be placed (unreadable, past the end,
    /// or with no view to read them beside).
    #[must_use]
    pub const fn gaps(&self) -> u32 {
        self.gaps.saturating_add(self.log.gaps())
    }

    /// The highest sequence number seen: what `ResumeGame` reports.
    #[must_use]
    pub const fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// Whether the table has opened.
    #[must_use]
    pub const fn curtain_up(&self) -> bool {
        self.curtain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_view::LogEvent;

    fn line(turn: u32) -> LogEntry {
        LogEntry {
            turn,
            repeat: 1,
            at: 0,
            event: LogEvent::TurnStarted {
                active: PlayerId::new(0),
            },
            phase: None,
        }
    }

    fn delta(seq: u64, view_seq: u64, tail: Option<LogTail>) -> Vec<u8> {
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.seq = view_seq;
        Envelope {
            msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
                game_id: "g".into(),
                seq,
                view_json: serde_json::to_vec(&view).unwrap(),
                log_json: tail
                    .map(|t| serde_json::to_vec(&t).unwrap())
                    .unwrap_or_default(),
            })),
        }
        .encode_to_vec()
    }

    /// The log is appended by `from`: a resend of lines already held adds
    /// nothing, an overlap adds only what is new, and the mind is handed
    /// each line once.
    #[test]
    fn the_log_is_appended_by_its_index_and_handed_over_once() {
        let mut memory = TableMemory::default();
        memory.hear(&delta(
            1,
            1,
            Some(LogTail {
                from: 0,
                entries: vec![line(1), line(1)],
            }),
        ));
        memory.hear(&delta(
            2,
            2,
            Some(LogTail {
                from: 1,
                entries: vec![line(1), line(2)],
            }),
        ));
        assert_eq!(
            memory.log().len(),
            3,
            "the overlapping line was not added twice"
        );
        let first = memory.hand_over_log();
        assert_eq!((first.from, first.entries.len()), (0, 3));
        memory.hear(&delta(
            3,
            3,
            Some(LogTail {
                from: 0,
                entries: vec![line(1), line(1), line(2), line(3)],
            }),
        ));
        let second = memory.hand_over_log();
        assert_eq!(
            (second.from, second.entries.len()),
            (3, 1),
            "a whole log told again after a reconnect hands over only the new line"
        );
        assert_eq!(memory.gaps(), 0);
    }

    /// A tail that starts past the end is not filed under the wrong numbers.
    #[test]
    fn a_tail_past_the_end_is_a_gap_and_not_appended() {
        let mut memory = TableMemory::default();
        memory.hear(&delta(
            1,
            1,
            Some(LogTail {
                from: 5,
                entries: vec![line(1)],
            }),
        ));
        assert!(memory.log().is_empty());
        assert_eq!(memory.gaps(), 1);
    }

    /// An older view is not read for its view, and its log tail still is.
    #[test]
    fn an_older_view_is_dropped_and_its_log_kept() {
        let mut memory = TableMemory::default();
        memory.hear(&delta(9, 9, None));
        memory.hear(&delta(
            4,
            4,
            Some(LogTail {
                from: 0,
                entries: vec![line(1)],
            }),
        ));
        assert_eq!(memory.view().map(|v| v.seq), Some(9));
        assert_eq!(memory.log().len(), 1);
        assert_eq!(memory.last_seq(), 9);
    }

    #[test]
    fn continuation_log_frames_append_lines_without_redecoding_the_view() {
        let mut memory = TableMemory::default();
        assert!(matches!(
            memory.hear(&delta(
                7,
                7,
                Some(LogTail {
                    from: 0,
                    entries: vec![line(1)],
                }),
            )),
            Heard::View
        ));
        let continuation = Envelope {
            msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
                game_id: "g".into(),
                seq: 7,
                view_json: b"not-decoded-again".to_vec(),
                log_json: serde_json::to_vec(&LogTail {
                    from: 1,
                    entries: vec![line(2)],
                })
                .unwrap(),
            })),
        }
        .encode_to_vec();
        assert!(matches!(memory.hear(&continuation), Heard::View));
        assert_eq!(memory.view().map(|v| v.seq), Some(7));
        assert_eq!(memory.log().len(), 2);
        assert_eq!(memory.gaps(), 0);
    }

    /// A frame whose view cannot be read still gives its lines to the book,
    /// read beside the view held; before any view, they wait for the whole
    /// log a resume tells again.
    #[test]
    fn lines_beside_an_unreadable_view_are_kept_where_a_view_is_held() {
        let unreadable = |seq: u64, from: u32| {
            Envelope {
                msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
                    game_id: "g".into(),
                    seq,
                    view_json: b"not a view".to_vec(),
                    log_json: serde_json::to_vec(&LogTail {
                        from,
                        entries: vec![line(1)],
                    })
                    .unwrap(),
                })),
            }
            .encode_to_vec()
        };
        let mut memory = TableMemory::default();
        assert!(matches!(
            memory.hear(&unreadable(1, 0)),
            Heard::Unreadable(_)
        ));
        assert!(memory.log().is_empty());
        assert_eq!(memory.gaps(), 1);
        memory.hear(&delta(2, 2, None));
        assert!(matches!(
            memory.hear(&unreadable(3, 0)),
            Heard::Unreadable(_)
        ));
        assert_eq!(memory.book().entries().len(), 1);
        assert_eq!(memory.gaps(), 1);
    }
}
