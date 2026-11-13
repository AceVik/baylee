//! Record streaming and authoritative game endings.
use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::Write as _;

impl EngineRunner {
    /// The gateway asked for the record as it stands (#323), for a bug
    /// report filed at this game: whatever of it has gathered, as a piece
    /// that is not `last`, and then [`v1::RecordFlushed`] with the gateway's
    /// `nonce`, on the same socket and so after the piece.
    ///
    /// Answered whatever the state: with nothing waiting, before the game is
    /// built, and after it ended, the answer is the acknowledgement alone,
    /// never an empty piece (it would cost the store a row and a `seq` for
    /// nothing). An unanswered ask costs the reporter the gateway's whole
    /// wait.
    ///
    /// Moves nothing in the game: no pump, no clock, no frame to a seat.
    pub(super) fn flush_record(&mut self, nonce: u64) -> Vec<Envelope> {
        let mut out = Vec::new();
        if !self.ended {
            let data = self.session.as_mut().map_or_default(Session::take_record);
            if !data.is_empty() {
                out.push(self.record_chunk(&data, false));
            }
        }
        out.push(Envelope {
            msg: Some(v1::envelope::Msg::RecordFlushed(v1::RecordFlushed {
                game_id: self.game_id.clone(),
                nonce,
                pieces: self.record_seq,
            })),
        });
        out
    }

    /// When (wall ms) the record waiting to be sent is due by
    /// [`RECORD_FLUSH_MS`] (#323), for the caller to arm a timer at and then
    /// call [`Self::flush_record_due`]; `None` when nothing waits, before
    /// the game is built, after it ended, and while the curtain is down.
    ///
    /// Not while the curtain is down, because a seat that is loading should
    /// have the database to itself (see the gateway's `record::Sink`); the
    /// first move after the curtain rises finds the header overdue.
    #[must_use]
    pub fn record_deadline(&self) -> Option<u64> {
        if self.ended || self.curtain.is_some() {
            return None;
        }
        let pending = self.session.as_ref()?.record_pending();
        (pending > 0).then(|| {
            self.record_waiting_since
                .unwrap_or(self.record_sent_at)
                .saturating_add(RECORD_FLUSH_MS)
        })
    }

    /// The record that has waited [`RECORD_FLUSH_MS`], as a piece that is
    /// not `last` (#323); nothing when none has, or when the time told
    /// ([`Self::tell_time`]) is not yet [`Self::record_deadline`]. Early
    /// and stale timers do nothing.
    pub fn flush_record_due(&mut self) -> Vec<Envelope> {
        if self.record_deadline().is_none_or(|at| self.now < at) {
            return Vec::new();
        }
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let data = session.take_record();
        vec![self.record_chunk(&data, false)]
    }

    /// Final record and game-end notification, after any participation endings.
    pub(super) fn game_ending(&mut self) -> Vec<Envelope> {
        if self.ended {
            return Vec::new();
        }
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let Pending::GameOver(result) = session.pending() else {
            let pending = session.record_pending();
            if pending > 0 && self.record_waiting_since.is_none() {
                self.record_waiting_since = Some(self.now);
            }
            if !record_due(pending) {
                return Vec::new();
            }
            let data = session.take_record();
            return vec![self.record_chunk(&data, false)];
        };
        let result = *result;
        let data = session.take_record();
        self.ended = true;
        // What the house's searches did here, for the operator: counts and
        // the slowest, nothing a player said and no card.
        let searches = session.foresight_stats();
        if searches.asked > 0 {
            tracing::info!(
                game_id = self.game_id,
                asked = searches.asked,
                searched = searches.searched,
                replaced = searches.replaced,
                refused = searches.refused,
                busy = searches.busy,
                walled = searches.walled,
                slowest_ms = searches.slowest_micros / 1000,
                most_work = searches.most_work,
                "house searches"
            );
        }
        // A draw has no winner, which is a shorter list rather than a
        // different message — and a team win is a longer one, which is why
        // the field was a list from the start.
        let winners = session
            .winning_seats(result)
            .into_iter()
            .map(|p| u32::from(p.get()))
            .collect();
        let reason = format!("{:?}", result.reason);
        let turn = session.state().turn.number;
        vec![
            self.record_chunk(&data, true),
            Envelope {
                msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded {
                    game_id: self.game_id.clone(),
                    winners,
                    reason,
                    turn,
                })),
            },
        ]
    }

    /// The next piece of the record, as one gzip member: pieces stored one
    /// after another in `seq` order are one gzip stream of the record.
    pub(super) fn record_chunk(&mut self, data: &[u8], last: bool) -> Envelope {
        let seq = self.record_seq;
        self.record_seq += 1;
        self.record_sent_at = self.now;
        self.record_waiting_since = None;
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        // Writing into a `Vec` cannot fail.
        gz.write_all(data).expect("compressing into memory");
        let data = gz.finish().expect("compressing into memory");
        Envelope {
            msg: Some(v1::envelope::Msg::GameRecordChunk(v1::GameRecordChunk {
                game_id: self.game_id.clone(),
                seq,
                data,
                last,
            })),
        }
    }
}
