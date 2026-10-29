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
    export_game_record, ordered_attribution, read_jsonl, sha256_file, validate_technical_result,
    write_atomic_json,
};
use crate::schedule::{
    Assignment, DeckInventory, collector_room_setup, derive_cli_decks, parse_deck_inventory,
};
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

/// Dedicated per-slot host account and cached uploaded deck IDs.
pub struct SlotAccount {
    pub client: GatewayClient,
    pub token: String,
    pub presence: Presence,
    pub uploaded_decks: BTreeMap<String, String>,
    password: String,
    room_closed: bool,
}

fn account_username(id: uuid::Uuid) -> String {
    let suffix = id.simple().to_string();
    format!("llmt{}", &suffix[12..])
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

async fn delete_owned_account(
    client: &GatewayClient,
    token: &str,
    password: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/account", client.base());
    let token = token.to_owned();
    let body = serde_json::to_vec(&json!({"password": password}))?;
    tokio::task::spawn_blocking(move || {
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
    pub async fn init(root: &Path, gateway: &str, slot_id: usize) -> anyhow::Result<Self> {
        let _ = root; // Credentials are intentionally process-local.
        let client = GatewayClient::new(gateway, slot_id);
        let username = account_username(uuid::Uuid::now_v7());
        let password = format!(
            "{}{}",
            uuid::Uuid::now_v7().simple(),
            uuid::Uuid::now_v7().simple()
        );
        client
            .post_json(
                "/auth/register",
                &json!({
                    "username": username, "display_name": format!("LLM-teacher-{slot_id}"),
                    "password": password, "lang": "en",
                }),
                None,
                20,
            )
            .await?;
        let val = client
            .post_json(
                "/auth/login",
                &json!({
                    "username": username, "password": password,
                }),
                None,
                20,
            )
            .await?;
        let token = val
            .get("token")
            .and_then(Value::as_str)
            .context("missing login token")?
            .to_owned();

        let presence = match Presence::connect(&client, &token).await {
            Ok(presence) => presence,
            Err(error) => {
                delete_owned_account(&client, &token, &password).await
                    .context("Lobby presence initialization failed and account cleanup is unresolved; retry refused")?;
                return Err(anyhow::Error::new(CleanAccountInitFailure(error)));
            }
        };
        Ok(Self {
            client,
            token,
            presence,
            uploaded_decks: BTreeMap::new(),
            password,
            room_closed: true,
        })
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.presence.close().await;
        ensure!(
            self.room_closed,
            "Account retained because owned room closure is unresolved"
        );
        delete_owned_account(&self.client, &self.token, &self.password).await
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

impl SeatProcessGroup {
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn spawn(
        seat_bin: &Path,
        game_id: &str,
        seat: usize,
        gateway: &str,
        profile: &str,
        profiles_config: &Path,
        ledger: &Path,
        deck_path: &Path,
        think_secs: u64,
        transcripts_dir: &Path,
        work_dir: &Path,
    ) -> anyhow::Result<Self> {
        let mut cmd = Command::new(seat_bin);
        cmd.arg("join")
            .arg(game_id)
            .arg("--gateway")
            .arg(gateway)
            .arg("--chair")
            .arg(seat.to_string())
            .arg("--tethered")
            .arg("--profile")
            .arg(profile)
            .arg("--config")
            .arg(profiles_config)
            .arg("--ledger")
            .arg(ledger)
            .arg("--deck")
            .arg(deck_path)
            .arg("--think-secs")
            .arg(think_secs.to_string())
            .arg("--level")
            .arg("sharp")
            .arg("--transcripts")
            .arg(transcripts_dir)
            .arg("--name")
            .arg(format!("teacher-{seat}"))
            .arg(if seat == 0 {
                "--host-chair"
            } else {
                "--chair-ticket"
            });
        cmd.current_dir(work_dir)
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
            if let Ok(val) = std::env::var(key) {
                cmd.env(key, val);
            }
        }
        cmd.env("OMP_NUM_THREADS", "1")
            .env("MKL_NUM_THREADS", "1")
            .env("OPENBLAS_NUM_THREADS", "1")
            .env("CUDA_VISIBLE_DEVICES", "")
            .env("BAYLEE_EP", "cpu")
            .env("RUST_BACKTRACE", "0");

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
        let diag_path = work_dir.join(format!("seat{seat}-diagnostics.json"));
        let diag_task = guard.child.stderr.take().map(|stderr| {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                let mut line_count = 0_u64;
                let mut rate_limit_mentions = 0_u64;
                let mut timeout_mentions = 0_u64;
                while let Ok(Some(line)) = reader.next_line().await {
                    line_count += 1;
                    let lower = line.to_ascii_lowercase();
                    if lower.contains("rate limit") || lower.contains("429") {
                        rate_limit_mentions += 1;
                    }
                    if lower.contains("timed out") || lower.contains("timeout") {
                        timeout_mentions += 1;
                    }
                }
                let _ = write_atomic_json(
                    &diag_path,
                    &json!({
                        "stderr_lines": line_count,
                        "rate_limit_mentions": rate_limit_mentions,
                        "timeout_mentions": timeout_mentions,
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

/// Revalidate scheduler identities before any room can be opened.
fn validate_profile_pins(
    config: &Path,
    request: &Value,
    profiles: &[String; 2],
    models: &[String; 2],
) -> anyhow::Result<(Option<PathBuf>, [baylee_gamehost::record::Mind; 2])> {
    let mut expected_minds = Vec::with_capacity(2);
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
    for (seat, profile) in profiles.iter().enumerate() {
        let plan = baylee_seat::config::plan(
            None,
            settings.as_ref(),
            &paths,
            Some(profile),
            &baylee_seat::config::Overrides::default(),
        )
        .map_err(anyhow::Error::msg)?
        .context("Requested profile has no model")?;
        let declared = baylee_seat::declare::llm(&plan.settings);
        ensure!(
            declared.model == models[seat],
            "Requested profile/model mismatch for seat {seat}"
        );
        ensure!(
            baylee_protocol::mind::fault(&declared).is_none(),
            "Invalid requested declared mind"
        );
        expected_minds
            .push(baylee_gamehost::record::Mind::from_wire(&declared).map_err(anyhow::Error::msg)?);
    }
    ensure!(
        sha256_file(config)? == expected,
        "Profiles config changed while validating"
    );
    let expected_minds: [baylee_gamehost::record::Mind; 2] = expected_minds
        .try_into()
        .map_err(|_| anyhow!("Expected two declared identities"))?;
    let source = request.get("pairings_file").filter(|v| !v.is_null());
    if let Some(source) = source {
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
        Ok((Some(path), expected_minds))
    } else {
        ensure!(
            request.get("pairings_sha256").is_none_or(Value::is_null),
            "Pairings pin has no file"
        );
        Ok((None, expected_minds))
    }
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

/// Run a single table attempt on `account`, writing all evidence into `assignment.attempt_dir`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub async fn run_attempt(
    assignment: &Assignment,
    account: &mut SlotAccount,
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
    let (pairings_path, expected_minds) = validate_profile_pins(
        &profiles_config,
        &assignment.request,
        &assignment.profiles,
        &assignment.models,
    )?;
    let mut frozen_paths = vec![
        seat_bin.to_path_buf(),
        profiles_config.clone(),
        PathBuf::from(&assignment.decks[0].path),
        PathBuf::from(&assignment.decks[1].path),
        derived_decks[0].clone(),
        derived_decks[1].clone(),
    ];
    if let Some(path) = pairings_path {
        frozen_paths.push(path);
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
            "requested_minds": expected_minds,
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

        let ticket_resp = account
            .client
            .post_json(
                &format!("/lobby/games/{game_id}/chairs/1/ticket"),
                &json!({}),
                Some(&account.token),
                20,
            )
            .await?;
        let chair1_ticket = ticket_resp
            .get("ticket")
            .and_then(Value::as_str)
            .context("missing ticket from POST /chairs/1/ticket")?
            .to_owned();

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

        let child0 = SeatProcessGroup::spawn(
            seat_bin,
            &game_id,
            0,
            account.client.base(),
            &assignment.profiles[0],
            &profiles_config,
            &ledger,
            &derived_decks[0],
            think_secs,
            &transcripts_dir,
            out,
        )?;
        children.push(child0);
        children[0].deliver_secret(&secret0).await?;

        let child1 = SeatProcessGroup::spawn(
            seat_bin,
            &game_id,
            1,
            account.client.base(),
            &assignment.profiles[1],
            &profiles_config,
            &ledger,
            &derived_decks[1],
            think_secs,
            &transcripts_dir,
            out,
        )?;
        children.push(child1);
        children[1].deliver_secret(&chair1_ticket).await?;

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
        let proof = audit_record(&record_path, &expected_minds)?;
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
            let (counts, limited) = audit_transcript(&tr_path, &proof.inputs[seat])?;
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
                "transcript_answer_counts": counts,
                "executed_input_sources": src_counts,
            }));
            artifacts.push(pin(&tr_path, &format!("transcript_seat{seat}"))?);
            let prov_path = out.join(format!("input-provenance-seat{seat}.json"));
            let attr = ordered_attribution(&tr_path, &proof.inputs[seat])?;
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
        account.room_closed = close_unstarted_room(&account.client, &account.token, &game_id).await;
        waiting_cancelled = account.room_closed;
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
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[test]
    fn credential_destination_requires_literal_loopback_without_url_secrets() {
        for base in ["http://127.0.0.1:28768", "http://[::1]:28768"] {
            gateway_endpoint(base).unwrap();
        }
        for base in [
            "http://example.com:80",
            "https://127.0.0.1:80",
            "http://127.0.0.1:0",
            "http://127.0.0.1:80/path",
            "http://user:password@127.0.0.1:80",
        ] {
            assert!(gateway_endpoint(base).is_err());
        }
    }

    #[tokio::test]
    async fn authenticated_gateway_requests_never_follow_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let target = format!("http://{}/unexpected", destination.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(socket);
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).await.unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            reader.get_mut().write_all(format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        });
        assert!(
            GatewayClient::new(&base, 0)
                .get_json("/test", Some("fixture-token"), 1)
                .await
                .is_err()
        );
        server.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(50), destination.accept())
                .await
                .is_err()
        );
    }

    #[test]
    fn waiting_room_absence_requires_complete_empty_first_page() {
        let valid = json!({"offset":0,"limit":100,"total":0,"games":[]});
        assert!(complete_empty_lobby_page(&valid));
        for key in ["offset", "limit", "total", "games"] {
            let mut incomplete = valid.clone();
            incomplete.as_object_mut().unwrap().remove(key);
            assert!(!complete_empty_lobby_page(&incomplete));
        }
        for (key, value) in [
            ("offset", json!(100)),
            ("limit", json!(25)),
            ("total", json!(1)),
            ("total", json!("0")),
            ("games", json!([{"id":"owned-game"}])),
        ] {
            let mut page = valid.clone();
            page[key] = value;
            assert!(!complete_empty_lobby_page(&page));
        }
    }

    #[tokio::test]
    async fn already_left_room_checks_listing_after_leave_refusal() {
        use tokio::io::AsyncReadExt as _;
        for (leave_status, listing_status, expected) in
            [(404, 200, true), (409, 200, true), (404, 403, false)]
        {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                for (status, expected_request) in [
                    (leave_status, "POST /lobby/games/owned-game/leave HTTP/1.1"),
                    (
                        listing_status,
                        "GET /lobby/games?q=owned-game&offset=0&limit=100&waiting_only=false HTTP/1.1",
                    ),
                ] {
                    let (tcp, _) = listener.accept().await.unwrap();
                    let mut reader = BufReader::new(tcp);
                    let mut first = String::new();
                    reader.read_line(&mut first).await.unwrap();
                    assert_eq!(first.trim_end(), expected_request);
                    let mut content_length = 0;
                    loop {
                        let mut line = String::new();
                        reader.read_line(&mut line).await.unwrap();
                        if line == "\r\n" {
                            break;
                        }
                        if let Some(value) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            content_length = value.trim().parse::<usize>().unwrap();
                        }
                    }
                    reader
                        .read_exact(&mut vec![0; content_length])
                        .await
                        .unwrap();
                    let body = r#"{"offset":0,"limit":100,"total":0,"games":[]}"#;
                    let response = format!(
                        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    reader
                        .get_mut()
                        .write_all(response.as_bytes())
                        .await
                        .unwrap();
                }
            });
            let client = GatewayClient::new(&format!("http://{addr}"), 0);
            assert_eq!(
                close_unstarted_room(&client, "test-session", "owned-game").await,
                expected
            );
            server.await.unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "requires explicit private gateway; creates one waiting room, no model or engine"]
    async fn live_setup_only_configures_twenty_life_and_deletes_owned_room_and_account()
    -> anyhow::Result<()> {
        let gateway = std::env::var("BAYLEE_COLLECTOR_SETUP_TEST_GATEWAY")
            .context("Set BAYLEE_COLLECTOR_SETUP_TEST_GATEWAY to an authorized private gateway")?;
        let authority = gateway
            .strip_prefix("http://")
            .context("Setup test requires loopback HTTP")?;
        crate::private_bind(authority)?;
        let dir = crate::audit::tests::TestDir::new("live-setup-only");
        let mut account = SlotAccount::init(dir.path(), &gateway, 0).await?;
        let mut owned_room = None;
        let result: anyhow::Result<()> = async {
            let grant = account
                .client
                .post_json(
                    "/lobby/games",
                    &json!({
                        "deck_id":"", "mode":"open", "seats":2,
                        "name":"Collector setup-only verification", "allow_spectators":false,
                    }),
                    Some(&account.token),
                    10,
                )
                .await?;
            let game = grant
                .get("game_id")
                .and_then(Value::as_str)
                .context("Missing owned setup-test room id")?
                .to_owned();
            owned_room = Some(game.clone());
            account.room_closed = false;
            let setup = collector_room_setup(["Constructed", "Constructed"])?;
            let listing = account
                .client
                .post_json(
                    &format!("/lobby/games/{game}/configure"),
                    &serde_json::to_value(room_update(0, &setup))?,
                    Some(&account.token),
                    10,
                )
                .await?;
            verify_configured_setup(&listing, &game, &setup)?;
            Ok(())
        }
        .await;
        let cleanup: anyhow::Result<()> = async {
            if let Some(game) = owned_room {
                account
                    .client
                    .post_json(
                        &format!("/lobby/games/{game}/leave"),
                        &json!({}),
                        Some(&account.token),
                        10,
                    )
                    .await?;
                let listing = account
                    .client
                    .get_json(
                        &format!("/lobby/games?q={game}&limit=100&waiting_only=false"),
                        Some(&account.token),
                        10,
                    )
                    .await?;
                ensure!(
                    listing
                        .get("games")
                        .and_then(Value::as_array)
                        .is_some_and(Vec::is_empty),
                    "Owned setup-test room closure unconfirmed"
                );
                account.room_closed = true;
            }
            account.shutdown().await
        }
        .await;
        if cleanup.is_err() {
            account.presence.close().await;
        }
        cleanup.context("Setup-only test cleanup failed")?;
        result
    }

    #[test]
    fn configure_request_roundtrips_and_refuses_gateway_default_setup() {
        let setup = collector_room_setup(["Constructed", "Constructed"]).unwrap();
        let request = serde_json::to_value(room_update(4, &setup)).unwrap();
        let body: baylee_core::preset::RoomUpdate =
            serde_json::from_value(request.clone()).unwrap();
        assert_eq!(body.chairs, 2);
        assert_eq!(
            (body.setup.starting_life, body.setup.free_mulligans),
            (20, 0)
        );
        let confirmed = json!({"games":[{"id":"game", "setup":request["setup"]}]});
        assert!(verify_configured_setup(&confirmed, "game", &setup).is_ok());
        let ignored =
            json!({"games":[{"id":"game", "setup":baylee_core::preset::RoomSetup::default()}]});
        assert!(verify_configured_setup(&ignored, "game", &setup).is_err());
        assert!(verify_configured_setup(&json!({"games":[]}), "game", &setup).is_err());
    }

    #[test]
    fn exited_seats_without_gameover_fail_before_absent_room_can_continue() {
        assert!(!owned_seats_finished(true, false, false).unwrap());
        assert!(owned_seats_finished(true, true, true).unwrap());
        assert!(owned_seats_finished(true, true, false).is_err());
        assert!(owned_seats_finished(false, true, true).is_err());
    }

    #[test]
    fn official_end_proves_closure_without_gameover_but_not_admission() {
        let dir = crate::audit::tests::TestDir::new("end-without-over");
        let out = dir.path();
        let record = out.join("official.jsonl");
        let expected = crate::audit::tests::codex_identity();
        crate::audit::tests::write_identity_record(&record, &expected);
        assert!(!both_transcripts_over(out, "game"));
        confirm_terminal_closure(out, &record, "game", true).unwrap();
        let room: Value =
            serde_json::from_slice(&std::fs::read(out.join("room-state.json")).unwrap()).unwrap();
        assert_eq!(room["closure_verified"], true);
        let proof = audit_record(&record, &[expected.clone(), expected]).unwrap();
        assert!(audit_transcript(&out.join("missing-seat.jsonl"), &proof.inputs[0]).is_err());
        assert!(!out.join("technical-result.json").exists());
        assert!(!out.join("receipt.json").exists());
    }

    #[test]
    fn absent_terminal_end_cannot_mark_unknown_room_closed() {
        let dir = crate::audit::tests::TestDir::new("missing-end");
        let out = dir.path();
        let record = out.join("incomplete.jsonl");
        write_atomic_json(
            &out.join("room-state.json"),
            &json!({"status":"unknown", "closure_verified":false}),
        )
        .unwrap();
        std::fs::write(&record, "{\"kind\":\"header\"}\n").unwrap();
        assert!(confirm_terminal_closure(out, &record, "game", true).is_err());
        let room: Value =
            serde_json::from_slice(&std::fs::read(out.join("room-state.json")).unwrap()).unwrap();
        assert_eq!(room["status"], "unknown");
        assert_eq!(room["closure_verified"], false);
    }

    #[test]
    fn completed_invalid_identity_proves_closure_but_never_creates_admission() {
        let dir = crate::audit::tests::TestDir::new("closed-not-admitted");
        let out = dir.path();
        let record = out.join("record.jsonl");
        let expected = crate::audit::tests::codex_identity();
        let mut spoof = expected.clone();
        spoof.provider = "agy".into();
        crate::audit::tests::write_identity_record(&record, &spoof);
        std::fs::create_dir(out.join("transcripts")).unwrap();
        for seat in 0..2 {
            std::fs::write(
                out.join(format!("transcripts/game-seat{seat}.jsonl")),
                "{\"event\":\"over\"}\n",
            )
            .unwrap();
        }
        confirm_terminal_closure(out, &record, "game", true).unwrap();
        let room: Value =
            serde_json::from_slice(&std::fs::read(out.join("room-state.json")).unwrap()).unwrap();
        assert_eq!(room["status"], "closed");
        assert_eq!(room["closure_verified"], true);
        assert!(audit_record(&record, &[expected.clone(), expected]).is_err());
        assert!(!out.join("technical-result.json").exists());
        assert!(!out.join("receipt.json").exists());
        // A correctly declared completed game still fails admission if its
        // accepted input has no matching ordered transcript send.
        let expected = crate::audit::tests::codex_identity();
        crate::audit::tests::write_identity_record(&record, &expected);
        let proof = audit_record(&record, &[expected.clone(), expected]).unwrap();
        assert!(
            ordered_attribution(&out.join("transcripts/game-seat0.jsonl"), &proof.inputs[0])
                .is_err()
        );
        assert!(!out.join("technical-result.json").exists());
        std::fs::write(&record, "{\"kind\":\"header\"}\n").unwrap();
        assert!(confirm_terminal_closure(out, &record, "game", true).is_err());
    }

    #[test]
    fn preopen_pins_reject_changes_and_model_mismatch() {
        let dir = crate::audit::tests::TestDir::new("preopen-pins");
        let config = dir.path().join("profiles.json");
        let pairings = dir.path().join("pairings.json");
        write_atomic_json(
            &config,
            &json!({"profiles": {
                "terra": {"provider":"cli", "model":"codex:gpt-6.1-terra", "effort":"low", "game_tokens":100_000}
            }}),
        )
        .unwrap();
        write_atomic_json(&pairings, &json!([["terra", "terra"]])).unwrap();
        let profiles = ["terra".to_owned(), "terra".to_owned()];
        let models = ["gpt-6.1-terra".to_owned(), "gpt-6.1-terra".to_owned()];
        let request = json!({
            "profiles_config_sha256":sha256_file(&config).unwrap(),
            "pairings_file":pairings, "pairings_sha256":sha256_file(&pairings).unwrap(),
        });
        let (_, expected_minds) =
            validate_profile_pins(&config, &request, &profiles, &models).unwrap();
        assert_eq!(expected_minds[0], crate::audit::tests::codex_identity());
        assert_eq!(expected_minds[1], crate::audit::tests::codex_identity());
        let wrong = ["wrong-model".to_owned(), models[1].clone()];
        assert!(validate_profile_pins(&config, &request, &profiles, &wrong).is_err());
        std::fs::write(&pairings, "[]").unwrap();
        assert!(validate_profile_pins(&config, &request, &profiles, &models).is_err());
        write_atomic_json(&pairings, &json!([["terra", "terra"]])).unwrap();
        std::fs::write(&config, "{}").unwrap();
        assert!(validate_profile_pins(&config, &request, &profiles, &models).is_err());
    }

    #[test]
    fn account_retry_requires_proven_cleanup() {
        assert!(!account_init_retry_safe(&anyhow!(
            "ambiguous login failure"
        )));
        assert!(!account_init_retry_safe(&anyhow!(
            "account deletion failed"
        )));
        let cleaned = anyhow::Error::new(CleanAccountInitFailure(anyhow!("presence failed")))
            .context("slot initialization");
        assert!(account_init_retry_safe(&cleaned));
    }

    #[test]
    fn ephemeral_username_satisfies_gateway_contract() {
        // Slot IDs never enter the credential name, including large slot IDs.
        for _ in 0..128 {
            let name = account_username(uuid::Uuid::now_v7());
            assert_eq!(name.len(), baylee_protocol::names::USERNAME_MAX);
            assert!(baylee_protocol::names::username(&name).is_ok());
        }
    }

    #[tokio::test]
    async fn tether_stays_open_until_explicit_cleanup() {
        let child = Command::new("/bin/sh")
            .args(["-c", "read first; read second"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap();
        let pgid = i32::try_from(child.id().unwrap()).unwrap();
        let mut guard = SeatProcessGroup {
            child,
            stdin: None,
            pgid,
            diag_task: None,
        };
        guard.stdin = guard.child.stdin.take();
        guard.deliver_secret("placeholder").await.unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            guard.try_wait().unwrap().is_none(),
            "tether EOF must not end a seated bridge"
        );
        guard.terminate().await.unwrap();
        assert!(!guard.group_alive());
    }

    #[tokio::test]
    async fn owned_child_is_killed_and_reaped_before_cleanup_succeeds() {
        let child = Command::new("/bin/sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap();
        let pgid = i32::try_from(child.id().unwrap()).unwrap();
        let mut guard = SeatProcessGroup {
            child,
            stdin: None,
            pgid,
            diag_task: None,
        };
        guard.terminate().await.unwrap();
        assert!(!guard.group_alive());
        assert!(guard.try_wait().unwrap().is_some());
    }

    #[tokio::test]
    async fn launch_pacer_spaces_concurrent_callers() {
        let pacer = Arc::new(LaunchPacer::new(Duration::from_millis(40)));
        let t0 = Instant::now();
        let mut handles = Vec::new();
        for _ in 0..3 {
            let p = Arc::clone(&pacer);
            handles.push(tokio::spawn(async move {
                p.wait_turn().await;
                Instant::now()
            }));
        }
        let mut stamps = Vec::new();
        for h in handles {
            stamps.push(h.await.unwrap());
        }
        stamps.sort();
        assert!(stamps[2].duration_since(t0) >= Duration::from_millis(75));
    }

    #[tokio::test]
    async fn presence_answers_websocket_pings_and_closes_cleanly() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            ws.send(Message::Ping(vec![9, 8, 7].into())).await.unwrap();
            let reply = tokio::time::timeout(Duration::from_secs(2), ws.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(reply, Message::Pong(vec![9, 8, 7].into()));
        });
        let mut presence = Presence::connect_with_ticket(&format!("http://{addr}"), "test-ticket")
            .await
            .unwrap();
        assert!(presence.check().is_ok());
        server.await.unwrap();
        presence.close().await;
    }
}
