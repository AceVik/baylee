//! One outstanding search and a private reply channel for each question.

use std::any::Any;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::Instant;

use anyhow::{Context as _, anyhow};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Gate {
    Ready,
    Deadline,
    Busy,
}

pub(super) struct Reply<T> {
    pub gate: Gate,
    pub value: Option<T>,
    pub dispatched: bool,
}

impl<T> Reply<T> {
    /// A question that started no job.
    const fn undispatched(gate: Gate) -> Self {
        Self {
            gate,
            value: None,
            dispatched: false,
        }
    }
}

/// A job that ended with a value, by the number it was given.
type Completion = (u64, Instant);

#[derive(Default)]
pub(super) struct Worker {
    pending: Option<JoinHandle<anyhow::Result<Completion>>>,
    pub completions: Vec<Completion>,
}

/// What a panicked thread said.
fn panic_text(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no message")
}

impl Worker {
    /// Joins the job once it has ended.
    ///
    /// # Errors
    /// The job's own error, or its panic.
    pub fn reap_finished(&mut self) -> anyhow::Result<()> {
        if self.pending.as_ref().is_some_and(JoinHandle::is_finished) {
            self.drain()?;
        }
        Ok(())
    }

    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Waits for the outstanding job, if any.
    ///
    /// # Errors
    /// The job's own error, or its panic, also when its question was
    /// answered without it long ago.
    pub fn drain(&mut self) -> anyhow::Result<()> {
        let Some(worker) = self.pending.take() else {
            return Ok(());
        };
        let completion = worker
            .join()
            .map_err(|payload| anyhow!("the deadline search panicked: {}", panic_text(&*payload)))?
            .context("the deadline search")?;
        self.completions.push(completion);
        Ok(())
    }

    /// Starts `compute` as job `job` unless another still runs or `until`
    /// has passed, and waits for its value no longer than `until`.
    ///
    /// # Errors
    /// The error or panic of this job or of one before it, or when no
    /// thread starts.
    pub fn wait<T: Send + 'static>(
        &mut self,
        job: u64,
        until: Instant,
        compute: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
    ) -> anyhow::Result<Reply<T>> {
        self.reap_finished()?;
        if self.is_busy() {
            return Ok(Reply::undispatched(Gate::Busy));
        }
        if Instant::now() >= until {
            return Ok(Reply::undispatched(Gate::Deadline));
        }
        let (send, receive) = mpsc::sync_channel(1);
        self.pending = Some(
            std::thread::Builder::new()
                .name("eval4-search".into())
                .spawn(move || {
                    let value = compute();
                    let finished = Instant::now();
                    // An error goes to whoever joins this thread: a reply
                    // that comes too late is read by nobody.
                    let value = value?;
                    let _ = send.send((job, finished, value));
                    Ok((job, finished))
                })?,
        );
        let result = receive.recv_timeout(until.saturating_duration_since(Instant::now()));
        let reply = match result {
            Ok((replied, finished, value))
                if job == replied && finished < until && Instant::now() < until =>
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
            Err(RecvTimeoutError::Disconnected) => {
                self.drain()?;
                anyhow::bail!("the deadline search ended without a value");
            }
        };
        self.reap_finished()?;
        Ok(reply)
    }
}

impl Drop for Worker {
    /// A worker still holds a job here only where the run already ends, by
    /// an error or a panic. The job is waited for, and what it failed with
    /// is told rather than raised over the failure that ends the run.
    fn drop(&mut self) {
        if let Err(error) = self.drain() {
            eprintln!("[eval4] {error:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{self, AssertUnwindSafe};
    use std::time::Duration;

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }

    #[test]
    fn first_ready_reply_is_returned_once() {
        let mut worker = Worker::default();
        let reply = worker.wait(4, soon(), || Ok("first")).unwrap();
        assert_eq!(reply.gate, Gate::Ready);
        assert_eq!(reply.value, Some("first"));
        worker.drain().unwrap();
        assert_eq!(worker.completions.len(), 1);
        assert_eq!(worker.completions[0].0, 4);
    }

    #[test]
    fn deadline_discards_stale_reply_and_never_queues_more_work() {
        let mut worker = Worker::default();
        let (release, blocked) = mpsc::sync_channel(1);
        let first_until = Instant::now() + Duration::from_secs(1);
        let first = worker
            .wait(4, first_until, move || {
                blocked.recv()?;
                Ok("stale")
            })
            .unwrap();
        assert_eq!(first.gate, Gate::Deadline);
        assert_eq!(first.value, None);
        assert!(Instant::now() >= first_until);
        for job in 5..105 {
            let reply = worker
                .wait(job, soon(), || -> anyhow::Result<()> {
                    panic!("busy work must never start")
                })
                .unwrap();
            assert_eq!(reply.gate, Gate::Busy);
            assert_eq!(reply.value, None::<()>);
        }
        release.send(()).unwrap();
        worker.drain().unwrap();
        assert_eq!(worker.completions.len(), 1);
        assert!(worker.completions[0].1 > first_until);
        let next = worker.wait(105, soon(), || Ok("next")).unwrap();
        assert_eq!(next.gate, Gate::Ready);
        assert_eq!(next.value, Some("next"));
        worker.drain().unwrap();
        assert_eq!(worker.completions.len(), 2);
        assert_eq!(worker.completions[1].0, 105);
    }

    #[test]
    fn expired_question_cannot_launch_work() {
        let mut worker = Worker::default();
        let reply = worker
            .wait(4, Instant::now(), || -> anyhow::Result<()> {
                panic!("expired work must never start")
            })
            .unwrap();
        assert_eq!(reply.gate, Gate::Deadline);
        assert!(!reply.dispatched && worker.completions.is_empty());
    }

    #[test]
    fn a_failed_job_ends_the_run_at_its_own_question() {
        let mut worker = Worker::default();
        let error = worker
            .wait(1, soon(), || -> anyhow::Result<()> {
                anyhow::bail!("the model failed")
            })
            .err()
            .unwrap();
        assert!(format!("{error:#}").contains("the model failed"));
        let error = worker
            .wait(2, soon(), || -> anyhow::Result<()> {
                panic!("out of memory")
            })
            .err()
            .unwrap();
        assert!(
            format!("{error:#}").contains("panicked: out of memory"),
            "{error:#}"
        );
        assert!(!worker.is_busy() && worker.completions.is_empty());
        // The worker itself is none the worse.
        assert_eq!(worker.wait(3, soon(), || Ok(3)).unwrap().value, Some(3));
    }

    /// A job that fails after its question was answered is no "deadline":
    /// the next question, or the end of the game, hears of it.
    #[test]
    fn a_job_that_fails_after_the_deadline_is_heard_of() {
        for panics in [false, true] {
            let mut worker = Worker::default();
            let (release, blocked) = mpsc::sync_channel::<()>(1);
            let reply = worker
                .wait(
                    1,
                    Instant::now() + Duration::from_millis(200),
                    move || -> anyhow::Result<()> {
                        blocked.recv()?;
                        assert!(!panics, "late panic");
                        anyhow::bail!("late error")
                    },
                )
                .unwrap();
            assert_eq!(reply.gate, Gate::Deadline);
            release.send(()).unwrap();
            let error = worker.drain().err().unwrap();
            let expected = if panics { "late panic" } else { "late error" };
            assert!(format!("{error:#}").contains(expected), "{error:#}");
            assert!(worker.completions.is_empty());
        }
    }

    /// A panic of the run itself, with a failing job outstanding, unwinds as
    /// itself: the worker's drop raises nothing over it.
    #[test]
    fn a_failing_job_does_not_replace_the_failure_that_ends_the_run() {
        let (release, blocked) = mpsc::sync_channel::<()>(1);
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut worker = Worker::default();
            let _ = worker
                .wait(
                    1,
                    Instant::now() + Duration::from_millis(50),
                    move || -> anyhow::Result<()> {
                        blocked.recv()?;
                        panic!("the job's own panic")
                    },
                )
                .unwrap();
            release.send(()).unwrap();
            panic!("the run's own panic");
        }))
        .unwrap_err();
        assert_eq!(panic_text(&*payload), "the run's own panic");
    }
}
