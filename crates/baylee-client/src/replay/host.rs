//! [`ReplayHost`]: a duel host that plays a tape (§7.1).
//!
//! It watches (`watching() == true`), answers nothing (`submit` is a no-op)
//! and opens its curtain the moment the table says it is drawn. Its frames
//! are the tape's: one step on is shown as a live table shows a frame,
//! every other move is a [`HostMessage::Jump`] followed by the table at the
//! new step, its log told from the first line. The `seq` it sends is its
//! own ever-increasing count, so only a `Jump` turns time back.
//!
//! The tape is made off the frame: a local record is projected on a worker
//! thread, a hosted tape fetched and read on the transport's. `poll` never
//! waits for either.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, LazyLock, Mutex};

use baylee_client_core::replay::{
    Header, Out, Perspective, Perspectives, Tape, Timeline, Transport, Verify,
};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;

use super::{Loading, Origin, ReplayAsk, ReplayStatus, SharedTimeline};
use crate::host::{DuelHost, HostMessage};

/// Where a hosted tape is asked for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HostedAccess {
    /// The gateway's address.
    pub gateway: String,
    /// What the gateway is called, for the veil.
    pub name: String,
    /// The session that sat at the game.
    pub token: String,
    /// The game's id.
    pub game: String,
}

/// Where a replay's tape comes from.
#[derive(Clone, Debug)]
pub enum ReplaySource {
    /// A kept record (or a `BAYLEE_REPLAY` record file), played again here
    /// on a worker. `bytes` is the record's JSON Lines, inflated.
    Record {
        /// The record.
        bytes: Arc<Vec<u8>>,
        /// What each seat is called (the record names nobody).
        names: Vec<String>,
        /// The record's file name (the tape header's `game`).
        file: String,
        /// Whether it was opened by hand (`BAYLEE_REPLAY`): every seat's
        /// perspective is offered then, not only the player's.
        by_hand: bool,
    },
    /// A tape file opened by hand.
    Tape {
        /// The file, gzipped as a gateway sends a tape.
        gzip: Arc<Vec<u8>>,
    },
    /// A hosted game, re-run by the gateway.
    Hosted(HostedAccess),
}

/// The place a new tape opens at: the record input of the step shown
/// (`None` inside: before the first input).
#[derive(Clone, Copy, Debug)]
struct Kept(Option<u64>);

/// A tape being made.
struct Job {
    /// Behind a lock only because a host must be `Sync`; one thread reads it.
    answer: Mutex<Receiver<Result<Tape, String>>>,
    progress: Arc<AtomicU8>,
    cancel: Arc<AtomicBool>,
    perspective: Perspective,
    verify: Verify,
    /// The record input to open at (a switch keeps the place).
    keep: Option<Kept>,
}

/// A tape being shown.
struct Shown {
    tape: Arc<Tape>,
    timeline: Arc<Timeline>,
    transport: Transport,
}

/// A host that plays a replay's tape.
pub struct ReplayHost {
    source: ReplaySource,
    perspective: Perspective,
    verify: Verify,
    offers: Perspectives,
    origin: Origin,
    shown: Option<Shown>,
    job: Option<Job>,
    out: Vec<HostMessage>,
    status: ReplayStatus,
    /// The `seq` last sent.
    seq: u64,
    /// The record a report about this replay offers, for a local source.
    record: Option<Arc<Vec<u8>>>,
}

impl ReplayHost {
    /// A host for `source`, starting to make its tape at once.
    #[must_use]
    pub fn new(
        source: ReplaySource,
        perspective: Perspective,
        verify: Verify,
        offers: Perspectives,
        origin: Origin,
    ) -> Self {
        let record = match &source {
            ReplaySource::Record { bytes, .. } => Some(Arc::clone(bytes)),
            _ => None,
        };
        let mut host = Self {
            source,
            perspective,
            verify,
            offers,
            origin,
            shown: None,
            job: None,
            out: Vec::new(),
            status: ReplayStatus {
                header: None,
                step: 0,
                steps: 0,
                playing: false,
                speed: baylee_client_core::replay::Speed::One,
                timeline: None,
                stop: None,
                check: None,
                perspective,
                offers,
                verify,
                loading: None,
                problem: None,
                origin: Origin::default(),
                seats: Vec::new(),
            },
            seq: 0,
            record,
        };
        host.start(perspective, verify, None);
        host.refresh_status();
        host
    }

    /// Installs it behind the renderer.
    #[must_use]
    pub fn into_installed(self) -> crate::InstalledHost {
        crate::InstalledHost(Box::new(self))
    }

    /// Starts making the tape for `perspective` under `verify`, opening at
    /// `keep`'s input once it is made.
    fn start(&mut self, perspective: Perspective, verify: Verify, keep: Option<Kept>) {
        if let Some(old) = self.job.take() {
            old.cancel.store(true, Ordering::Relaxed);
        }
        let progress = Arc::new(AtomicU8::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, answer) = mpsc::channel();
        match &self.source {
            ReplaySource::Record {
                bytes,
                names,
                file,
                by_hand,
            } => {
                let header = Header {
                    tape: baylee_client_core::replay::TAPE_VERSION,
                    view: baylee_view::VIEW_VERSION,
                    build: baylee_build::short().to_string(),
                    source: if *by_hand {
                        baylee_client_core::replay::Source::File
                    } else {
                        baylee_client_core::replay::Source::Local
                    },
                    perspective: baylee_client_core::replay::tape_perspective(perspective),
                    seat: perspective.seat().get(),
                    game: file.clone(),
                    recorded_by: String::new(),
                };
                let (bytes, names) = (Arc::clone(bytes), names.clone());
                let (progress, cancel) = (Arc::clone(&progress), Arc::clone(&cancel));
                std::thread::Builder::new()
                    .name("replay".into())
                    .spawn(move || {
                        let tape = super::local::project(
                            &bytes,
                            names,
                            header,
                            perspective,
                            verify,
                            &progress,
                            &cancel,
                        );
                        let _ = tx.send(tape);
                    })
                    .ok();
            }
            ReplaySource::Tape { gzip } => {
                let gzip = Arc::clone(gzip);
                std::thread::Builder::new()
                    .name("replay".into())
                    .spawn(move || {
                        let _ = tx.send(Tape::read(&gzip).map_err(|e| e.to_string()));
                    })
                    .ok();
            }
            ReplaySource::Hosted(access) => fetch_tape(access.clone(), perspective, verify, tx),
        }
        self.job = Some(Job {
            answer: Mutex::new(answer),
            progress,
            cancel,
            perspective,
            verify,
            keep,
        });
    }

    /// Takes a finished tape in, if one is ready.
    fn take_job(&mut self) {
        let Some(job) = self.job.as_ref() else {
            return;
        };
        let received = match job.answer.lock() {
            Ok(answer) => answer.try_recv(),
            Err(_) => Err(TryRecvError::Disconnected),
        };
        let answer = match received {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("the replay could not be made".to_string()),
        };
        let job = self.job.take().expect("checked above");
        match answer {
            Ok(tape) if tape.frames.is_empty() => {
                self.status.problem = Some(stopped_words(&tape));
            }
            Ok(tape) => {
                let tape = Arc::new(tape);
                let timeline = Arc::new(Timeline::of(&tape));
                let at = job.keep.map_or(0, |Kept(n)| tape.step_at_input(n));
                let (playing, speed) = self
                    .shown
                    .as_ref()
                    .map_or((false, baylee_client_core::replay::Speed::default()), |s| {
                        (s.transport.playing, s.transport.speed)
                    });
                let was_shown = self.shown.is_some();
                let mut transport = Transport::at(at);
                transport.playing = playing && at < timeline.last();
                transport.speed = speed;
                // A tape says whose table it is: the gateway's word for a
                // hosted seat (read from the table, never from the ask).
                self.perspective = match self.source {
                    ReplaySource::Record { .. } => job.perspective,
                    ReplaySource::Tape { .. } | ReplaySource::Hosted(_) => {
                        baylee_client_core::replay::perspective_of(&tape.header)
                    }
                };
                self.verify = job.verify;
                self.status.problem = None;
                self.shown = Some(Shown {
                    tape,
                    timeline,
                    transport,
                });
                if was_shown {
                    self.emit(Out::Jump(at));
                } else {
                    self.open_at(at);
                }
            }
            Err(why) => self.status.problem = Some(why),
        }
    }

    /// The table's first moment: as a socket's first frames.
    fn open_at(&mut self, at: usize) {
        let Some(shown) = self.shown.as_ref() else {
            return;
        };
        let tape = Arc::clone(&shown.tape);
        self.place(&tape, at);
    }

    /// The static, the view at `at` with the whole log to it, and the end
    /// if `at` is the last step.
    fn place(&mut self, tape: &Tape, at: usize) {
        match tape.statics_at(at) {
            Some(Ok(statics)) => self.out.push(HostMessage::Static(Box::new(statics))),
            Some(Err(why)) => self.out.push(HostMessage::Failed(why)),
            None => {}
        }
        let log = tape.log_to(at);
        let log = (!log.entries.is_empty()).then_some(log);
        self.push_view(tape, at, log);
        self.push_end(tape, at);
    }

    fn push_view(&mut self, tape: &Tape, at: usize, log: Option<baylee_view::LogTail>) {
        match tape.view(at) {
            Ok(mut view) => {
                self.seq += 1;
                view.seq = self.seq;
                // A replay runs no clock (§7.3): a hosted game's view still
                // says how long its question had left when it was made.
                view.decision_remaining_ms = None;
                view.clocks.clear();
                self.out.push(HostMessage::View(Box::new(view), log));
            }
            Err(why) => self.out.push(HostMessage::Failed(why)),
        }
    }

    fn push_end(&mut self, tape: &Tape, at: usize) {
        if at + 1 >= tape.frames.len()
            && let Some(over) = tape.over()
        {
            self.out.push(match over {
                Ok(pending) => HostMessage::Choice(Box::new(pending)),
                Err(why) => HostMessage::Failed(why),
            });
        }
    }

    /// What the transport asked for, as frames.
    fn emit(&mut self, out: Out) {
        let Some(tape) = self.shown.as_ref().map(|s| Arc::clone(&s.tape)) else {
            return;
        };
        match out {
            Out::Show(at) => {
                if tape.static_arrives_at(at)
                    && let Some(statics) = tape.statics_at(at)
                {
                    self.out.push(match statics {
                        Ok(statics) => HostMessage::Static(Box::new(statics)),
                        Err(why) => HostMessage::Failed(why),
                    });
                }
                let tail = tape.tail_at(at);
                self.push_view(&tape, at, tail);
                self.push_end(&tape, at);
            }
            Out::Jump(at) => {
                self.out.push(HostMessage::Jump);
                self.place(&tape, at);
            }
        }
    }

    /// The record input of the step shown: what a new tape opens at.
    fn place_kept(&self) -> Option<Kept> {
        let shown = self.shown.as_ref()?;
        Some(Kept(
            shown
                .tape
                .frames
                .get(shown.transport.at)
                .and_then(|frame| frame.n),
        ))
    }

    fn refresh_status(&mut self) {
        let status = &mut self.status;
        status.perspective = self.perspective;
        status.verify = self.verify;
        status.offers = self.offers;
        if let Some(shown) = &self.shown {
            status.header = Some(shown.tape.header.clone());
            status.step = shown.transport.at;
            status.steps = shown.timeline.len();
            status.playing = shown.transport.playing;
            status.speed = shown.transport.speed;
            if status
                .timeline
                .as_ref()
                .is_none_or(|t| !Arc::ptr_eq(&t.0, &shown.timeline))
            {
                status.timeline = Some(SharedTimeline(Arc::clone(&shown.timeline)));
            }
            status.stop = Some(shown.tape.stop);
            if status.seats.is_empty()
                && let Some(Ok(statics)) = shown.tape.statics_at(0)
            {
                status.seats = statics
                    .seats
                    .iter()
                    .map(|s| super::SeatName {
                        seat: s.player,
                        name: s.display_name.clone(),
                    })
                    .collect();
            }
            status.check = shown.timeline.step(shown.transport.at).map(|m| m.check);
        }
        status.loading = self.job.as_ref().map(|job| match &self.source {
            ReplaySource::Hosted(access) => Loading::Gateway {
                name: access.name.clone(),
            },
            ReplaySource::Record { .. } => Loading::Here {
                percent: Some(job.progress.load(Ordering::Relaxed)),
            },
            ReplaySource::Tape { .. } => Loading::Here { percent: None },
        });
        let mut origin = self.origin.clone();
        if origin.started.is_none() {
            origin.started = self
                .shown
                .as_ref()
                .and_then(|s| s.tape.frames.first())
                .map(|f| f.at)
                .filter(|at| *at > 0);
        }
        status.origin = origin;
    }
}

/// What a tape with no step at all says: why it stopped before the first.
fn stopped_words(tape: &Tape) -> String {
    use baylee_client_core::replay::StopReason;
    match tape.stop.stop {
        StopReason::Refused => "This version can't play this game.".to_string(),
        StopReason::Diverged => {
            "This version would play this game differently from its start.".to_string()
        }
        _ => "This replay has nothing to show.".to_string(),
    }
}

impl DuelHost for ReplayHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        self.take_job();
        self.refresh_status();
        std::mem::take(&mut self.out)
    }

    fn submit(&mut self, _action: PlayerAction) {}

    fn ready(&mut self) {
        // The duel's one curtain: a tape's `curtain` line is a time on its
        // timeline, never sent again.
        self.out.push(HostMessage::Curtain);
    }

    fn seat(&self) -> PlayerId {
        self.perspective.seat()
    }

    fn watching(&self) -> bool {
        true
    }

    fn local_record(&self) -> Option<&[u8]> {
        self.record.as_deref().map(Vec::as_slice)
    }

    fn replay(&self) -> Option<&ReplayStatus> {
        Some(&self.status)
    }

    fn control(&mut self, ask: ReplayAsk) {
        match ask {
            ReplayAsk::Transport(command) => {
                if let Some(shown) = self.shown.as_mut()
                    && let Some(out) = shown.transport.apply(command, &shown.timeline)
                {
                    self.emit(out);
                }
            }
            ReplayAsk::Perspective(perspective) => {
                if perspective != self.perspective && self.status.choices().contains(&perspective) {
                    let keep = self.place_kept();
                    self.start(perspective, self.verify, keep);
                }
            }
            ReplayAsk::NextPerspective => {
                let next = self.status.next_choice();
                if next != self.perspective {
                    let keep = self.place_kept();
                    self.start(next, self.verify, keep);
                }
            }
            ReplayAsk::PlayOn => {
                let keep = self.place_kept();
                self.start(self.perspective, Verify::Note, keep);
            }
            ReplayAsk::Cancel => {
                if let Some(job) = self.job.take() {
                    job.cancel.store(true, Ordering::Relaxed);
                    if self.shown.is_none() {
                        self.status.problem = Some("Cancelled.".to_string());
                    }
                }
            }
        }
        self.refresh_status();
    }

    fn game_id(&self) -> Option<&str> {
        match &self.source {
            ReplaySource::Hosted(access) => Some(&access.game),
            _ => None,
        }
    }
}

// ------------------------------------------------------------ hosted tapes

/// The tapes this session fetched, newest last: the last three stay in
/// memory, keyed by everything that makes a tape what it is (the session
/// too: another account on the same gateway is another seat). Nothing is
/// written to disk.
static TAPES: LazyLock<Mutex<Vec<KeptTape>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// How many fetched tapes are kept.
const KEPT_TAPES: usize = 3;

type TapeKey = (String, String, String, &'static str, &'static str);

/// A fetched tape, gzipped, under its key.
type KeptTape = (TapeKey, Arc<Vec<u8>>);

fn tape_key(access: &HostedAccess, perspective: Perspective, verify: Verify) -> TapeKey {
    (
        access.gateway.clone(),
        access.token.clone(),
        access.game.clone(),
        perspective.wire(),
        verify.wire(),
    )
}

/// Forgets every fetched tape (signed out).
pub fn forget_tapes() {
    if let Ok(mut tapes) = TAPES.lock() {
        tapes.clear();
    }
}

/// Asks the gateway for a tape (`POST /replays/{game}/tape`), and hands
/// `tx` the tape read, or why there is none. A `502` (an engine replaced
/// under the job by a deploy) is asked once more.
fn fetch_tape(
    access: HostedAccess,
    perspective: Perspective,
    verify: Verify,
    tx: mpsc::Sender<Result<Tape, String>>,
) {
    let key = tape_key(&access, perspective, verify);
    let kept = TAPES.lock().ok().and_then(|tapes| {
        tapes
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, t)| Arc::clone(t))
    });
    if let Some(gzip) = kept {
        std::thread::Builder::new()
            .name("replay".into())
            .spawn(move || {
                let _ = tx.send(Tape::read(&gzip).map_err(|e| e.to_string()));
            })
            .ok();
        return;
    }
    ask(access, perspective, verify, key, tx, true);
}

fn ask(
    access: HostedAccess,
    perspective: Perspective,
    verify: Verify,
    key: TapeKey,
    tx: mpsc::Sender<Result<Tape, String>>,
    again: bool,
) {
    let url = format!(
        "{}/replays/{}/tape",
        access.gateway.trim_end_matches('/'),
        access.game
    );
    // Any seat's view (§18.3): the gateway draws it, masked where that
    // seat's player did not share.
    let body = match perspective {
        Perspective::Public => {
            serde_json::json!({ "perspective": perspective.wire(), "verify": verify.wire() })
        }
        Perspective::Seat(seat) | Perspective::AllHands(seat) => serde_json::json!({
            "perspective": perspective.wire(),
            "seat": seat.get(),
            "verify": verify.wire(),
        }),
    };
    let mut request = ehttp::Request::post(url, serde_json::to_vec(&body).unwrap_or_default());
    request.headers = ehttp::Headers::new(&[
        ("Accept", "application/gzip"),
        ("Content-Type", "application/json"),
    ]);
    request
        .headers
        .insert("Authorization", format!("Bearer {}", access.token));
    // A projection takes seconds; the gateway gives a job sixty.
    request.timeout = Some(std::time::Duration::from_secs(75));
    crate::transport::fetch(request, move |answer| match answer {
        Ok(response) if response.ok => {
            // Gzip as the route sends it; JSON Lines if something on the
            // way inflated it (a `Content-Encoding` honoured in transit).
            let gzip = Arc::new(if response.bytes.first() == Some(&b'{') {
                baylee_client_core::bugreport::gzip(&response.bytes)
            } else {
                response.bytes
            });
            if let Ok(mut tapes) = TAPES.lock() {
                tapes.retain(|(k, _)| *k != key);
                tapes.push((key, Arc::clone(&gzip)));
                let over = tapes.len().saturating_sub(KEPT_TAPES);
                tapes.drain(..over);
            }
            let _ = tx.send(Tape::read(&gzip).map_err(|e| e.to_string()));
        }
        Ok(response) if response.status == 502 && again => {
            ask(access, perspective, verify, key, tx, false);
        }
        // The game ended a moment ago and the gateway does not yet know
        // (`409`, still being played): once more, a second later.
        Ok(response) if response.status == 409 && again => {
            std::thread::sleep(std::time::Duration::from_secs(1));
            ask(access, perspective, verify, key, tx, false);
        }
        Ok(response) => {
            #[derive(serde::Deserialize)]
            struct Refused {
                error: String,
            }
            let said = serde_json::from_slice::<Refused>(&response.bytes).map_or_else(
                |_| String::from_utf8_lossy(&response.bytes).trim().to_string(),
                |refused| refused.error,
            );
            let _ = tx.send(Err(if said.is_empty() {
                format!("{} {}", response.status, response.status_text)
            } else {
                said
            }));
        }
        Err(why) => {
            let _ = tx.send(Err(why));
        }
    });
}
