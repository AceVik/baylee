//! Starting CLI processes and sending their questions.

use super::{
    Arc, CliMind, CliSeat, Command, Conversation, GameContext, Gone, Instant, MindError, Mutex,
    Queue, Reader, Reply, Session, SessionDir, Stdio, Store, Usage, Waiter, Weak, Wire, collect,
    detach, lock, mpsc, oneshot, write,
};

impl CliMind {
    /// Starts a process for `context`'s seat; for a tool that resumes a
    /// conversation, in `conversation`'s store, going on with it once it
    /// has an id.
    pub(super) fn spawn(
        &self,
        context: &GameContext,
        seat: Weak<Mutex<CliSeat>>,
        conversation: Option<&Conversation>,
    ) -> Result<Session, String> {
        let tool = self.tool();
        let dir = SessionDir::new(context.seat.get())
            .map_err(|e| format!("{tool}'s private directory could not be made: {e}"))?;
        let store = conversation.map(|going| &going.store);
        let env = self.launch.env(
            &dir.tmp(),
            &dir.support(),
            store.map(Store::data).as_deref(),
        )?;
        let dialect = &self.launch.dialect;
        let model = self.launch.model.as_deref();
        dir.write(&dialect.files(&self.settings, &self.system))
            .and_then(|()| dir.write_workspace(&dialect.workspace_files(&self.system)))
            .map_err(|e| format!("{tool}'s files could not be written: {e}"))?;
        let mut args = dialect.args(&self.settings, model, &self.system, &dir.support());
        let resumes = conversation.and_then(|going| going.id.as_deref());
        if let Some(id) = resumes {
            dialect.resume(&mut args, id);
        }
        let lookup = |name: &str| {
            env.iter()
                .find(|(passed, _)| passed == name)
                .map(|(_, value)| value.clone())
        };
        if let Some(warning) = dialect.home_warning(&lookup)
            && !self.warned.swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            tracing::warn!("{tool}: {warning}");
        }
        let work = store.map_or_else(|| dir.work(), Store::work);
        if let Some(store) = store {
            store.touch();
        }
        let mut command = Command::new(&self.launch.program);
        command
            .args(args)
            .env_clear()
            .envs(env)
            .current_dir(&work)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if dialect.detaches() {
            detach(&mut command);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("{tool} could not start: {e}"))?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return Err(format!("{tool} started without its pipes"));
        };
        let (lines, to_stdin) = mpsc::unbounded_channel();
        let queue = Arc::new(Mutex::new(Queue::default()));
        let tail = Arc::new(Mutex::new(String::new()));
        let mut wire = Wire {
            asked: resumes.map(str::to_string),
            ..Wire::default()
        };
        for line in dialect.opening(&self.settings, model, &work, &mut wire) {
            // The receiver lives until the writer below ends.
            let _ = lines.send(line);
        }
        let wire = Arc::new(Mutex::new(wire));
        tokio::spawn(write(stdin, to_stdin));
        tokio::spawn(
            Reader {
                dialect: Arc::clone(dialect),
                wire: Arc::clone(&wire),
                // Weak: the session's sender alone keeps stdin open.
                lines: lines.downgrade(),
                queue: Arc::clone(&queue),
                tally: Arc::clone(&self.tally),
                seat,
                seats: Arc::downgrade(&self.seats),
                locked_out: Arc::clone(&self.locked_out),
                counted: Usage::default(),
                running: conversation
                    .filter(|_| dialect.usage_spans_resumes())
                    .map(|going| Arc::clone(&going.counted)),
            }
            .run(stdout),
        );
        tokio::spawn(collect(stderr, Arc::clone(&tail)));
        Ok(Session {
            child,
            sent: if resumes.is_some() {
                conversation.map_or(0, |going| going.sent)
            } else {
                0
            },
            used: Instant::now(),
            lines: Some(lines),
            wire,
            queue,
            stderr: tail,
            dir,
        })
    }

    /// Sends `text` as the seat's next message, holding what it may cost:
    /// the receiver hears its reply.
    pub(super) fn ask(
        &self,
        seat: &Mutex<CliSeat>,
        text: &str,
        question: u64,
    ) -> Result<oneshot::Receiver<Reply>, MindError> {
        let mut state = lock(seat);
        let Some(session) = state.session.as_mut() else {
            return Err(MindError::Unavailable(format!(
                "the {} process is gone",
                self.tool()
            )));
        };
        let (reply, heard) = oneshot::channel();
        {
            let mut queue = lock(&session.queue);
            if let Some(gone) = &queue.gone {
                let _ = reply.send(Reply::Gone(gone.clone()));
                return Ok(heard);
            }
            // The whole conversation is read again at every message.
            let worst = self.settings.worst(session.sent + text.len());
            {
                let mut tally = lock(&self.tally);
                if let Err(why) = tally.hold(worst, &self.settings) {
                    tally.spent = true;
                    return Err(MindError::Declined(why));
                }
            }
            queue.waiting.push_back(Waiter {
                question,
                worst,
                reply,
            });
            let framed = self.launch.dialect.message(text, &mut lock(&session.wire));
            let sent = session
                .lines
                .as_ref()
                .is_some_and(|lines| framed.into_iter().all(|line| lines.send(line).is_ok()));
            if !sent {
                // The writer is gone, and so is the process: the reader
                // ends every question waiting.
                queue.gone.get_or_insert(Gone::Ended);
            }
        }
        if self.launch.dialect.one_shot() {
            // Its one message is all it reads: its stdin closes.
            session.lines = None;
        }
        session.sent += text.len();
        session.used = Instant::now();
        let (sent, used) = (session.sent, session.used);
        if let Some(going) = state.conversation.as_mut() {
            going.sent = sent;
            going.used = used;
        }
        Ok(heard)
    }
}
