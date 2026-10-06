//! Reading and writing a CLI's lines.

use super::{
    Arc, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ChildStderr, ChildStdin,
    ChildStdout, CliSeat, Decision, Dialect, Event, Gone, LockedOut, MAX_LINE, Menu, Mutex,
    Outcome, Queue, Reply, STDERR_TAIL, Seats, Tally, Usage, Value, Waiter, Weak, Wire, lock,
    lock_out, mpsc, narrator,
};

/// Reads an answer object against its question's menu.
pub(super) fn read(
    value: Option<&Value>,
    menu: &Menu,
) -> Result<(narrator::Resolved, Option<String>), String> {
    let Some(value) = value else {
        return Err("answer with one JSON object and nothing else".into());
    };
    let decision = Decision::from_json_answer(value)?;
    let resolved = menu.resolve(&decision)?;
    Ok((resolved, decision.say.filter(|say| !say.is_empty())))
}

/// Writes each line to the process's stdin; closes it when the session
/// drops its sender, which tells the tool the conversation is over.
pub(super) async fn write(mut stdin: ChildStdin, mut lines: mpsc::UnboundedReceiver<String>) {
    while let Some(line) = lines.recv().await {
        let sent = async {
            stdin.write_all(line.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await
        };
        if sent.await.is_err() {
            break;
        }
    }
}

/// Keeps the last [`STDERR_TAIL`] bytes of the process's stderr.
pub(super) async fn collect(mut stderr: ChildStderr, tail: Arc<Mutex<String>>) {
    let mut chunk = [0u8; 1024];
    while let Ok(n) = stderr.read(&mut chunk).await {
        if n == 0 {
            break;
        }
        let mut tail = lock(&tail);
        tail.push_str(&String::from_utf8_lossy(&chunk[..n]));
        if tail.len() > STDERR_TAIL {
            let from = tail.len() - STDERR_TAIL;
            let from = (from..tail.len())
                .find(|at| tail.is_char_boundary(*at))
                .unwrap_or(tail.len());
            tail.drain(..from);
        }
    }
}

/// A line read, or why not.
pub(super) enum Line {
    Read,
    TooLong,
    End,
}

/// Reads one line into `buf`, at most [`MAX_LINE`] bytes of it.
pub(super) async fn read_line(
    out: &mut BufReader<ChildStdout>,
    buf: &mut Vec<u8>,
) -> std::io::Result<Line> {
    buf.clear();
    let mut long = false;
    loop {
        let (used, ended) = {
            let available = out.fill_buf().await?;
            if available.is_empty() {
                return Ok(match (long, buf.is_empty()) {
                    (true, _) => Line::TooLong,
                    (false, true) => Line::End,
                    (false, false) => Line::Read,
                });
            }
            let newline = available.iter().position(|b| *b == b'\n');
            let chunk = &available[..newline.unwrap_or(available.len())];
            if !long {
                if buf.len() + chunk.len() > MAX_LINE {
                    long = true;
                    buf.clear();
                } else {
                    buf.extend_from_slice(chunk);
                }
            }
            (
                chunk.len() + usize::from(newline.is_some()),
                newline.is_some(),
            )
        };
        out.consume(used);
        if ended {
            return Ok(if long { Line::TooLong } else { Line::Read });
        }
    }
}

/// What reads a process's output: each reply to its question, counted
/// in the tally whether or not the question still waits; and the lockdown,
/// held by the reader itself, so it holds when no question waits.
pub(super) struct Reader {
    pub(super) dialect: Arc<dyn Dialect>,
    /// The conversation's state, shared with the messages sent to it.
    pub(super) wire: Arc<Mutex<Wire>>,
    /// The process's stdin, for what the dialect writes back as it reads.
    pub(super) lines: mpsc::WeakUnboundedSender<String>,
    pub(super) queue: Arc<Mutex<Queue>>,
    pub(super) tally: Arc<Mutex<Tally>>,
    /// Weak: a seat owns its process, and the process its reader.
    pub(super) seat: Weak<Mutex<CliSeat>>,
    /// Every seat of the mind, weakly for the same reason: a lockout ends
    /// all their processes.
    pub(super) seats: Weak<Seats>,
    pub(super) locked_out: LockedOut,
    /// The last usage this process reported, for a dialect that reports a
    /// running count ([`Dialect::usage_is_cumulative`]): one reader a
    /// process, so a new process counts from nothing again.
    pub(super) counted: Usage,
    /// The conversation's last reading, where the running count runs on
    /// across the processes that resume it
    /// ([`Dialect::usage_spans_resumes`]): read and kept in place of
    /// [`Self::counted`].
    pub(super) running: Option<Arc<Mutex<Usage>>>,
}

impl Reader {
    /// Reads until the output ends or the process breaks the lockdown: a
    /// start that names a tool it must not have (or does not name them),
    /// a line that shows the model used one ([`Event::Breach`]), or an
    /// answer or a failure before any start, which would be a reply
    /// nothing vouched for. A break locks the mind out and kills its
    /// processes before any question waiting hears it. A rate limit before
    /// any start carries no answer and breaks nothing: reading stops there
    /// ([`Gone::Limited`]), and the mind cools down instead.
    pub(super) async fn run(mut self, stdout: ChildStdout) {
        let mut out = BufReader::new(stdout);
        let mut buf = Vec::new();
        let mut started = false;
        let gone = 'reading: loop {
            match read_line(&mut out, &mut buf).await {
                Ok(Line::Read) => {}
                Ok(Line::TooLong) => continue,
                Ok(Line::End) | Err(_) => break Gone::Ended,
            }
            let Ok(line) = std::str::from_utf8(&buf) else {
                continue;
            };
            // A line is read once, or twice where the dialect asks (a start
            // read off a line that says more): never more.
            let mut events = Vec::with_capacity(1);
            let (back, strayed) = {
                let mut wire = lock(&self.wire);
                events.push(self.dialect.read_event(line.trim_end(), &mut wire));
                if std::mem::take(&mut wire.again) {
                    events.push(self.dialect.read_event(line.trim_end(), &mut wire));
                    wire.again = false;
                }
                (std::mem::take(&mut wire.out), wire.strayed())
            };
            if strayed {
                // Nothing it says belongs to the conversation it was asked
                // to go on with.
                break Gone::Strayed;
            }
            if let Some(lines) = self.lines.upgrade() {
                for line in back {
                    let _ = lines.send(line);
                }
            }
            for event in events {
                if let Some(gone) = self.take(event, &mut started) {
                    break 'reading gone;
                }
            }
        };
        self.finish(&gone);
    }

    /// Takes one event, and says why the process is done with where it is.
    pub(super) fn take(&mut self, event: Event, started: &mut bool) -> Option<Gone> {
        match event {
            Event::Breach(why) => return Some(Gone::Refused(why)),
            Event::Started(said) => {
                if let Some(why) = self.dialect.lockdown_fault(&said) {
                    return Some(Gone::Refused(why));
                }
                *started = true;
                lock(&self.queue).heard = true;
            }
            Event::Reply(Outcome::RateLimited { why, lifts_in }) if !*started => {
                return Some(Gone::Limited { why, lifts_in });
            }
            Event::Reply(_) if !*started => {
                return Some(Gone::Refused(format!(
                    "the {} process replied before it said what it offers the model: the seat \
                     does not play through it",
                    self.dialect.tool().name()
                )));
            }
            Event::Reply(outcome) => self.reply(outcome),
            Event::Other => {}
        }
        None
    }

    /// The process is done with, for `gone`: a break locks the mind out,
    /// and every question still waiting hears it.
    pub(super) fn finish(&self, gone: &Gone) {
        if let Gone::Refused(why) = &gone {
            lock_out(&self.locked_out, self.seats.upgrade().as_deref(), why);
        }
        // Every question still waiting hears it, and counts at its worst:
        // nobody knows what the tool spent on it. The oldest one's rate
        // limit, which answered it, is unbilled, as any rate limit is.
        let (waiting, heard): (Vec<Waiter>, bool) = {
            let mut queue = lock(&self.queue);
            queue.gone = Some(gone.clone());
            (queue.waiting.drain(..).collect(), queue.heard)
        };
        // One that ended without a word reached no model: failed, but at no
        // cost.
        let unheard = matches!(gone, Gone::Ended) && !heard;
        let mut limited = matches!(gone, Gone::Limited { .. });
        for waiter in waiting {
            lock(&self.tally).back(waiter.worst, Err(!(limited || unheard)), None);
            limited = false;
            let _ = waiter.reply.send(Reply::Gone(gone.clone()));
        }
    }

    /// Hands `outcome` to the oldest question waiting, after counting it:
    /// its tokens as the tool counted them (for a running count, what it
    /// grew by since the process's last reading), at its worst where it did
    /// not say, nothing for a rate limit. An answer nobody waits for any
    /// more is noted as late for the seat's next message.
    pub(super) fn reply(&mut self, mut outcome: Outcome) {
        if let Outcome::Answer {
            usage: Some(usage), ..
        } = &mut outcome
            && self.dialect.usage_is_cumulative()
        {
            let reading = *usage;
            let before = self
                .running
                .as_ref()
                .map_or(self.counted, |running| *lock(running));
            *usage = reading.since(before);
            self.counted = reading;
            if let Some(running) = &self.running {
                *lock(running) = reading;
            }
        }
        let Some(waiter) = lock(&self.queue).waiting.pop_front() else {
            return;
        };
        {
            let mut tally = lock(&self.tally);
            match &outcome {
                Outcome::Answer {
                    usage: Some(usage), ..
                } => tally.back(waiter.worst, Ok(*usage), None),
                Outcome::Answer { usage: None, .. } => tally.back_at_worst(waiter.worst),
                Outcome::RateLimited { .. } => tally.back(waiter.worst, Err(false), None),
                Outcome::Failed(_) => tally.back(waiter.worst, Err(true), None),
            }
        }
        if let Err(Reply::Outcome(Outcome::Answer { .. })) =
            waiter.reply.send(Reply::Outcome(outcome))
            && let Some(seat) = self.seat.upgrade()
        {
            lock(&seat).seat.late = Some(waiter.question);
        }
    }
}
