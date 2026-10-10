//! A seat agent (`baylee-seathost`): hosted language-model profiles on the
//! operator's machine, one `baylee-seat` bridge per hosted chair
//! (`docs/llm-seat.md` §"A hosted seat", `docs/protocol.md` §"Hosted
//! language-model seats").
//!
//! It dials the gateway's `/seathost/ws` as an engine agent dials
//! `/agent/ws`, and keeps everything that is the operator's on its own
//! machine: the profiles (`hosted.json` in its state directory, written
//! only through the gateway's console or by hand), the keys (its key
//! store), the spend books (`spend/<id>.json`, one per profile, so each
//! profile's caps are its own) and the CLIs' sign-ins (its user's home).
//! The gateway is told states, games and spend, never a key.
//!
//! Each hosted chair is one bridge, given a settings file that holds its
//! profile alone (the profile's caps as the file's) and the chair ticket on
//! its stdin, as a host's client starts one (`--tethered --chair-ticket`),
//! with `--hosted`: it names no player to the model.

#![warn(missing_docs)]

pub mod launch;
pub mod state;

use baylee_client_core::llmseat::keys::{self, KeyState, KeyStore};
use baylee_client_core::llmseat::ledger::{Book, Moment};
use baylee_client_core::llmseat::{Caps, Profile, Provider, SeatSettings};
use baylee_protocol::seathost::{
    ControlAction, Definition, Frame, KeyKept, Reported, SeatStatusKind, Spent, State, valid_label,
    valid_name,
};
use futures_util::{SinkExt as _, StreamExt as _};
use launch::{Exit, Launcher, SeatJob};
use state::{Facts, Probe, Runtime};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

/// The profiles file in the state directory.
pub const PROFILES_FILE: &str = "hosted.json";

/// How often the seat agent looks for checks that are due.
const TICK: Duration = Duration::from_secs(5);

/// The first wait before dialling again, and the longest.
const RETRY_START: Duration = Duration::from_secs(1);
const RETRY_MAX: Duration = Duration::from_secs(30);

/// What a seat agent is.
pub struct Config {
    /// Its stable name (`BAYLEE_SEATHOST_NAME`).
    pub name: String,
    /// The gateway: `http(s)://…` or `unix:<path>`.
    pub gateway: String,
    /// `BAYLEE_SEATHOST_TOKEN`.
    pub token: String,
    /// Bridges at once; 0 = no bound.
    pub capacity: u32,
    /// Where its profiles, books and runs are.
    pub state_dir: PathBuf,
    /// Where its bridges dial, when not the address the gateway names (a
    /// seat agent on the unix socket: bridges speak HTTP).
    pub bridge_gateway: Option<String>,
    /// The key store its bridges read (`BAYLEE_KEY_STORE`).
    pub keys: Arc<dyn KeyStore>,
    /// How bridges and checks run.
    pub launcher: Arc<dyn Launcher>,
}

/// One order's bridge.
struct Order {
    profile: String,
    stop: Option<oneshot::Sender<()>>,
}

/// What the seat agent keeps in memory.
#[derive(Default)]
struct Inner {
    defs: BTreeMap<String, Definition>,
    rt: BTreeMap<String, Runtime>,
    orders: HashMap<String, Order>,
    draining: bool,
}

/// A running seat agent.
pub struct Host {
    config: Config,
    inner: Mutex<Inner>,
    /// The current connection's outgoing frames.
    out: Mutex<Option<mpsc::UnboundedSender<Frame>>>,
}

/// The clock, in Unix seconds.
#[must_use]
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Why a definition cannot be kept, or `None`.
#[must_use]
pub fn definition_fault(id: &str, def: &Definition) -> Option<String> {
    if !valid_name(id) {
        return Some("a profile's id is 1-64 of A-Z a-z 0-9 . - _".into());
    }
    if !valid_label(&def.label) {
        return Some("a label is 1-40 characters, no control character".into());
    }
    if !valid_label(&def.vendor) {
        return Some("a vendor is 1-40 characters, no control character".into());
    }
    if def.max_games == Some(0) {
        return Some("max_games is at least 1, or absent for no bound".into());
    }
    settings_of(id, def).err()
}

/// The settings file a bridge for `def` is given: the profile alone, the
/// default, its caps as the file's. Every refusal of the settings file
/// holds (no key, no key-shaped value).
///
/// # Errors
/// The settings file's refusal, one sentence.
pub fn settings_of(id: &str, def: &Definition) -> Result<SeatSettings, String> {
    let profile: Profile = serde_json::from_value(def.profile.clone())
        .map_err(|e| format!("the profile is not one of the settings file: {e}"))?;
    let caps: Caps = match &def.caps {
        Some(caps) => serde_json::from_value(caps.clone())
            .map_err(|e| format!("the caps are not the settings file's: {e}"))?,
        None => Caps::default(),
    };
    let settings = SeatSettings {
        default: Some(id.to_string()),
        caps,
        profiles: BTreeMap::from([(id.to_string(), profile)]),
    };
    settings.check()?;
    Ok(settings)
}

impl Host {
    /// A seat agent with the profiles its state directory holds.
    ///
    /// # Errors
    /// A profiles file that is there and not readable, or holds a profile
    /// the settings file would refuse.
    pub fn new(config: Config) -> Result<Arc<Self>, String> {
        let defs = read_profiles(&config.state_dir.join(PROFILES_FILE))?;
        let at = now();
        let rt = defs
            .keys()
            .map(|id| (id.clone(), Runtime::fresh(at)))
            .collect();
        Ok(Arc::new(Self {
            config,
            inner: Mutex::new(Inner {
                defs,
                rt,
                ..Inner::default()
            }),
            out: Mutex::new(None),
        }))
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn send(&self, frame: Frame) {
        if let Some(out) = self
            .out
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            let _ = out.send(frame);
        }
    }

    /// Frames to the gateway go to this channel from now on (a test's).
    #[cfg(test)]
    fn attach(&self) -> mpsc::UnboundedReceiver<Frame> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.out.lock().unwrap_or_else(PoisonError::into_inner) = Some(tx);
        rx
    }

    fn book(&self, id: &str) -> Book {
        Book::new(
            self.config
                .state_dir
                .join("spend")
                .join(format!("{id}.json")),
        )
    }

    /// Every profile as the gateway is told it.
    #[must_use]
    pub fn report(&self) -> Vec<Reported> {
        let at = now();
        let inner = self.inner();
        let live = inner.orders.len();
        let at_capacity = self.config.capacity > 0 && live >= self.config.capacity as usize;
        inner
            .defs
            .iter()
            .map(|(id, def)| {
                let rt = inner.rt.get(id).cloned().unwrap_or_default();
                let profile: Option<Profile> = serde_json::from_value(def.profile.clone()).ok();
                let caps: Caps = def
                    .caps
                    .as_ref()
                    .and_then(|c| serde_json::from_value(c.clone()).ok())
                    .unwrap_or_default();
                let moment = Moment {
                    unix: at,
                    offset: None,
                };
                let ledger = self.book(id).read().unwrap_or_default();
                let day = ledger.on_day(&moment.day());
                let month = ledger.in_month(&moment.month());
                let facts = Facts {
                    enabled: def.enabled && !inner.draining,
                    max_games: def.max_games,
                    at_capacity,
                    capped_until: state::capped_until(&caps, &day, &month, at),
                };
                let (state, until) = state::state(&rt, facts, at);
                Reported {
                    id: id.clone(),
                    label: def.label.clone(),
                    vendor: def.vendor.clone(),
                    kind: match profile.as_ref().map(|p| p.provider) {
                        Some(Provider::Cli) => "cli".into(),
                        _ => "api".into(),
                    },
                    model: profile
                        .as_ref()
                        .map(|p| p.model.clone())
                        .unwrap_or_default(),
                    state,
                    until_unix: until,
                    games: rt.games,
                    max_games: def.max_games,
                    enabled: def.enabled,
                    canary: def.canary,
                    caps: def.caps.clone(),
                    spent: Spent {
                        day_usd: day.usd,
                        month_usd: month.usd,
                        day_tokens: day.tokens,
                        month_tokens: month.tokens,
                    },
                    key: profile.map_or(KeyKept::NoneNeeded, |p| self.key_kept(&p)),
                    last_error: rt.last_error.clone(),
                    last_ok_unix: rt.last_ok,
                    definition: def.clone(),
                }
            })
            .collect()
    }

    fn key_kept(&self, profile: &Profile) -> KeyKept {
        match keys::entry(profile, None) {
            None => KeyKept::NoneNeeded,
            Some(entry) => match keys::state(&*self.config.keys, &entry) {
                KeyState::Set => KeyKept::Kept,
                KeyState::Absent => KeyKept::Absent,
                KeyState::Unavailable(_) => KeyKept::Unavailable,
            },
        }
    }

    /// Tells the gateway every profile again.
    pub fn report_all(&self) {
        self.send(Frame::Profiles {
            profiles: self.report(),
        });
    }

    /// Stops taking chairs: every profile reads `disabled`, a new order is
    /// refused, the bridges playing finish their games.
    pub fn drain(&self) {
        self.inner().draining = true;
        self.report_all();
    }

    /// Bridges playing now.
    #[must_use]
    pub fn live(&self) -> usize {
        self.inner().orders.len()
    }

    /// Runs every check that is due.
    pub fn tick(self: &Arc<Self>) {
        let at = now();
        let due: Vec<String> = self
            .inner()
            .rt
            .iter()
            .filter(|(_, rt)| rt.probe_due(at))
            .map(|(id, _)| id.clone())
            .collect();
        for id in due {
            self.probe(&id, false);
        }
    }

    /// Checks one profile now; `admin` asked for it (and its canary, where
    /// it has one).
    fn probe(self: &Arc<Self>, id: &str, asked: bool) {
        let (settings, canary) = {
            let mut inner = self.inner();
            let Some(def) = inner.defs.get(id).cloned() else {
                return;
            };
            let Ok(settings) = settings_of(id, &def) else {
                return;
            };
            let Some(rt) = inner.rt.get_mut(id) else {
                return;
            };
            if rt.probing {
                return;
            }
            rt.probing = true;
            (settings.to_json(), def.canary && asked)
        };
        self.report_all();
        let host = Arc::clone(self);
        let id = id.to_string();
        tokio::spawn(async move {
            let probe = host
                .config
                .launcher
                .probe(settings, id.clone(), canary)
                .await;
            tracing::info!(profile = id, ok = matches!(probe, Probe::Ok), "checked");
            if let Some(rt) = host.inner().rt.get_mut(&id) {
                rt.probed(probe, now());
            }
            host.report_all();
        });
    }

    /// Acts on one frame from the gateway.
    pub fn handle(self: &Arc<Self>, frame: Frame) {
        match frame {
            Frame::Welcome { .. } => self.report_all(),
            Frame::Refused { message } => tracing::error!(message, "the gateway refused"),
            Frame::StartSeat {
                order,
                game_id,
                seat,
                profile,
                chair_ticket,
                gateway_url,
                deck_text,
            } => self.start(SeatJob {
                order,
                game_id,
                seat,
                name: String::new(),
                profile,
                chair_ticket,
                gateway_url: self.config.bridge_gateway.clone().unwrap_or(gateway_url),
                deck_text,
                settings: String::new(),
                ledger: PathBuf::new(),
            }),
            Frame::StopSeat { order } => {
                let stop = self
                    .inner()
                    .orders
                    .get_mut(&order)
                    .and_then(|o| o.stop.take());
                if let Some(stop) = stop {
                    tracing::info!(order, "stopping a bridge");
                    let _ = stop.send(());
                }
            }
            Frame::ProfileWrite {
                request,
                id,
                definition,
                admin,
            } => {
                let done = self.write(&id, definition);
                tracing::info!(admin, profile = id, ok = done.is_ok(), "llm.profile.write");
                self.answer(request, done);
            }
            Frame::ProfileDelete { request, id, admin } => {
                let done = self.delete(&id);
                tracing::info!(admin, profile = id, ok = done.is_ok(), "llm.profile.delete");
                self.answer(request, done);
            }
            Frame::Control {
                request,
                id,
                action,
                admin,
            } => {
                let done = self.control(&id, action);
                tracing::info!(
                    admin,
                    profile = id,
                    ?action,
                    ok = done.is_ok(),
                    "llm.profile.control"
                );
                self.answer(request, done);
            }
            Frame::KeyWrite {
                request,
                id,
                key,
                admin,
            } => {
                let set = key.is_some();
                let done = self.key(&id, key);
                tracing::info!(
                    admin,
                    profile = id,
                    action = if set { "llm.key.set" } else { "llm.key.delete" },
                    ok = done.is_ok(),
                    "llm.key"
                );
                self.answer(request, done);
            }
            Frame::Hello { .. }
            | Frame::Profiles { .. }
            | Frame::SeatStatus { .. }
            | Frame::Answer { .. }
            | Frame::Heartbeat => {}
        }
    }

    fn answer(&self, request: String, done: Result<(), String>) {
        let (ok, error) = match done {
            Ok(()) => (true, None),
            Err(why) => (false, Some(why)),
        };
        self.send(Frame::Answer { request, ok, error });
        self.report_all();
    }

    fn status(&self, order: &str, kind: SeatStatusKind, detail: &str) {
        self.send(Frame::SeatStatus {
            order: order.to_string(),
            kind,
            detail: detail.to_string(),
        });
    }

    /// Starts the bridge for a chair, if the profile may take it now.
    fn start(self: &Arc<Self>, mut job: SeatJob) {
        let state = self
            .report()
            .into_iter()
            .find(|p| p.id == job.profile)
            .map(|p| p.state);
        let refusal = match state {
            None => Some("this seat agent has no such profile".to_string()),
            Some(_) if self.inner().draining => Some("draining".into()),
            Some(State::Available) => None,
            Some(state) => Some(format!("busy: the profile is {}", state.word())),
        };
        let prepared = if let Some(why) = refusal {
            Err(why)
        } else {
            let mut inner = self.inner();
            let def = inner.defs.get(&job.profile).cloned();
            match def.map(|def| (settings_of(&job.profile, &def), def)) {
                Some((Ok(settings), def)) => {
                    job.name = baylee_protocol::seathost::chair_name(&def.label, &job.profile);
                    job.settings = settings.to_json();
                    job.ledger = self
                        .config
                        .state_dir
                        .join("spend")
                        .join(format!("{}.json", job.profile));
                    let (stop_tx, stop_rx) = oneshot::channel();
                    inner.orders.insert(
                        job.order.clone(),
                        Order {
                            profile: job.profile.clone(),
                            stop: Some(stop_tx),
                        },
                    );
                    if let Some(rt) = inner.rt.get_mut(&job.profile) {
                        rt.games += 1;
                    }
                    Ok(stop_rx)
                }
                Some((Err(why), _)) => Err(why),
                None => Err("this seat agent has no such profile".into()),
            }
        };
        let stop = match prepared {
            Ok(stop) => stop,
            Err(why) => {
                tracing::info!(
                    order = job.order,
                    profile = job.profile,
                    why,
                    "order refused"
                );
                self.status(&job.order, SeatStatusKind::Failed, &why);
                return;
            }
        };
        tracing::info!(?job, "starting a bridge");
        self.report_all();
        let _ = std::fs::create_dir_all(self.config.state_dir.join("spend"));
        let host = Arc::clone(self);
        let order = job.order.clone();
        let profile = job.profile.clone();
        let (started_tx, started_rx) = oneshot::channel();
        let run = self.config.launcher.run(job, started_tx, stop);
        let announcer = {
            let host = Arc::clone(self);
            let order = order.clone();
            tokio::spawn(async move {
                if started_rx.await.is_ok() {
                    host.status(&order, SeatStatusKind::Started, "");
                }
            })
        };
        tokio::spawn(async move {
            let exit = run.await;
            announcer.abort();
            let (kind, detail, outcome) = match exit {
                Exit::Ended => (SeatStatusKind::Exited, String::new(), Ok(())),
                Exit::Failed(probe) => {
                    let detail = match &probe {
                        Probe::Ok => String::new(),
                        Probe::SignedOut(why)
                        | Probe::Failing(why)
                        | Probe::Limited { why, .. } => why.clone(),
                    };
                    (SeatStatusKind::Failed, detail, Err(probe))
                }
            };
            tracing::info!(order, profile, ?kind, detail, "bridge ended");
            {
                let mut inner = host.inner();
                inner.orders.remove(&order);
                if let Some(rt) = inner.rt.get_mut(&profile) {
                    rt.ended(outcome, now());
                }
            }
            host.status(&order, kind, &detail);
            host.report_all();
        });
    }

    fn write(&self, id: &str, def: Definition) -> Result<(), String> {
        if let Some(why) = definition_fault(id, &def) {
            return Err(why);
        }
        let mut inner = self.inner();
        let mut defs = inner.defs.clone();
        defs.insert(id.to_string(), def);
        write_profiles(&self.config.state_dir.join(PROFILES_FILE), &defs)?;
        inner.defs = defs;
        // A changed profile is checked again at once.
        let rt = inner.rt.entry(id.to_string()).or_default();
        rt.next_probe = Some(now());
        rt.failing = None;
        rt.needs_login = false;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let mut inner = self.inner();
        if !inner.defs.contains_key(id) {
            return Err("no such profile".into());
        }
        let mut defs = inner.defs.clone();
        defs.remove(id);
        write_profiles(&self.config.state_dir.join(PROFILES_FILE), &defs)?;
        inner.defs = defs;
        if inner.orders.values().all(|o| o.profile != id) {
            inner.rt.remove(id);
        }
        Ok(())
    }

    fn control(self: &Arc<Self>, id: &str, action: ControlAction) -> Result<(), String> {
        match action {
            ControlAction::Probe => {
                if !self.inner().defs.contains_key(id) {
                    return Err("no such profile".into());
                }
                self.probe(id, true);
                Ok(())
            }
            ControlAction::Enable | ControlAction::Disable => {
                let mut inner = self.inner();
                let mut defs = inner.defs.clone();
                let def = defs.get_mut(id).ok_or("no such profile")?;
                def.enabled = action == ControlAction::Enable;
                write_profiles(&self.config.state_dir.join(PROFILES_FILE), &defs)?;
                inner.defs = defs;
                Ok(())
            }
        }
    }

    /// Keeps or forgets a profile's key: the key goes into the store and
    /// nowhere else (no log, no answer).
    fn key(&self, id: &str, key: Option<String>) -> Result<(), String> {
        let profile = {
            let inner = self.inner();
            let def = inner.defs.get(id).ok_or("no such profile")?;
            serde_json::from_value::<Profile>(def.profile.clone())
                .map_err(|_| "the profile is not readable".to_string())?
        };
        let entry = keys::entry(&profile, None).ok_or("this profile takes no key")?;
        let store = &*self.config.keys;
        let done = match key {
            Some(key) => store.set(&entry, key.trim()),
            None => store.delete(&entry),
        };
        // A store's sentence names no key; blanked all the same.
        done.map_err(|why| baylee_client_core::llmseat::blank_key_shapes(&why))?;
        if let Some(rt) = self.inner().rt.get_mut(id) {
            rt.next_probe = Some(now());
            rt.failing = None;
        }
        Ok(())
    }

    /// Dials the gateway, and again whenever the link ends, forever.
    pub async fn serve(self: Arc<Self>) {
        let ticker = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(TICK).await;
                ticker.tick();
            }
        });
        let mut wait = RETRY_START;
        loop {
            match self.session().await {
                Ok(()) => wait = RETRY_START,
                Err(why) => tracing::warn!(why, "the gateway link failed"),
            }
            *self.out.lock().unwrap_or_else(PoisonError::into_inner) = None;
            tokio::time::sleep(wait).await;
            wait = (wait * 2).min(RETRY_MAX);
        }
    }

    /// The URL the handshake names.
    fn control_url(&self) -> String {
        let path = baylee_protocol::seathost::PATH;
        if baylee_protocol::unix_socket(&self.config.gateway).is_some() {
            return format!("ws://localhost{path}");
        }
        let base = self.config.gateway.trim_end_matches('/');
        let base = match base.split_once("://") {
            Some(("https", rest)) => format!("wss://{rest}"),
            Some(("http", rest)) => format!("ws://{rest}"),
            _ => base.to_string(),
        };
        format!("{base}{path}")
    }

    /// One connection, from hello to close.
    ///
    /// # Errors
    /// The dial or the socket failed, or the gateway refused.
    pub async fn session(self: &Arc<Self>) -> Result<(), String> {
        let url = self.control_url();
        #[cfg(unix)]
        if let Some(path) = baylee_protocol::unix_socket(&self.config.gateway) {
            let socket = tokio::net::UnixStream::connect(path)
                .await
                .map_err(|e| format!("dial {}: {e}", self.config.gateway))?;
            let (ws, _) = tokio_tungstenite::client_async(&url, socket)
                .await
                .map_err(|e| format!("dial {}: {e}", self.config.gateway))?;
            return self.run(ws).await;
        }
        let (ws, _) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| format!("dial {url}: {e}"))?;
        self.run(ws).await
    }

    async fn run<S>(
        self: &Arc<Self>,
        ws: tokio_tungstenite::WebSocketStream<S>,
    ) -> Result<(), String>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let (mut sink, mut stream) = ws.split();
        let hello = Frame::Hello {
            token: self.config.token.clone(),
            name: self.config.name.clone(),
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
            capacity: self.config.capacity,
        };
        sink.send(Message::text(hello.to_text()))
            .await
            .map_err(|e| e.to_string())?;
        let (tx, mut rx) = mpsc::unbounded_channel::<Frame>();
        *self.out.lock().unwrap_or_else(PoisonError::into_inner) = Some(tx.clone());
        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
        loop {
            tokio::select! {
                frame = rx.recv() => {
                    let Some(frame) = frame else { return Ok(()) };
                    sink.send(Message::text(frame.to_text())).await.map_err(|e| e.to_string())?;
                }
                _ = heartbeat.tick() => {
                    let _ = tx.send(Frame::Heartbeat);
                }
                message = stream.next() => {
                    let text = match message {
                        Some(Ok(Message::Text(text))) => text,
                        Some(Ok(Message::Close(_))) | None => return Ok(()),
                        Some(Ok(_)) => continue,
                        Some(Err(e)) => return Err(e.to_string()),
                    };
                    match Frame::parse(&text) {
                        Ok(Frame::Welcome { heartbeat_secs }) => {
                            tracing::info!(name = self.config.name, "the gateway took this seat agent");
                            heartbeat = tokio::time::interval(Duration::from_secs(
                                u64::from(heartbeat_secs.max(2) / 2),
                            ));
                            self.report_all();
                        }
                        Ok(Frame::Refused { message }) => return Err(format!("refused: {message}")),
                        Ok(frame) => self.handle(frame),
                        Err(why) => tracing::debug!(why, "a frame not read"),
                    }
                }
            }
        }
    }
}

/// Reads the profiles file; none there is none kept.
///
/// # Errors
/// A file that is there and is not the profiles' JSON, or holds a profile
/// [`definition_fault`] refuses.
pub fn read_profiles(path: &Path) -> Result<BTreeMap<String, Definition>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let defs: BTreeMap<String, Definition> =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for (id, def) in &defs {
        if let Some(why) = definition_fault(id, def) {
            return Err(format!("{}: profile «{id}»: {why}", path.display()));
        }
    }
    Ok(defs)
}

/// Writes the profiles file whole, `0600`, through a temp file.
///
/// # Errors
/// The file could not be written.
pub fn write_profiles(path: &Path, defs: &BTreeMap<String, Definition>) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("the state directory: {}", e.kind()))?;
    }
    let mut text = serde_json::to_string_pretty(defs).map_err(|e| e.to_string())?;
    text.push('\n');
    let temp = path.with_extension("json.tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
        .open(&temp)
        .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()))
        .and_then(|()| std::fs::rename(&temp, path))
        .map_err(|e| format!("the profiles file could not be written: {}", e.kind()))
}

#[cfg(test)]
mod tests;
