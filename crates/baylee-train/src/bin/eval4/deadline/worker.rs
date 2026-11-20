//! One outstanding search and a private reply channel for each question.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Gate {
    Ready,
    Deadline,
    Busy,
    WorkerLost,
}

impl Gate {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Deadline => "deadline",
            Self::Busy => "busy",
            Self::WorkerLost => "worker_lost",
        }
    }
}

pub(super) struct Reply<T> {
    pub gate: Gate,
    pub value: Option<T>,
    pub dispatched: bool,
}

type Completion = (u64, Instant);

#[derive(Default)]
pub(super) struct Worker {
    pending: Option<JoinHandle<Completion>>,
    pub completions: Vec<Completion>,
}

impl Worker {
    pub fn reap_finished(&mut self) {
        if self.pending.as_ref().is_some_and(JoinHandle::is_finished) {
            self.drain();
        }
    }

    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    pub fn drain(&mut self) {
        if let Some(worker) = self.pending.take()
            && let Ok(completion) = worker.join()
        {
            self.completions.push(completion);
        }
    }

    pub fn wait<T: Send + 'static>(
        &mut self,
        sequence: u64,
        until: Instant,
        compute: impl FnOnce() -> T + Send + 'static,
    ) -> std::io::Result<Reply<T>> {
        self.reap_finished();
        if self.is_busy() {
            return Ok(Reply {
                gate: Gate::Busy,
                value: None,
                dispatched: false,
            });
        }
        if Instant::now() >= until {
            return Ok(Reply {
                gate: Gate::Deadline,
                value: None,
                dispatched: false,
            });
        }
        let (send, receive) = mpsc::sync_channel(1);
        self.pending = Some(
            std::thread::Builder::new()
                .name("eval4-search".into())
                .spawn(move || {
                    let value = compute();
                    let finished = Instant::now();
                    let _ = send.send((sequence, finished, value));
                    (sequence, finished)
                })?,
        );
        let result = receive.recv_timeout(until.saturating_duration_since(Instant::now()));
        let reply = match result {
            Ok((reply_sequence, finished, value))
                if sequence == reply_sequence && finished < until && Instant::now() < until =>
            {
                Reply {
                    gate: Gate::Ready,
                    value: Some(value),
                    dispatched: true,
                }
            }
            Ok(_) | Err(RecvTimeoutError::Timeout) => Reply {
                gate: Gate::Deadline,
                value: None,
                dispatched: true,
            },
            Err(RecvTimeoutError::Disconnected) => Reply {
                gate: Gate::WorkerLost,
                value: None,
                dispatched: true,
            },
        };
        self.reap_finished();
        Ok(reply)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.drain();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn first_ready_reply_is_returned_once() {
        let mut worker = Worker::default();
        let until = Instant::now() + Duration::from_secs(5);
        let reply = worker.wait(4, until, || "first").unwrap();
        assert_eq!(reply.gate.key(), Gate::Ready.key());
        assert_eq!(reply.value, Some("first"));
        worker.drain();
        assert_eq!(worker.completions.len(), 1);
        assert_eq!(worker.completions[0].0, 4);
    }

    #[test]
    fn deadline_discards_stale_reply_and_never_queues_more_work() {
        let mut worker = Worker::default();
        let (release, blocked) = mpsc::sync_channel(1);
        let first_until = Instant::now() + Duration::from_secs(5);
        let first = worker
            .wait(4, first_until, move || {
                blocked.recv().unwrap();
                "stale"
            })
            .unwrap();
        assert_eq!(first.gate, Gate::Deadline);
        assert_eq!(first.value, None);
        assert!(Instant::now() >= first_until);
        for sequence in 5..105 {
            let until = Instant::now() + Duration::from_secs(5);
            let reply = worker
                .wait(sequence, until, || panic!("busy work must never start"))
                .unwrap();
            assert_eq!(reply.gate, Gate::Busy);
            assert_eq!(reply.value, None::<()>);
        }
        release.send(()).unwrap();
        worker.drain();
        assert_eq!(worker.completions.len(), 1);
        assert!(worker.completions[0].1 > first_until);
        let until = Instant::now() + Duration::from_secs(5);
        let next = worker.wait(105, until, || "next").unwrap();
        assert_eq!(next.gate, Gate::Ready);
        assert_eq!(next.value, Some("next"));
        worker.drain();
        assert_eq!(worker.completions.len(), 2);
        assert_eq!(worker.completions[1].0, 105);
    }

    #[test]
    fn expired_question_cannot_launch_work() {
        let mut worker = Worker::default();
        let reply = worker
            .wait(4, Instant::now(), || {
                panic!("expired work must never start")
            })
            .unwrap();
        assert_eq!(reply.gate, Gate::Deadline);
        assert!(worker.completions.is_empty());
    }
}
