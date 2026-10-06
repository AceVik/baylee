//! A conversation with one CLI process: its queue, its replies and when it
//! is gone.

use super::{
    Arc, BTreeMap, Child, Dialect, Duration, Instant, Menu, Mutex, Narrator, Outcome, PathBuf,
    Seat, SessionDir, Store, Usage, VecDeque, Wire, Worst, forget, lock, mpsc, oneshot, scrub,
};

/// Why a process stopped answering.
#[derive(Clone, Debug)]
pub(super) enum Gone {
    /// Its output ended: it exited or was killed.
    Ended,
    /// It broke the lockdown: a tool or a server it must not have, or an
    /// answer or a failure before it said what it offers.
    Refused(String),
    /// It said it was rate limited before it said what it offers: no
    /// answer, so nothing it could break, and nothing it vouched for
    /// either. Its reader stops, so nothing more is taken from it; the
    /// mind ends it and cools down.
    Limited {
        why: String,
        lifts_in: Option<Duration>,
    },
    /// Asked to go on with a conversation, it named another: it began a
    /// new one, which knows nothing of the game. Its reader stops before
    /// any reply, and the question begins the conversation again.
    Strayed,
}

/// What a question waiting on a process hears.
pub(super) enum Reply {
    Outcome(Outcome),
    Gone(Gone),
}

/// A message sent, waiting for its reply.
pub(super) struct Waiter {
    pub(super) question: u64,
    pub(super) worst: Worst,
    pub(super) reply: oneshot::Sender<Reply>,
}

/// The messages a process has not answered yet, in the order sent, and
/// whether it stopped answering: one lock, so no question waits on a
/// process that is gone.
#[derive(Default)]
pub(super) struct Queue {
    pub(super) waiting: VecDeque<Waiter>,
    pub(super) gone: Option<Gone>,
    /// Whether the process said anything at all (its start): one that
    /// ended without a word reached no model.
    pub(super) heard: bool,
}

/// One process, holding one seat's conversation across turns.
pub(super) struct Session {
    /// Killed when dropped.
    pub(super) child: Child,
    /// The bytes sent to it so far: its conversation's size.
    pub(super) sent: usize,
    /// When it was last asked.
    pub(super) used: Instant,
    /// Lines for its stdin, which close it when dropped: `None` once a
    /// one-shot process has its message ([`Dialect::one_shot`]).
    pub(super) lines: Option<mpsc::UnboundedSender<String>>,
    /// What its conversation keeps between lines, shared with its reader.
    pub(super) wire: Arc<Mutex<Wire>>,
    pub(super) queue: Arc<Mutex<Queue>>,
    pub(super) stderr: Arc<Mutex<String>>,
    /// Removed after the process, being dropped last.
    pub(super) dir: SessionDir,
}

impl Session {
    /// Ends the process: its stdin closed, `grace` to finish, then killed;
    /// its directory removed after it. Outside a runtime, at once.
    pub(super) fn end(self, grace: Duration) {
        let Self {
            mut child,
            lines,
            dir,
            ..
        } = self;
        drop(lines);
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(async move {
                if tokio::time::timeout(grace, child.wait()).await.is_err() {
                    let _ = child.kill().await;
                }
                drop(dir);
            });
        }
    }

    /// Whether the process ended while no message waited on it (between
    /// turns, say): its output ended, or it exited and its reader has not
    /// heard yet. One that stopped for another reason (a lockout, a rate
    /// limit before its start) is for the next message to hear, and a
    /// question still waiting hears it from the reader.
    pub(super) fn died(&mut self) -> bool {
        {
            let queue = lock(&self.queue);
            if !queue.waiting.is_empty() {
                return false;
            }
            match queue.gone {
                Some(Gone::Ended) => return true,
                Some(_) => return false,
                None => {}
            }
        }
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// The last line it wrote to stderr, scrubbed and short.
    pub(super) fn last_words(&self) -> Option<String> {
        let tail = lock(&self.stderr);
        tail.lines()
            .map(str::trim)
            .rfind(|line| !line.is_empty())
            .map(|line| scrub(line, None).chars().take(200).collect())
    }
}

/// What one seat keeps: what every language-model seat keeps, and its
/// process.
pub(super) struct CliSeat {
    pub(super) seat: Seat,
    pub(super) session: Option<Session>,
    /// Whether the last process ended before its conversation was done
    /// with: the next one's first message says the conversation was lost.
    pub(super) lost: bool,
    /// The conversation a tool that resumes one goes on with
    /// ([`Dialect::resumes`]); `None` for every other tool.
    pub(super) conversation: Option<Conversation>,
}

/// A conversation a tool keeps on disk, which the next process goes on
/// with ([`Dialect::resumes`]).
pub(super) struct Conversation {
    /// Its id, once the tool named it: until then nothing can resume it.
    pub(super) id: Option<String>,
    /// The bytes sent to it so far: its size.
    pub(super) sent: usize,
    /// When it was last asked.
    pub(super) used: Instant,
    /// Every id its processes named (one that strayed too), whose session
    /// files are removed with it.
    pub(super) ids: Vec<String>,
    /// Where its tool keeps sessions outside the store, with the tool:
    /// `None` where they are in the store.
    pub(super) sessions: Option<(Arc<dyn Dialect>, PathBuf)>,
    /// The running count its tool reported last, where that count runs on
    /// across the processes that resume it
    /// ([`Dialect::usage_spans_resumes`]).
    pub(super) counted: Arc<Mutex<Usage>>,
    /// Its own directory (the processes' working directory, and the tool's
    /// data where it can be put there); removed last.
    pub(super) store: Store,
}

impl Conversation {
    /// Whether the tool still has what it needs to go on with it: its
    /// session files, where they are kept outside the store.
    pub(super) fn kept(&self) -> bool {
        let Some(id) = &self.id else {
            return false;
        };
        self.sessions
            .as_ref()
            .is_none_or(|(dialect, root)| !dialect.session_files(root, id).is_empty())
    }

    /// Notes `id`, which a process named, for removal; the first is the
    /// conversation's own.
    pub(super) fn named(&mut self, id: String) {
        if self.ids.contains(&id) {
            return;
        }
        self.id.get_or_insert_with(|| id.clone());
        self.ids.push(id);
        if let Some((dialect, root)) = &self.sessions {
            self.store.record(dialect.tool(), root, &self.ids);
        }
    }

    /// Removes the session files of every id it named, outside its store.
    pub(super) fn forget(&self) {
        if let Some((dialect, root)) = &self.sessions {
            for id in &self.ids {
                forget(dialect.as_ref(), root, id);
            }
        }
    }
}

impl Drop for Conversation {
    fn drop(&mut self) {
        self.forget();
        // A process ended with it may still be writing its last line: once
        // its grace is over, again.
        if self.sessions.is_some()
            && !self.ids.is_empty()
            && tokio::runtime::Handle::try_current().is_ok()
        {
            let Some((dialect, root)) = self.sessions.clone() else {
                return;
            };
            let ids = self.ids.clone();
            tokio::spawn(async move {
                tokio::time::sleep(LATE_WRITE).await;
                for id in &ids {
                    forget(dialect.as_ref(), &root, id);
                }
            });
        }
    }
}

/// How long after a conversation is over its session files are removed
/// once more, past a process's grace ([`Limits::grace`]).
pub(super) const LATE_WRITE: Duration = Duration::from_secs(5);

/// A decision told and ready to send.
pub(super) struct Prepared {
    pub(super) menu: Menu,
    pub(super) narrator: Narrator,
    pub(super) text: String,
    pub(super) asked: u64,
    /// Whether it goes to a process that resumes the seat's conversation
    /// ([`Dialect::resumes`]): only what is new, so a resume that fails
    /// is told again from the start.
    pub(super) resumed: bool,
}

/// When the mind may ask again after a rate limit.
pub(super) struct Cooldown {
    pub(super) until: Option<Instant>,
    pub(super) next: Duration,
}

/// Each seat's state, by game and seat.
pub(super) type Seats = Mutex<BTreeMap<(String, u8), Arc<Mutex<CliSeat>>>>;

/// Why the mind will not play again: a process broke the lockdown. Shared
/// with every process's reader, which sets it.
pub(super) type LockedOut = Arc<Mutex<Option<String>>>;
