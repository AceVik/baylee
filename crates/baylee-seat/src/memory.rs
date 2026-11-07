//! What the seat remembers of the table: what a client keeps, and nothing a
//! client does not.
//!
//! The roster and print table ([`GameStatic`]), the view its frames left
//! ([`ViewReader`]), the seat's log, and how far along the game it has been
//! told (for `ResumeGame`). A frame may carry no view at all, only more of a
//! long log, and the log is appended by `from`, never by frame
//! (`docs/protocol.md` §"The game log").
//!
//! The log is kept in the client's own book ([`LogBook`]), each tail read
//! beside the view that came with it, as the client reads it: the book a
//! report's game log is written from ([`crate::reports::game_log`]), line
//! for line what a person's client at this seat would attach.

use baylee_client_core::gamelog::LogBook;
use baylee_engine::choice::Pending;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, LogEntry, LogTail, PlayerView, StaticReader, ViewReader};
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
    /// The view could not be kept: a frame was lost, or did not apply. The
    /// table is asked for the whole view, and until it comes the view held
    /// stands.
    Lost(String),
    /// A frame that could not be read, and why.
    Unreadable(String),
    /// Anything else (a loading count, a heartbeat): nothing to do.
    Other,
}

/// The seat's memory of the table.
#[derive(Debug, Default)]
pub struct TableMemory {
    statics: StaticReader,
    stream: ViewReader,
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
                match self.statics.read(&msg.statics, &msg.more) {
                    Ok(_) => Heard::Statics,
                    // Printings it cannot place come again with a whole view.
                    Err(why) if !msg.more.is_empty() => Heard::Lost(why.to_string()),
                    Err(e) => Heard::Unreadable(format!("unreadable game setup: {e}")),
                }
            }
            v1::envelope::Msg::StateDelta(delta) => {
                self.last_seq = self.last_seq.max(delta.seq);
                let lines = !delta.log.is_empty();
                // The view is the game and the lines are its commentary; a
                // frame's view is not wrong because its lines are, and the
                // whole log comes again on resume.
                let tail = if lines {
                    let tail = baylee_core::wire::decode::<LogTail>(&delta.log).ok();
                    if tail.is_none() {
                        self.gaps += 1;
                    }
                    tail
                } else {
                    None
                };
                let lost = self.stream.read(&delta.view_frame).err();
                // Its lines are read beside the view this frame left, or the
                // one held if it left none; with neither, they come again on
                // resume.
                match (&tail, self.stream.view()) {
                    (Some(tail), Some(held)) => {
                        self.log.append(tail, held);
                    }
                    (Some(_), None) => self.gaps += 1,
                    (None, _) => {}
                }
                lost.map_or(Heard::View, |why| Heard::Lost(why.to_string()))
            }
            v1::envelope::Msg::ChoiceRequest(request) => {
                self.last_seq = self.last_seq.max(request.seq);
                match baylee_core::wire::decode::<Pending>(&request.pending) {
                    Ok(pending) => Heard::Question {
                        seq: request.seq,
                        pending: Box::new(pending),
                        raw: request.pending,
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
        self.statics.statics()
    }

    /// The newest view.
    #[must_use]
    pub const fn view(&self) -> Option<&PlayerView> {
        self.stream.view()
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
                view_frame: baylee_view::ViewFrame::snapshot_of(&view),
                log: tail
                    .map(|t| baylee_core::wire::encode(&t))
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

    /// A patch that does not place keeps the view held, and says so once.
    #[test]
    fn a_patch_that_does_not_place_is_lost_and_asked_for_once() {
        let mut sender = baylee_view::ViewSender::default();
        let mut memory = TableMemory::default();
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.seq = 5;
        let frame = |bytes: Vec<u8>, seq| {
            Envelope {
                msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
                    game_id: "g".into(),
                    seq,
                    view_frame: bytes,
                    log: Vec::new(),
                })),
            }
            .encode_to_vec()
        };
        let whole = sender.next(&view);
        assert!(matches!(memory.hear(&frame(whole, 5)), Heard::View));
        view.seq = 6;
        let _lost = sender.next(&view);
        view.seq = 7;
        let after = sender.next(&view);
        assert!(matches!(memory.hear(&frame(after, 7)), Heard::Lost(_)));
        assert_eq!(memory.view().map(|held| held.seq), Some(5));
        view.seq = 8;
        let more = sender.next(&view);
        assert!(matches!(memory.hear(&frame(more, 8)), Heard::View));
        assert_eq!(memory.view().map(|held| held.seq), Some(5));
        let again = sender.snapshot(&view);
        assert!(matches!(memory.hear(&frame(again, 8)), Heard::View));
        assert_eq!(memory.view().map(|held| held.seq), Some(8));
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
                view_frame: Vec::new(),
                log: baylee_core::wire::encode(&LogTail {
                    from: 1,
                    entries: vec![line(2)],
                }),
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
                    view_frame: b"not a view".to_vec(),
                    log: baylee_core::wire::encode(&LogTail {
                        from,
                        entries: vec![line(1)],
                    }),
                })),
            }
            .encode_to_vec()
        };
        let mut memory = TableMemory::default();
        assert!(matches!(memory.hear(&unreadable(1, 0)), Heard::Lost(_)));
        assert!(memory.log().is_empty());
        assert_eq!(memory.gaps(), 1);
        memory.hear(&delta(2, 2, None));
        assert!(matches!(memory.hear(&unreadable(3, 0)), Heard::Lost(_)));
        assert_eq!(memory.book().entries().len(), 1);
        assert_eq!(memory.gaps(), 1);
    }
}
