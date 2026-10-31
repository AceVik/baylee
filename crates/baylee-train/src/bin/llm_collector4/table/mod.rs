//! Async table worker: host session, lobby presence, seat subprocess guard, and per-game collection.
use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow, bail, ensure};
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Semaphore, oneshot};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;

use crate::audit::{
    audit_record, audit_record_setup, audit_tape, audit_transcript, completed_record_end,
    export_game_record, ordered_attribution, read_jsonl, sha256_file, transcript_fallbacks,
    validate_technical_result, write_atomic_json,
};
use crate::schedule::{
    Assignment, Chain, DeckInventory, collector_room_setup, derive_cli_decks, house_level,
    parse_deck_inventory, requested_chain,
};
use crate::teacher::{OwnDatabase, Sits};
use baylee_db::sea_orm::DatabaseConnection;

pub fn gateway_endpoint(base: &str) -> anyhow::Result<()> {
    let authority = base
        .strip_prefix("http://")
        .context("Collector gateway requires loopback HTTP")?;
    let address: std::net::SocketAddr = authority
        .parse()
        .context("Collector gateway requires a literal IP and port without credentials or path")?;
    ensure!(
        address.ip().is_loopback() && address.port() != 0,
        "Collector gateway requires a nonzero loopback port"
    );
    Ok(())
}

/// Paces table launches across all 50 workers so `POST /lobby/games/{id}/chairs/1/redeem`
/// from loopback stays strictly below `baylee-gateway`'s 30-per-60s IP rate limit.
pub struct LaunchPacer {
    next_allowed: Mutex<Instant>,
    interval: Duration,
}

impl LaunchPacer {
    pub fn new(interval: Duration) -> Self {
        Self {
            next_allowed: Mutex::new(Instant::now()),
            interval,
        }
    }

    pub async fn wait_turn(&self) {
        let sleep_dur = {
            let mut next = self.next_allowed.lock().await;
            let now = Instant::now();
            if *next <= now {
                *next = now + self.interval;
                Duration::ZERO
            } else {
                let wait = *next - now;
                *next += self.interval;
                wait
            }
        };
        if !sleep_dur.is_zero() {
            tokio::time::sleep(sleep_dur).await;
        }
    }
}

/// A searched first page can prove absence only when its complete count is
/// zero. Do not accept an empty later page or a response from another schema.
fn complete_empty_lobby_page(page: &Value) -> bool {
    page.get("offset").and_then(Value::as_u64) == Some(0)
        && page.get("limit").and_then(Value::as_u64) == Some(100)
        && page.get("total").and_then(Value::as_u64) == Some(0)
        && page
            .get("games")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
}

/// Called only after owned children exited and before any start was requested.
/// A child may already have left the host chair, so failure of this duplicate
/// leave must not suppress the independent authenticated absence check.
async fn close_unstarted_room(client: &GatewayClient, token: &str, game_id: &str) -> bool {
    let _ = client
        .post_json(
            &format!("/lobby/games/{game_id}/leave"),
            &json!({}),
            Some(token),
            5,
        )
        .await;
    client
        .get_json(
            &format!("/lobby/games?q={game_id}&offset=0&limit=100&waiting_only=false"),
            Some(token),
            5,
        )
        .await
        .is_ok_and(|page| complete_empty_lobby_page(&page))
}

#[derive(Clone)]
pub struct GatewayClient {
    base: String,
    forwarded_ip: String,
}

impl GatewayClient {
    pub fn new(base: &str, slot_id: usize) -> Self {
        let b = (slot_id / 250) % 250 + 1;
        let c = (slot_id % 250) + 1;
        Self {
            base: base.trim_end_matches('/').to_owned(),
            forwarded_ip: format!("10.250.{b}.{c}"),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub async fn get_json(
        &self,
        path: &str,
        token: Option<&str>,
        timeout_secs: u64,
    ) -> anyhow::Result<Value> {
        let url = format!("{}{path}", self.base);
        let tok = token.map(str::to_owned);
        let xff = self.forwarded_ip.clone();
        tokio::task::spawn_blocking(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .proxy(None)
                .max_redirects(0)
                .timeout_global(Some(Duration::from_secs(timeout_secs)))
                .build()
                .into();
            let mut req = agent.get(&url).header("X-Forwarded-For", &xff);
            if let Some(t) = &tok {
                req = req.header("Authorization", &format!("Bearer {t}"));
            }
            let mut resp = req.call().map_err(|e| anyhow!("GET {url} failed: {e}"))?;
            let text = resp.body_mut().read_to_string()?;
            serde_json::from_str(&text).with_context(|| format!("parsing JSON from GET {url}"))
        })
        .await?
    }

    pub async fn post_json(
        &self,
        path: &str,
        body: &Value,
        token: Option<&str>,
        timeout_secs: u64,
    ) -> anyhow::Result<Value> {
        let url = format!("{}{path}", self.base);
        let payload = serde_json::to_vec(body)?;
        let tok = token.map(str::to_owned);
        let xff = self.forwarded_ip.clone();
        tokio::task::spawn_blocking(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .proxy(None)
                .max_redirects(0)
                .timeout_global(Some(Duration::from_secs(timeout_secs)))
                .build()
                .into();
            let mut req = agent
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Forwarded-For", &xff);
            if let Some(t) = &tok {
                req = req.header("Authorization", &format!("Bearer {t}"));
            }
            let mut resp = req
                .send(payload.as_slice())
                .map_err(|e| anyhow!("POST {url} failed: {e}"))?;
            let text = resp.body_mut().read_to_string()?;
            if text.trim().is_empty() {
                Ok(json!({}))
            } else {
                serde_json::from_str(&text).with_context(|| format!("parsing JSON from POST {url}"))
            }
        })
        .await?
    }

    pub async fn post_bytes(
        &self,
        path: &str,
        body: &Value,
        token: Option<&str>,
        timeout_secs: u64,
    ) -> anyhow::Result<Vec<u8>> {
        let url = format!("{}{path}", self.base);
        let payload = serde_json::to_vec(body)?;
        let tok = token.map(str::to_owned);
        let xff = self.forwarded_ip.clone();
        tokio::task::spawn_blocking(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .proxy(None)
                .max_redirects(0)
                .timeout_global(Some(Duration::from_secs(timeout_secs)))
                .build()
                .into();
            let mut req = agent
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Forwarded-For", &xff);
            if let Some(t) = &tok {
                req = req.header("Authorization", &format!("Bearer {t}"));
            }
            let mut resp = req
                .send(payload.as_slice())
                .map_err(|e| anyhow!("POST {url} (bytes) failed: {e}"))?;
            let mut out = Vec::new();
            resp.body_mut()
                .as_reader()
                .read_to_end(&mut out)
                .with_context(|| format!("reading binary body from POST {url}"))?;
            Ok(out)
        })
        .await?
    }
}

/// Persistent lobby presence WebSocket (`/lobby/ws`) answering server pings.
pub struct Presence {
    stop: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<anyhow::Result<()>>>,
}

impl Presence {
    pub async fn connect(client: &GatewayClient, token: &str) -> anyhow::Result<Self> {
        let resp = client
            .post_json("/ws-ticket", &json!({ "socket": "lobby" }), Some(token), 15)
            .await?;
        let ticket = resp
            .get("ticket")
            .and_then(Value::as_str)
            .context("missing ticket from POST /ws-ticket")?;
        Self::connect_with_ticket(client.base(), ticket).await
    }

    pub async fn connect_with_ticket(gateway_base: &str, ticket: &str) -> anyhow::Result<Self> {
        let ws_base = if let Some(rest) = gateway_base.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = gateway_base.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            format!("ws://{gateway_base}")
        };
        let ws_url = format!(
            "{}/lobby/ws?ticket={ticket}&protocol={}&limit=1",
            ws_base.trim_end_matches('/'),
            baylee_protocol::PROTOCOL_VERSION
        );
        let request = ws_url.into_client_request()?;
        let (mut socket, _) = tokio_tungstenite::connect_async(request)
            .await
            .context("connecting lobby presence websocket")?;
        let (stop, mut stopped) = oneshot::channel();
        let worker = tokio::spawn(async move {
            let res = async {
                loop {
                    tokio::select! {
                        _ = &mut stopped => return Ok(()),
                        next = socket.next() => match next {
                            Some(Ok(Message::Ping(bytes))) => {
                                socket.send(Message::Pong(bytes)).await
                                    .map_err(|_| anyhow!("lobby presence pong failed"))?;
                            }
                            Some(Ok(Message::Close(_))) | None => bail!("lobby presence socket closed"),
                            Some(Err(e)) => bail!("lobby presence socket error: {e}"),
                            Some(Ok(_)) => {}
                        }
                    }
                }
            }
            .await;
            let _ = tokio::time::timeout(Duration::from_secs(2), socket.close(None)).await;
            res
        });
        Ok(Self {
            stop: Some(stop),
            worker: Some(worker),
        })
    }

    pub fn check(&self) -> anyhow::Result<()> {
        if self.worker.as_ref().is_none_or(JoinHandle::is_finished) {
            bail!("lobby presence ended");
        }
        Ok(())
    }

    pub async fn close(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(mut worker) = self.worker.take()
            && tokio::time::timeout(Duration::from_secs(2), &mut worker)
                .await
                .is_err()
        {
            worker.abort();
            let _ = worker.await;
        }
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            worker.abort();
        }
    }
}

/// A slot's account (its teacher, or its house: [`Sits`]) and cached uploaded deck IDs.
pub struct SlotAccount {
    pub client: GatewayClient,
    pub token: String,
    pub presence: Presence,
    pub uploaded_decks: BTreeMap<String, String>,
    /// What its chair seats, which its name says.
    pub sits: Sits,
    room_closed: bool,
}

#[derive(Debug)]
struct CleanAccountInitFailure(anyhow::Error);

impl std::fmt::Display for CleanAccountInitFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Account initialization failed after verified account deletion")
    }
}

impl std::error::Error for CleanAccountInitFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

/// Only explicit successful cleanup permits creating another account.
/// Registration or login transport failure may have committed remotely.
pub fn account_init_retry_safe(error: &anyhow::Error) -> bool {
    error.downcast_ref::<CleanAccountInitFailure>().is_some()
}

/// `DELETE /account` with the teacher's own session: since the cutover it
/// takes `{}`, no password (`docs/protocol.md`).
async fn delete_owned_account(client: &GatewayClient, token: &str) -> anyhow::Result<()> {
    let url = format!("{}/account", client.base());
    let token = token.to_owned();
    tokio::task::spawn_blocking(move || {
        let body = b"{}";
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        let response = agent
            .delete(&url)
            .force_send_body()
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .send(body.as_slice())
            .map_err(|_| anyhow!("Owned account deletion failed"))?;
        ensure!(
            response.status().as_u16() == 204,
            "Owned account deletion was not confirmed"
        );
        Ok(())
    })
    .await?
}

impl SlotAccount {
    /// The slot's account that seats `sits`, written into the collector's
    /// own gateway's database ([`crate::teacher`]) and present in the lobby
    /// at `gateway`. The presence socket's ticket is the first call with its
    /// session, and what binds `gateway` to that database: a gateway on
    /// another database does not know the session, and the teacher is
    /// deleted again.
    pub async fn init(
        own: &OwnDatabase,
        gateway: &str,
        slot_id: usize,
        sits: Sits,
    ) -> anyhow::Result<Self> {
        let client = GatewayClient::new(gateway, slot_id);
        let teacher = own.teacher(slot_id, sits).await?;
        let presence = match Presence::connect(&client, &teacher.token).await {
            Ok(presence) => presence,
            Err(error) => {
                // Through the gateway where it knows the teacher, so that
                // nothing it holds of the session outlives it; straight
                // from the database where it does not.
                if delete_owned_account(&client, &teacher.token).await.is_err() {
                    own.forget(teacher.account).await
                        .context("Lobby presence initialization failed and account cleanup is unresolved; retry refused")?;
                }
                return Err(anyhow::Error::new(CleanAccountInitFailure(error)));
            }
        };
        Ok(Self {
            client,
            token: teacher.token,
            presence,
            uploaded_decks: BTreeMap::new(),
            sits,
            room_closed: true,
        })
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.presence.close().await;
        ensure!(
            self.room_closed,
            "Account retained because owned room closure is unresolved"
        );
        delete_owned_account(&self.client, &self.token).await
    }

    pub async fn ensure_presence(&mut self) -> anyhow::Result<()> {
        if self.presence.check().is_err() {
            self.presence.close().await;
            self.presence = Presence::connect(&self.client, &self.token).await?;
        }
        Ok(())
    }

    pub async fn ensure_deck_uploaded(
        &mut self,
        key: &str,
        inv: &DeckInventory,
    ) -> anyhow::Result<String> {
        if let Some(id) = self.uploaded_decks.get(key) {
            return Ok(id.clone());
        }
        let body = serde_json::to_value(inv)?;
        let resp = self
            .client
            .post_json("/decks", &body, Some(&self.token), 20)
            .await?;
        let deck_id = resp
            .get("deck_id")
            .and_then(Value::as_str)
            .context("missing deck_id from POST /decks")?
            .to_owned();
        self.uploaded_decks.insert(key.to_owned(), deck_id.clone());
        Ok(deck_id)
    }
}

/// Guard for a `baylee-seat` child process group (`process_group(0)`).
/// Guarantees `SIGTERM` / `SIGKILL` to the entire process group on cleanup or drop.
pub struct SeatProcessGroup {
    child: Child,
    stdin: Option<tokio::process::ChildStdin>,
    pgid: i32,
    diag_task: Option<JoinHandle<()>>,
}

/// The variable an API profile reads its key from; `None` for an agent CLI,
/// which signs in by itself.
fn profile_key_env(profiles_config: &Path, profile: &str) -> anyhow::Result<Option<String>> {
    let config: Value = serde_json::from_slice(&std::fs::read(profiles_config)?)?;
    let entry = &config["profiles"][profile];
    let default = match entry["provider"].as_str() {
        Some("cli") => return Ok(None),
        Some("openai") => "BAYLEE_LLM_API_KEY",
        Some("anthropic") => "ANTHROPIC_API_KEY",
        other => bail!("profile {profile} has no known provider: {other:?}"),
    };
    Ok(Some(
        entry["key_env"].as_str().unwrap_or(default).to_owned(),
    ))
}

/// The variables a seat's bridge reads its keys from: each API profile's
/// of the chain it plays (`profile`, then its `fallbacks`), each once.
fn chain_key_envs(
    profiles_config: &Path,
    profile: &str,
    fallbacks: &[String],
) -> anyhow::Result<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for named in std::iter::once(profile).chain(fallbacks.iter().map(String::as_str)) {
        if let Some(name) = profile_key_env(profiles_config, named)?
            && !names.contains(&name)
        {
            names.push(name);
        }
    }
    Ok(names)
}

/// The last lines a bridge wrote to stderr, for its `seatN-diagnostics.json`:
/// its last line is why it stopped (`Error: …`), the one thing that says
/// why a seat's transport exited. Each is cut short, and one shaped like a
/// key is never kept (`baylee_protocol::mind::shaped_like_a_key`).
#[derive(Default)]
struct StderrTail {
    lines: std::collections::VecDeque<String>,
}

impl StderrTail {
    /// The most lines kept.
    const LINES: usize = 20;
    /// The most characters kept of one line.
    const CHARS: usize = 500;

    fn push(&mut self, line: &str) {
        let kept = if baylee_protocol::mind::shaped_like_a_key(line) {
            "[a line shaped like a key, not kept]".to_owned()
        } else {
            line.chars().take(Self::CHARS).collect()
        };
        if self.lines.len() == Self::LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(kept);
    }
}

/// One seat's bridge, as `baylee-seat join` is told it.
pub struct SeatLaunch<'a> {
    pub seat_bin: &'a Path,
    pub game_id: &'a str,
    pub seat: usize,
    pub gateway: &'a str,
    /// The pairing entry the seat plays: a profile of `profiles_config`, or
    /// `house:<level>` for the house, which reads no profile, key or spend
    /// book ([`house_level`]).
    pub profile: &'a str,
    /// The fallbacks the profile names, in the order they take over: each
    /// a `--fallback-profile` of the same profiles file and spend book.
    pub fallbacks: &'a [String],
    pub profiles_config: &'a Path,
    pub ledger: &'a Path,
    pub deck_path: &'a Path,
    pub think_secs: u64,
    pub transcripts_dir: &'a Path,
    pub work_dir: &'a Path,
    pub send_reports: bool,
}

impl SeatLaunch<'_> {
    /// The command that starts the bridge, its environment cleared to the
    /// variables it needs and the key variable of every API profile of its
    /// chain, read through `env`, never any other key. The house plays at
    /// its level with no profile, so it reserves nothing and reads no key.
    fn command(&self, env: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<Command> {
        let seat = self.seat;
        let house = house_level(self.profile);
        let mut cmd = Command::new(self.seat_bin);
        cmd.arg("join")
            .arg(self.game_id)
            .arg("--gateway")
            .arg(self.gateway)
            .arg("--chair")
            .arg(seat.to_string())
            .arg("--tethered");
        if let Some(level) = house {
            cmd.arg("--mind").arg("house").arg("--level").arg(level);
        } else {
            // The house a model falls back to plays sharp.
            cmd.arg("--profile")
                .arg(self.profile)
                .arg("--config")
                .arg(self.profiles_config)
                .arg("--ledger")
                .arg(self.ledger)
                .arg("--level")
                .arg("sharp");
            // Named explicitly, as the request froze them: each in the
            // profiles file and the slot's spend book, as the profile is.
            for fallback in self.fallbacks {
                cmd.arg("--fallback-profile").arg(fallback);
            }
        }
        cmd.arg("--deck")
            .arg(self.deck_path)
            .arg("--think-secs")
            .arg(self.think_secs.to_string())
            .arg("--transcripts")
            .arg(self.transcripts_dir)
            .arg("--name")
            .arg(format!("teacher-{seat}"))
            // Chair 0 is its host's own; so is the house's chair 1, which
            // its account joined, since a chair ticket seats only a
            // language model (`run_attempt`).
            .arg(if seat == 0 || house.is_some() {
                "--host-chair"
            } else {
                "--chair-ticket"
            });
        if !self.send_reports {
            cmd.arg("--no-send-reports");
        }
        cmd.current_dir(self.work_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .process_group(0);

        cmd.env_clear();
        for key in [
            "PATH",
            "HOME",
            "USER",
            "LOGNAME",
            "LANG",
            "LD_LIBRARY_PATH",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_STATE_HOME",
            "DBUS_SESSION_BUS_ADDRESS",
            "CODEX_HOME",
        ] {
            if let Some(val) = env(key) {
                cmd.env(key, val);
            }
        }
        // An API profile's key, on a machine without a credential store, is
        // read from its own variable (`docs/llm-seat.md` §"Where a key is
        // kept"): forward exactly those of the chain, never any other key.
        if house.is_none() {
            for name in chain_key_envs(self.profiles_config, self.profile, self.fallbacks)? {
                if let Some(val) = env(&name) {
                    cmd.env(name, val);
                }
            }
        }
        cmd.env("OMP_NUM_THREADS", "1")
            .env("MKL_NUM_THREADS", "1")
            .env("OPENBLAS_NUM_THREADS", "1")
            .env("CUDA_VISIBLE_DEVICES", "")
            .env("BAYLEE_EP", "cpu")
            .env("RUST_BACKTRACE", "0");
        Ok(cmd)
    }
}

impl SeatProcessGroup {
    pub fn spawn(launch: &SeatLaunch<'_>) -> anyhow::Result<Self> {
        let seat = launch.seat;
        let mut cmd = launch.command(&|key| std::env::var(key).ok())?;

        let child = cmd
            .spawn()
            .with_context(|| format!("spawning baylee-seat for seat {seat}"))?;
        let pid = child.id().context("missing child PID")?;
        let pgid = i32::try_from(pid).context("invalid child PID")?;

        let mut guard = Self {
            child,
            stdin: None,
            pgid,
            diag_task: None,
        };
        guard.stdin = guard.child.stdin.take();
        let diag_path = launch.work_dir.join(format!("seat{seat}-diagnostics.json"));
        let diag_task = guard.child.stderr.take().map(|stderr| {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                let mut line_count = 0_u64;
                let mut rate_limit_mentions = 0_u64;
                let mut timeout_mentions = 0_u64;
                let mut tail = StderrTail::default();
                while let Ok(Some(line)) = reader.next_line().await {
                    line_count += 1;
                    let lower = line.to_ascii_lowercase();
                    if lower.contains("rate limit") || lower.contains("429") {
                        rate_limit_mentions += 1;
                    }
                    if lower.contains("timed out") || lower.contains("timeout") {
                        timeout_mentions += 1;
                    }
                    tail.push(&line);
                }
                let _ = write_atomic_json(
                    &diag_path,
                    &json!({
                        "stderr_lines": line_count,
                        "rate_limit_mentions": rate_limit_mentions,
                        "timeout_mentions": timeout_mentions,
                        "stderr_tail": tail.lines,
                    }),
                );
            })
        });

        guard.diag_task = diag_task;
        Ok(guard)
    }

    async fn deliver_secret(&mut self, secret_line: &str) -> anyhow::Result<()> {
        tokio::time::timeout(Duration::from_secs(5), async {
            let pipe = self.stdin.as_mut().context("missing tether stdin")?;
            let mut line = secret_line.to_owned();
            line.push('\n');
            pipe.write_all(line.as_bytes()).await?;
            pipe.flush().await?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| anyhow!("Tether write timed out"))?
        .map_err(|_| anyhow!("Failed to deliver tether credentials"))
    }

    pub fn try_wait(&mut self) -> anyhow::Result<Option<std::process::ExitStatus>> {
        crate::processes::peek_exit(&mut self.child)
    }

    fn group_alive(&self) -> bool {
        self.pgid > 0 && crate::processes::group_alive(&self.child).unwrap_or(true)
    }

    pub async fn terminate(&mut self) -> anyhow::Result<()> {
        drop(self.stdin.take());
        for _ in 0..30 {
            if !self.group_alive() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        crate::processes::cleanup(&mut self.child).await?;
        self.pgid = 0;
        if let Some(task) = self.diag_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(1), task).await;
        }
        ensure!(
            !self.group_alive(),
            "Owned child process group -{} still alive after SIGKILL",
            self.pgid
        );
        Ok(())
    }
}

impl Drop for SeatProcessGroup {
    fn drop(&mut self) {
        if self.pgid > 0 && self.group_alive() {
            let _ = crate::processes::signal_owned(&self.child, rustix::process::Signal::TERM);
            let _ = crate::processes::signal_owned(&self.child, rustix::process::Signal::KILL);
        }
        if let Some(task) = self.diag_task.take() {
            task.abort();
        }
    }
}

/// The rules or engine errors the seats' models reported in `game_id`
/// (the transcripts' `reports.jsonl`, `docs/llm-protocol.md` §"Reports and
/// lookups"), and per seat what became of its bug report to the gateway:
/// its transcript's `llm_reports` line, `null` where it filed none.
fn llm_reports(transcripts_dir: &Path, game_id: &str) -> (usize, Vec<Value>) {
    let filed = read_jsonl(&transcripts_dir.join(baylee_seat::reports::FILE))
        .unwrap_or_default()
        .iter()
        .filter(|row| row.get("game_id").and_then(Value::as_str) == Some(game_id))
        .count();
    let sent = [0_usize, 1_usize]
        .iter()
        .map(|seat| {
            read_jsonl(&transcripts_dir.join(format!("{game_id}-seat{seat}.jsonl")))
                .unwrap_or_default()
                .into_iter()
                .find_map(|row| row.get("llm_reports").cloned())
                .unwrap_or(Value::Null)
        })
        .collect();
    (filed, sent)
}

fn both_transcripts_over(out_dir: &Path, game_id: &str) -> bool {
    for seat in [0_usize, 1_usize] {
        let p = out_dir
            .join("transcripts")
            .join(format!("{game_id}-seat{seat}.jsonl"));
        let Ok(rows) = read_jsonl(&p) else {
            return false;
        };
        if !rows
            .iter()
            .any(|v| v.get("event").and_then(Value::as_str) == Some("over"))
        {
            return false;
        }
    }
    true
}

/// What a game's two seats were asked to play, as the pinned profiles file
/// says it again at room creation ([`validate_profile_pins`]).
struct Requested {
    /// The pinned pairings file, where the request names one.
    pairings: Option<PathBuf>,
    /// Each seat's chain: the mind its profile declares, then its
    /// fallbacks' in the order they take over.
    chains: [Chain; 2],
}

impl Requested {
    /// The minds each seat's profile declares.
    fn minds(&self) -> [baylee_gamehost::record::Mind; 2] {
        self.chains.clone().map(|chain| chain.mind)
    }

    /// The fallback profiles of each seat, in order.
    fn fallback_profiles(&self) -> [Vec<String>; 2] {
        self.chains.clone().map(|chain| chain.fallback_profiles())
    }

    /// The minds each seat's fallbacks declare, in order.
    fn fallback_minds(&self) -> [Vec<baylee_gamehost::record::Mind>; 2] {
        self.chains.clone().map(|chain| chain.fallback_minds())
    }

    /// Every mind each seat may declare, in the order it may.
    fn declarable(&self) -> [Vec<baylee_gamehost::record::Mind>; 2] {
        self.chains.clone().map(|chain| chain.minds())
    }
}

/// Revalidate scheduler identities before any room can be opened: each
/// seat's profile and the fallbacks it names, as the request froze them
/// (`fallback_profiles`; none where an older request has no such field).
fn validate_profile_pins(
    config: &Path,
    request: &Value,
    profiles: &[String; 2],
    models: &[String; 2],
) -> anyhow::Result<Requested> {
    let expected = request
        .get("profiles_config_sha256")
        .and_then(Value::as_str)
        .context("Missing profiles config pin")?;
    ensure!(
        sha256_file(config)? == expected,
        "Profiles config changed before room creation"
    );
    let paths = baylee_seat::config::Paths {
        settings: Some(config.to_path_buf()),
        named: true,
        ledger: None,
    };
    let settings = paths.load().map_err(anyhow::Error::msg)?;
    let mut chains = Vec::with_capacity(2);
    for (seat, profile) in profiles.iter().enumerate() {
        let chain = requested_chain(settings.as_ref(), &paths, profile)?;
        ensure!(
            chain.model() == models[seat],
            "Requested profile/model mismatch for seat {seat}"
        );
        let frozen = request
            .get("fallback_profiles")
            .and_then(|seats| seats.get(seat))
            .cloned()
            .unwrap_or_else(|| json!([]));
        ensure!(
            frozen == json!(chain.fallback_profiles()),
            "Requested fallback chain mismatch for seat {seat}"
        );
        chains.push(chain);
    }
    ensure!(
        sha256_file(config)? == expected,
        "Profiles config changed while validating"
    );
    let chains: [Chain; 2] = chains
        .try_into()
        .map_err(|_| anyhow!("Expected two declared identities"))?;
    let source = request.get("pairings_file").filter(|v| !v.is_null());
    let pairings = if let Some(source) = source {
        let path = PathBuf::from(source.as_str().context("Invalid pairings path")?);
        let expected = request
            .get("pairings_sha256")
            .and_then(Value::as_str)
            .context("Missing pairings pin")?;
        ensure!(
            sha256_file(&path)? == expected,
            "Pairings changed before room creation"
        );
        let pairs: Vec<[String; 2]> = serde_json::from_slice(&std::fs::read(&path)?)?;
        ensure!(
            pairs.contains(profiles),
            "Requested pair absent from frozen pairings"
        );
        ensure!(
            sha256_file(&path)? == expected,
            "Pairings changed while validating"
        );
        Some(path)
    } else {
        ensure!(
            request.get("pairings_sha256").is_none_or(Value::is_null),
            "Pairings pin has no file"
        );
        None
    };
    Ok(Requested { pairings, chains })
}

fn room_update(
    slot: usize,
    setup: &baylee_core::preset::RoomSetup,
) -> baylee_core::preset::RoomUpdate {
    baylee_core::preset::RoomUpdate {
        name: format!("LLM teacher slot {slot}"),
        chairs: 2,
        password: None,
        setup: setup.clone(),
    }
}

fn verify_configured_setup(
    page: &Value,
    game_id: &str,
    requested: &baylee_core::preset::RoomSetup,
) -> anyhow::Result<()> {
    let room = page
        .get("games")
        .and_then(Value::as_array)
        .and_then(|games| {
            games
                .iter()
                .find(|game| game.get("id").and_then(Value::as_str) == Some(game_id))
        })
        .context("Configured owned room absent from response")?;
    let actual: baylee_core::preset::RoomSetup = serde_json::from_value(
        room.get("setup")
            .context("Configured room response lacks setup")?
            .clone(),
    )?;
    ensure!(
        actual == *requested,
        "Gateway did not apply requested room setup"
    );
    Ok(())
}

fn owned_seats_finished(started: bool, all_exited: bool, both_over: bool) -> anyhow::Result<bool> {
    if all_exited {
        ensure!(
            started && both_over,
            "Seats ended without both GameOver transcripts"
        );
        return Ok(true);
    }
    Ok(false)
}

fn confirm_terminal_closure(
    out: &Path,
    record: &Path,
    game_id: &str,
    start_requested: bool,
) -> anyhow::Result<()> {
    let end = completed_record_end(record)?;
    write_atomic_json(
        &out.join("room-state.json"),
        &json!({
            "status":"closed", "closure_verified":true, "game_id":game_id,
            "start_requested":start_requested,
            "proof":"Complete official exported End for this owned game; transcript and model admission checked separately",
            "terminal":end,
            "record_path":record.canonicalize()?,
            "record_sha256":sha256_file(record)?,
        }),
    )
}

/// Run a single table attempt, writing all evidence into `assignment.attempt_dir`.
///
/// `account` opens the room and holds chair 0: the slot's house account
/// where seat 0 is the house (`house:<level>`), else its teacher. Chair 1 is
/// a model's on `account`'s chair ticket, or, where seat 1 is the house,
/// the chair `house` (the slot's house account) joins: a chair ticket seats
/// only a language model (`LLM-…`), and a chair must be called what plays
/// it, so the house always sits in a `HOUSE-…` account's own chair.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub async fn run_attempt(
    assignment: &Assignment,
    account: &mut SlotAccount,
    mut house: Option<&mut SlotAccount>,
    seat_bin: &Path,
    gateway_bin: &Path,
    db: &DatabaseConnection,
    pacer: &LaunchPacer,
    replay_sem: &Semaphore,
    stop_flag: &AtomicBool,
    stop_file: &Path,
    admin_cleanup: &crate::admin_cleanup::AdminCleanup,
) -> anyhow::Result<Value> {
    let out = &assignment.attempt_dir;
    std::fs::create_dir_all(out)?;
    ensure!(
        !out.join("technical-result.json").exists(),
        "Refuse existing attempt result in {}",
        out.display()
    );
    write_atomic_json(&out.join("request.json"), &assignment.request)?;
    let set_phase = |stage: &str| -> anyhow::Result<()> {
        write_atomic_json(&out.join("phase.json"), &json!({ "stage": stage }))
    };
    let set_room_state = |status: &str,
                          game_id: Option<&str>,
                          start_req: bool,
                          proof: Option<&str>|
     -> anyhow::Result<()> {
        let mut obj = json!({
            "status": status,
            "game_id": game_id,
            "start_requested": start_req,
            "closure_verified": status == "closed",
        });
        if let Some(p) = proof {
            obj["proof"] = json!(p);
        }
        write_atomic_json(&out.join("room-state.json"), &obj)
    };

    set_phase("validating-inputs")?;
    let room_setup =
        collector_room_setup([&assignment.decks[0].format, &assignment.decks[1].format])?;
    let requested_setup: baylee_core::preset::RoomSetup = serde_json::from_value(
        assignment
            .request
            .get("room_setup")
            .context("Missing frozen room setup")?
            .clone(),
    )?;
    ensure!(
        room_setup == requested_setup,
        "Frozen room setup differs from declared deck formats"
    );
    ensure!(
        seat_bin.is_file(),
        "Missing baylee-seat binary: {}",
        seat_bin.display()
    );
    let inv0 = parse_deck_inventory(&assignment.decks[0])?;
    let inv1 = parse_deck_inventory(&assignment.decks[1])?;
    let derived_decks = derive_cli_decks(out, &[inv0.clone(), inv1.clone()])?;

    let profiles_config = PathBuf::from(
        assignment
            .request
            .get("profiles_config")
            .and_then(Value::as_str)
            .context("missing profiles_config")?,
    );
    let ledger = PathBuf::from(
        assignment
            .request
            .get("ledger")
            .and_then(Value::as_str)
            .context("missing ledger")?,
    );
    let requested = validate_profile_pins(
        &profiles_config,
        &assignment.request,
        &assignment.profiles,
        &assignment.models,
    )?;
    let sits = requested.chains.clone().map(|chain| {
        if chain.is_house() {
            Sits::House
        } else {
            Sits::Model
        }
    });
    ensure!(
        account.sits == sits[0],
        "Chair 0's account is not named for what plays it"
    );
    ensure!(
        house.as_ref().map(|house| house.sits) == (sits[1] == Sits::House).then_some(Sits::House),
        "Chair 1 takes the slot's house account where the house plays it, and only there"
    );
    let mut frozen_paths = vec![
        seat_bin.to_path_buf(),
        profiles_config.clone(),
        PathBuf::from(&assignment.decks[0].path),
        PathBuf::from(&assignment.decks[1].path),
        derived_decks[0].clone(),
        derived_decks[1].clone(),
    ];
    if let Some(path) = &requested.pairings {
        frozen_paths.push(path.clone());
    }
    if gateway_bin.is_file() {
        frozen_paths.push(gateway_bin.to_path_buf());
    }
    let mut input_pins = serde_json::Map::new();
    for p in &frozen_paths {
        input_pins.insert(p.to_string_lossy().into_owned(), json!(sha256_file(p)?));
    }
    write_atomic_json(
        &out.join("input-receipt.json"),
        &json!({
            "sha256": input_pins,
            "requested_models": assignment.models,
            "requested_minds": requested.minds(),
            "requested_fallback_profiles": requested.fallback_profiles(),
            "requested_fallbacks": requested.fallback_minds(),
            "requested_room_setup": room_setup,
            "policy_targets": "none_until_review",
            "ppo_eligible": false,
        }),
    )?;

    let info = account.client.get_json("/info", None, 15).await?;
    ensure!(
        info.get("protocol_version").and_then(Value::as_u64) == Some(36)
            && info.get("view_version").and_then(Value::as_u64) == Some(82),
        "Wrong gateway ABI: {info}"
    );

    account.ensure_presence().await?;
    set_phase("uploading-host-deck")?;
    let host_deck_id = account
        .ensure_deck_uploaded(&assignment.decks[0].key, &inv0)
        .await?;
    // The house brings its own deck to the chair its account joins.
    let house_deck_id = match house.as_deref_mut() {
        Some(house) => {
            house.ensure_presence().await?;
            Some(
                house
                    .ensure_deck_uploaded(&assignment.decks[1].key, &inv1)
                    .await?,
            )
        }
        None => None,
    };

    // Pace room creation + chair 1 ticket redemption across all 50 slots.
    pacer.wait_turn().await;
    validate_profile_pins(
        &profiles_config,
        &assignment.request,
        &assignment.profiles,
        &assignment.models,
    )?;
    for (path, expected) in &input_pins {
        ensure!(
            expected.as_str() == Some(sha256_file(Path::new(path))?.as_str()),
            "Transport input changed before room creation"
        );
    }

    set_phase("opening-room")?;
    account.room_closed = false;
    set_room_state("opening", None, false, None)?;
    let grant = account
        .client
        .post_json(
            "/lobby/games",
            &json!({
                "deck_id": host_deck_id,
                "mode": "open",
                "seats": 2,
                "name": format!("LLM teacher slot {}", assignment.slot_id),
                "decision_timeout_secs": 0,
            }),
            Some(&account.token),
            20,
        )
        .await?;
    let game_id = grant
        .get("game_id")
        .and_then(Value::as_str)
        .context("missing game_id from POST /lobby/games")?
        .to_owned();
    let mut children: Vec<SeatProcessGroup> = Vec::with_capacity(2);
    let mut start_requested = false;
    let run_future = async {
        let seat0_token = grant
            .get("seat_token")
            .and_then(Value::as_str)
            .context("missing seat_token from POST /lobby/games")?
            .to_owned();
        set_room_state("waiting", Some(&game_id), false, None)?;
        set_phase("configuring-room")?;
        // Room creation does not read setup; use the host-only configuration
        // endpoint before any seat process can mark itself ready.
        let configured = account
            .client
            .post_json(
                &format!("/lobby/games/{game_id}/configure"),
                &serde_json::to_value(room_update(assignment.slot_id, &room_setup))?,
                Some(&account.token),
                20,
            )
            .await?;
        verify_configured_setup(&configured, &game_id, &room_setup)?;
        write_atomic_json(
            &out.join("room-setup-receipt.json"),
            &json!({
                "game_id":game_id, "requested_setup":room_setup,
                "confirmed_setup":room_setup, "method":"authenticated_host_configure_then_listing_confirmation",
            }),
        )?;
        set_phase("seating-models")?;

        // Chair 1's first stdin line: the house account's own chair, which
        // it joins, or a model's chair ticket from the host.
        let secret1 = if let (Some(house), Some(deck_id)) = (house.as_deref_mut(), &house_deck_id) {
            house.room_closed = false;
            let joined = house
                .client
                .post_json(
                    &format!("/lobby/games/{game_id}/join"),
                    &json!({ "deck_id": deck_id, "seat": 1 }),
                    Some(&house.token),
                    20,
                )
                .await?;
            ensure!(
                joined.get("seat").and_then(Value::as_u64) == Some(1),
                "The house account was not seated in chair 1"
            );
            let seat_token = joined
                .get("seat_token")
                .and_then(Value::as_str)
                .context("missing seat_token from POST /join")?;
            serde_json::to_string(&json!({
                "game_id": game_id,
                "seat": 1,
                "seat_token": seat_token,
                "session_token": house.token,
            }))?
        } else {
            let ticket_resp = account
                .client
                .post_json(
                    &format!("/lobby/games/{game_id}/chairs/1/ticket"),
                    &json!({}),
                    Some(&account.token),
                    20,
                )
                .await?;
            ticket_resp
                .get("ticket")
                .and_then(Value::as_str)
                .context("missing ticket from POST /chairs/1/ticket")?
                .to_owned()
        };

        let transcripts_dir = out.join("transcripts");
        std::fs::create_dir_all(&transcripts_dir)?;
        let think_secs = assignment
            .request
            .get("call_timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(90);
        let game_timeout_secs = assignment
            .request
            .get("game_timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(3600);

        let secret0 = serde_json::to_string(&json!({
            "game_id": game_id,
            "seat": 0,
            "seat_token": seat0_token,
            "session_token": account.token,
        }))?;

        let fallbacks = requested.fallback_profiles();
        let launch = |seat: usize| SeatLaunch {
            seat_bin,
            game_id: &game_id,
            seat,
            gateway: account.client.base(),
            profile: &assignment.profiles[seat],
            fallbacks: &fallbacks[seat],
            profiles_config: &profiles_config,
            ledger: &ledger,
            deck_path: &derived_decks[seat],
            think_secs,
            transcripts_dir: &transcripts_dir,
            work_dir: out,
            send_reports: assignment.send_reports,
        };
        let child0 = SeatProcessGroup::spawn(&launch(0))?;
        children.push(child0);
        children[0].deliver_secret(&secret0).await?;

        let child1 = SeatProcessGroup::spawn(&launch(1))?;
        children.push(child1);
        children[1].deliver_secret(&secret1).await?;

        let mut started = false;
        let deadline = Instant::now() + Duration::from_secs(game_timeout_secs);
        loop {
            ensure!(
                !stop_flag.load(Ordering::SeqCst) && !stop_file.exists(),
                "Collection stopped"
            );
            ensure!(Instant::now() < deadline, "Game deadline exceeded");
            account.presence.check()?;
            for (s, ch) in children.iter_mut().enumerate() {
                if let Some(status) = ch.try_wait()?
                    && !status.success()
                {
                    bail!("Seat {s} transport exited with {status}");
                }
            }
            let page = account
                .client
                .get_json(
                    &format!("/lobby/games?q={game_id}&limit=10"),
                    Some(&account.token),
                    15,
                )
                .await?;
            let room = page
                .get("games")
                .and_then(Value::as_array)
                .and_then(|arr| {
                    arr.iter()
                        .find(|v| v.get("id").and_then(Value::as_str) == Some(game_id.as_str()))
                })
                .cloned();
            let all_exited = children
                .iter_mut()
                .map(SeatProcessGroup::try_wait)
                .collect::<anyhow::Result<Vec<_>>>()?
                .iter()
                .all(Option::is_some);
            if owned_seats_finished(started, all_exited, both_transcripts_over(out, &game_id))? {
                break;
            }
            let Some(room) = room else {
                ensure!(started, "Owned waiting room disappeared before start");
                if both_transcripts_over(out, &game_id) {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            if !started && room.get("startable").and_then(Value::as_bool) == Some(true) {
                set_phase("starting-game")?;
                start_requested = true;
                account
                    .client
                    .post_json(
                        &format!("/lobby/games/{game_id}/start"),
                        &json!({}),
                        Some(&account.token),
                        20,
                    )
                    .await?;
                started = true;
                set_room_state("playing", Some(&game_id), true, None)?;
                set_phase("playing")?;
            }
            if started && room.get("state").and_then(Value::as_str) == Some("over") {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Wait up to 10s for both seats to flush their final `over` line to transcripts.
        for _ in 0..20 {
            if both_transcripts_over(out, &game_id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        ensure!(
            both_transcripts_over(out, &game_id),
            "Both seat transcripts did not record GameOver"
        );

        set_phase("exporting-completed-game")?;
        let after_dir = out.join("official-export");
        std::fs::create_dir_all(&after_dir)?;
        let record_path = after_dir.join("000001.jsonl.gz");
        let mut exported = false;
        for attempt in 0..15 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            if export_game_record(db, &game_id, &record_path).await.is_ok() {
                exported = true;
                break;
            }
        }
        ensure!(
            exported && record_path.is_file(),
            "No newly completed official record for {game_id} in PostgreSQL"
        );

        // An admission rejection cannot reopen a completed game. Establish the
        // lifecycle independently before checking declared identity/provenance.
        confirm_terminal_closure(out, &record_path, &game_id, start_requested)?;
        account.room_closed = true;
        let proof = audit_record(&record_path, &requested.declarable())?;
        let actual_setup = audit_record_setup(&record_path, &room_setup)?;

        let mut seats = Vec::with_capacity(2);
        let mut artifacts = Vec::new();
        let mut quota = false;
        let pin = |p: &Path, role: &str| -> anyhow::Result<Value> {
            let canon = p.canonicalize()?;
            Ok(json!({
                "path": canon.to_string_lossy(),
                "sha256": sha256_file(&canon)?,
                "role": role,
            }))
        };
        artifacts.push(pin(&record_path, "anonymized_record")?);
        artifacts.push(pin(
            &out.join("room-setup-receipt.json"),
            "room_setup_receipt",
        )?);
        for (s, p) in derived_decks.iter().enumerate() {
            artifacts.push(pin(p, &format!("derived_deck_seat{s}"))?);
        }
        for seat in [0_usize, 1_usize] {
            let tr_path = transcripts_dir.join(format!("{game_id}-seat{seat}.jsonl"));
            // A house seat's mind is the house: none of its answers is a model's.
            let house_seat = requested.chains[seat].is_house();
            let (counts, limited) = audit_transcript(&tr_path, &proof.inputs[seat], house_seat)?;
            quota |= limited;
            let mut src_counts = serde_json::Map::new();
            for v in &proof.inputs[seat] {
                let by = v.get("by").and_then(Value::as_str).unwrap_or("unknown");
                let prev = src_counts.get(by).and_then(Value::as_u64).unwrap_or(0);
                src_counts.insert(by.to_owned(), json!(prev + 1));
            }
            seats.push(json!({
                "seat": seat,
                "model": assignment.models[seat],
                "declared_mind_verified": true,
                "declared_minds": proof.declared[seat],
                "profile_fallbacks": transcript_fallbacks(&read_jsonl(&tr_path)?),
                "transcript_answer_counts": counts,
                "executed_input_sources": src_counts,
            }));
            artifacts.push(pin(&tr_path, &format!("transcript_seat{seat}"))?);
            let prov_path = out.join(format!("input-provenance-seat{seat}.json"));
            let attr = ordered_attribution(&tr_path, &proof.inputs[seat], house_seat)?;
            write_atomic_json(&prov_path, &attr)?;
            artifacts.push(pin(&prov_path, &format!("executed_provenance_seat{seat}"))?);
        }

        set_phase("verifying-public-replay")?;
        let tape_path = out.join("public-replay.jsonl.gz");
        {
            let _permit = replay_sem.acquire().await?;
            let mut tape_bytes = None;
            for attempt in 0..8 {
                if attempt > 0 {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                match account
                    .client
                    .post_bytes(
                        &format!("/replays/{game_id}/tape"),
                        &json!({ "perspective": "public", "verify": "stop" }),
                        Some(&account.token),
                        300,
                    )
                    .await
                {
                    Ok(bytes) => {
                        tape_bytes = Some(bytes);
                        break;
                    }
                    Err(e) if attempt + 1 == 8 => return Err(e),
                    Err(_) => {}
                }
            }
            std::fs::write(&tape_path, tape_bytes.context("missing replay tape bytes")?)?;
        }
        let stop = audit_tape(&tape_path, &proof)?;
        let rep_receipt = out.join("replay-receipt.json");
        let rec_sha = sha256_file(&record_path)?;
        write_atomic_json(
            &rep_receipt,
            &json!({
                "status": "verified",
                "method": "official_gateway_public_tape_verify_stop",
                "stop": stop,
                "tape_sha256": sha256_file(&tape_path)?,
                "record_sha256": rec_sha,
                "gateway_info": info,
            }),
        )?;
        artifacts.push(pin(&tape_path, "replay_tape")?);
        artifacts.push(pin(&rep_receipt, "replay_receipt")?);
        artifacts.push(pin(&out.join("input-receipt.json"), "input_receipt")?);

        for (p_str, exp) in &input_pins {
            let cur = sha256_file(Path::new(p_str))?;
            ensure!(
                exp.as_str() == Some(cur.as_str()),
                "Transport input changed during game: {p_str}"
            );
        }

        let (reported, report_submissions) = llm_reports(&transcripts_dir, &game_id);
        let result = json!({
            "schema_version": 1,
            "status": "complete",
            "game_id": game_id,
            "actual_seed": proof.actual_seed,
            "requested_room_setup": room_setup,
            "actual_room_setup": actual_setup,
            "terminal": proof.terminal,
            "seats": seats,
            "checks": {
                "connection": true,
                "view_boundary": true,
                "full_record": true,
                "replay": true,
                "provenance": true,
            },
            "artifacts": artifacts,
            "record_sha256": rec_sha,
            "bc_only": true,
            "ppo_eligible": false,
            "quota_limited": quota,
            "policy_targets": "none_until_ordered_provenance_review",
            "provenance_scope": "Transcript counts are sends, not accepted-input labels; ordered companion preserves all compatible origins and ambiguity, fallback is never implicitly LLM. No targets or semantic strength filter.",
            "transport_binary_sha256": sha256_file(seat_bin)?,
            "llm_reports": reported,
            "llm_report_submissions": report_submissions,
        });
        Ok(result)
    };
    let run_res: anyhow::Result<Value> = tokio::select! {
        result = run_future => result,
        () = async {
            loop {
                if stop_flag.load(Ordering::SeqCst) || stop_file.exists() { break; }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        } => Err(anyhow!("Collection stopped")),
    };

    let mut cleanup_errors = Vec::new();
    for ch in children.iter_mut().rev() {
        if let Err(error) = ch.terminate().await {
            cleanup_errors.push(format!("{error:#}"));
        }
    }
    let mut waiting_cancelled = false;
    if run_res.is_err() && !account.room_closed {
        // A transcript or provenance failure does not imply the game is alive.
        // Query the official completed record for this exact owned game, rather
        // than trusting a missing lobby row or any earlier local export.
        let closure_record = out.join("closure-export/000001.jsonl.gz");
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            let proof = tokio::time::timeout(Duration::from_secs(5), async {
                export_game_record(db, &game_id, &closure_record).await?;
                confirm_terminal_closure(out, &closure_record, &game_id, start_requested)
            })
            .await;
            if matches!(proof, Ok(Ok(()))) {
                account.room_closed = true;
                break;
            }
        }
    }
    if run_res.is_err() && !account.room_closed && start_requested && cleanup_errors.is_empty() {
        // Administrative lifecycle closure is deliberately separate from an
        // official rules End. Never admit this aborted attempt or invent a draw.
        match admin_cleanup
            .close_owned_started(&account.client, &account.token, &game_id, out)
            .await
        {
            Ok(()) => account.room_closed = true,
            Err(error) => cleanup_errors.push(format!("Owned game abort unresolved: {error:#}")),
        }
    }
    if run_res.is_ok() {
        account.room_closed = true;
    } else if !account.room_closed && !start_requested && cleanup_errors.is_empty() {
        // The house's account stands up first: a room its host leaves
        // passes to whoever still sits there rather than closing.
        if let Some(house) = house.as_deref() {
            let _ = house
                .client
                .post_json(
                    &format!("/lobby/games/{game_id}/leave"),
                    &json!({}),
                    Some(&house.token),
                    5,
                )
                .await;
        }
        account.room_closed = close_unstarted_room(&account.client, &account.token, &game_id).await;
        waiting_cancelled = account.room_closed;
    }
    // The house's chair goes with the room it sits in.
    if let Some(house) = house {
        house.room_closed = account.room_closed;
    }
    if !cleanup_errors.is_empty() || !account.room_closed {
        if !account.room_closed {
            set_room_state("unknown", Some(&game_id), start_requested, None)?;
        }
        let root = out
            .parent()
            .and_then(Path::parent)
            .context("attempt root missing")?;
        write_atomic_json(
            &root.join("transport-blocked.json"),
            &json!({
                "attempt_id": assignment.attempt_id, "game_id": game_id,
                "reason": "Owned room or child cleanup unresolved",
                "run_error": run_res.as_ref().err().map(|error| format!("{error:#}")),
                "child_cleanup_errors": cleanup_errors,
                "room_closed": account.room_closed,
            }),
        )?;
        if let Err(error) = run_res {
            return Err(error.context("Transport blocked: owned room or child cleanup unresolved"));
        }
        bail!("Transport blocked: owned room or child cleanup unresolved");
    }
    if waiting_cancelled {
        set_room_state(
            "closed",
            Some(&game_id),
            start_requested,
            Some(
                "Unstarted room and all owned children closed; complete authenticated exact-ID lobby query confirmed absence",
            ),
        )?;
    }
    let result = run_res?;
    validate_technical_result(&result, &assignment.request, out)?;
    write_atomic_json(&out.join("technical-result.json"), &result)?;
    set_phase("complete")?;
    Ok(result)
}

#[cfg(test)]
mod tests;
