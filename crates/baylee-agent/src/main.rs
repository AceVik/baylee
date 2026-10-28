//! baylee-agent — starts engines for a gateway, and reports what became of them.
//!
//! One socket to the gateway, one child process per game. The agent is the
//! only thing in the topology that spawns anything, which is why it is also
//! the only thing that needs to be allowed to: a gateway that could start
//! processes would be a gateway worth attacking for that reason alone.
//!
//! ```bash
//! BAYLEE_AGENT_TOKEN=… BAYLEE_GATEWAY=http://127.0.0.1:28766 baylee-agent
//! ```
//!
//! The decisions live in [`baylee_agent`]; this file is the socket, the child
//! processes and the reconnect loop.

use baylee_agent::{AgentConfig, Order, engine_argv, heartbeat, order, status};
use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

/// How long to wait before dialling the gateway again, and the ceiling.
const RETRY_START: std::time::Duration = std::time::Duration::from_secs(1);
/// Backoff ceiling. A gateway that is down for an hour should still be found
/// within half a minute of coming back.
const RETRY_MAX: std::time::Duration = std::time::Duration::from_secs(30);

/// Frame budget on the control socket.
///
/// Nothing here is large — the biggest control message is a `StartEngine`
/// carrying three short strings — so the default 64 MiB is 64 MiB of trust
/// this link has no reason to extend.
fn ws_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(1 << 20))
        .max_frame_size(Some(1 << 20))
}

/// Engines this agent started, by game, each holding the switch that stops it.
type Running = Arc<Mutex<HashMap<String, Option<oneshot::Sender<()>>>>>;

/// Cancelling a session must cancel its socket writer and heartbeat too.
struct SessionTasks {
    /// The sole control-socket writer, aborted on cancellation.
    writer: tokio::task::JoinHandle<()>,
    /// Heartbeats belong to this control session only.
    beat: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for SessionTasks {
    fn drop(&mut self) {
        self.writer.abort();
        if let Some(beat) = &self.beat {
            beat.abort();
        }
    }
}

/// The gateway has no engine reattachment protocol. A lost control session
/// therefore ends only this agent's engines, never a player's seat reconnect.
async fn stop_all(running: &Running) {
    {
        let mut entries = running.lock().expect("agent process table");
        for switch in entries.values_mut() {
            if let Some(switch) = switch.take() {
                let _ = switch.send(());
            }
        }
    }
    // Entries remain owned until their supervisor has successfully reaped them.
    while !running.lock().expect("agent process table").is_empty() {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

/// Wait for either supported process shutdown signal.
async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = term.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let config = match AgentConfig::from_env() {
        Ok(config) => config,
        Err(reason) => {
            tracing::error!(%reason, "agent not configured");
            std::process::exit(2);
        }
    };
    tracing::info!(
        gateway = config.gateway,
        name = config.name,
        capacity = config.capacity,
        engine = %config.engine_bin.display(),
        "baylee-agent starting"
    );

    let running: Running = Arc::new(Mutex::new(HashMap::new()));
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    let mut wait = RETRY_START;
    loop {
        let ended = tokio::select! {
            signal = &mut shutdown => {
                if let Err(err) = signal {
                    tracing::error!(%err, "shutdown signal failed");
                }
                stop_all(&running).await;
                break;
            }
            result = session(&config, &running) => match result {
                Ok(ended) => ended,
                Err(err) => {
                    tracing::warn!(%err, "control socket failed");
                    Ended::Failed
                }
            }
        };
        stop_all(&running).await;
        if ended == Ended::Closed {
            tracing::info!("gateway closed the control socket");
        }
        let (now, next) = redial_after(&ended, wait);
        tokio::select! {
            _ = &mut shutdown => break,
            () = tokio::time::sleep(now) => {}
        }
        wait = next;
    }
}

/// How one session with the gateway ended, as far as dialling it again goes.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    /// The gateway closed the socket.
    Closed,
    /// The gateway refused this agent, and said why (#271).
    Refused,
    /// The dial or the socket failed.
    Failed,
}

/// The wait before the next dial, given how the last session ended and the
/// wait it was dialled after, and the wait to keep for the dial after that.
///
/// A socket the gateway closed starts again from [`RETRY_START`]: the
/// gateway was there, and a restart is the likeliest reason. A failure
/// doubles towards [`RETRY_MAX`], and so does a refusal: the same agent is
/// refused again until someone changes a binary, and a refusal counted as a
/// close would dial a gateway that said no once a second for as long as
/// nobody looked.
fn redial_after(
    ended: &Ended,
    last: std::time::Duration,
) -> (std::time::Duration, std::time::Duration) {
    let now = match ended {
        Ended::Closed => RETRY_START,
        Ended::Refused | Ended::Failed => last,
    };
    (now, (now * 2).min(RETRY_MAX))
}

/// One connection to the gateway, from hello to close.
async fn session(
    config: &AgentConfig,
    running: &Running,
) -> Result<Ended, Box<dyn std::error::Error + Send + Sync>> {
    let url = config.control_url();
    #[cfg(unix)]
    if let Some(path) = baylee_protocol::unix_socket(&config.gateway) {
        let socket = tokio::net::UnixStream::connect(path)
            .await
            .map_err(|err| format!("dial {}: {err}", config.gateway))?;
        let (ws, _) = tokio_tungstenite::client_async_with_config(&url, socket, Some(ws_config()))
            .await
            .map_err(|err| format!("dial {}: {err}", config.gateway))?;
        return run(ws, config, running).await;
    }
    let ws = dial(&url)
        .await
        .map_err(|err| format!("dial {url}: {err}"))?;
    run(ws, config, running).await
}

/// Dials the gateway over TCP with Nagle's algorithm off (`TCP_NODELAY`),
/// as every TCP socket between gateway, agent, engine and seat is: a frame
/// goes out when it is written, not after the peer has acknowledged the one
/// before it, which Linux delays by up to 40 ms.
async fn dial(
    url: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Error,
> {
    let (ws, _) =
        tokio_tungstenite::connect_async_with_config(url, Some(ws_config()), true).await?;
    Ok(ws)
}

/// One connection to the gateway, once dialled, over whichever socket.
async fn run<S>(
    ws: tokio_tungstenite::WebSocketStream<S>,
    config: &AgentConfig,
    running: &Running,
) -> Result<Ended, Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut stream) = ws.split();

    // Everything that writes to the gateway does it through this queue: the
    // read loop, the heartbeat and every child's waiter task all report, and
    // a single writer keeps the frames whole.
    let (out, mut outbox) = mpsc::unbounded_channel::<Envelope>();
    out.send(config.hello())?;
    let writer = tokio::spawn(async move {
        while let Some(envelope) = outbox.recv().await {
            if sink
                .feed(Message::Binary(envelope.encode_to_vec().into()))
                .await
                .is_err()
            {
                break;
            }
            while let Ok(next) = outbox.try_recv() {
                if sink
                    .feed(Message::Binary(next.encode_to_vec().into()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            if sink.flush().await.is_err() {
                break;
            }
        }
    });

    let mut tasks = SessionTasks { writer, beat: None };
    let result = loop {
        let frame = tokio::select! {
            _ = &mut tasks.writer => break Err(Box::from("control writer exited")),
            frame = stream.next() => frame,
        };
        let Some(frame) = frame else {
            break Ok(Ended::Closed);
        };
        let frame = match frame {
            Ok(frame) => frame,
            Err(err) => break Err(Box::from(err.to_string())),
        };
        if frame.is_close() {
            break Ok(Ended::Closed);
        }
        if !frame.is_binary() {
            continue;
        }
        let envelope = match Envelope::decode(frame.into_data()) {
            Ok(envelope) => envelope,
            Err(err) => {
                tracing::warn!(%err, "undecodable control frame");
                continue;
            }
        };
        match order(envelope) {
            Order::Welcomed {
                agent_id,
                heartbeat_secs,
            } => {
                tracing::info!(agent_id, "registered with the gateway");
                if heartbeat_secs > 0
                    && let Some(old) = tasks.beat.replace(tokio::spawn(beating(
                        out.clone(),
                        u64::from(heartbeat_secs),
                    )))
                {
                    old.abort();
                }
            }
            Order::Start {
                game_id,
                engine_token,
                gateway_url,
            } => start(config, running, &out, &game_id, &engine_token, &gateway_url),
            Order::Stop { game_id } => stop(running, &game_id),
            // Error level: nothing this agent does will be accepted until
            // someone changes a binary, and the sentence says which.
            Order::Refused(why) => {
                tracing::error!(why, "the gateway refused this agent");
                break Ok(Ended::Refused);
            }
            Order::Nothing => {}
        }
    };

    drop(tasks);
    stop_all(running).await;
    result
}

/// Tells the gateway this agent is still here.
async fn beating(out: mpsc::UnboundedSender<Envelope>, secs: u64) {
    let mut ticks = tokio::time::interval(std::time::Duration::from_secs(secs));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick is immediate; the welcome already said we are here.
    ticks.tick().await;
    loop {
        ticks.tick().await;
        if out.send(heartbeat(0)).is_err() {
            break;
        }
    }
}

/// Starts one engine, or says why it did not.
fn start(
    config: &AgentConfig,
    running: &Running,
    out: &mpsc::UnboundedSender<Envelope>,
    game_id: &str,
    engine_token: &str,
    gateway_url: &str,
) {
    let (stop_tx, stop_rx) = oneshot::channel();
    {
        let mut running = running.lock().expect("agent process table");
        if running.contains_key(game_id) {
            // The gateway asked twice — a reconnect replaying its queue, most
            // likely. The engine is already dialling it; saying so is enough.
            let _ = out.send(status(
                game_id,
                v1::engine_status::Kind::Started,
                "already running",
            ));
            return;
        }
        if !config.has_room(running.len()) {
            let _ = out.send(status(
                game_id,
                v1::engine_status::Kind::Failed,
                "at capacity",
            ));
            return;
        }
        running.insert(game_id.to_string(), Some(stop_tx));
    }

    let child = tokio::process::Command::new(&config.engine_bin)
        .args(engine_argv(
            config.engine_address(gateway_url),
            game_id,
            engine_token,
        ))
        // A dropped agent must not leave engines behind holding games nobody
        // is routing any more.
        .kill_on_drop(true)
        .spawn();
    let child = match child {
        Ok(child) => child,
        Err(err) => {
            running.lock().expect("agent process table").remove(game_id);
            tracing::error!(game_id, %err, "could not start an engine");
            let _ = out.send(status(
                game_id,
                v1::engine_status::Kind::Failed,
                &err.to_string(),
            ));
            return;
        }
    };
    tracing::info!(game_id, pid = child.id(), "engine started");
    let _ = out.send(status(game_id, v1::engine_status::Kind::Started, ""));
    tokio::spawn(supervise(
        child,
        stop_rx,
        running.clone(),
        out.clone(),
        game_id.to_string(),
    ));
}

/// Waits for one engine to exit — or stops it — and reports either way.
async fn supervise(
    mut child: tokio::process::Child,
    stop: oneshot::Receiver<()>,
    running: Running,
    out: mpsc::UnboundedSender<Envelope>,
    game_id: String,
) {
    let detail = tokio::select! {
        exit = child.wait() => match exit {
            Ok(exit) => exit.to_string(),
            Err(err) => {
                tracing::error!(game_id, %err, "engine wait failed; retaining ownership");
                reap_after_error(&mut child, &game_id).await
            },
        },
        _ = stop => {
            // Stop only this owned process, then confirm its exit before
            // releasing the slot. The gateway has already ended the game.
            let _ = child.start_kill();
            match child.wait().await {
                Ok(exit) => format!("stopped ({exit})"),
                Err(err) => {
                    tracing::error!(game_id, %err, "engine stop wait failed; retaining ownership");
                    reap_after_error(&mut child, &game_id).await
                },
            }
        }
    };
    running
        .lock()
        .expect("agent process table")
        .remove(&game_id);
    tracing::info!(game_id, detail, "engine gone");
    let _ = out.send(status(&game_id, v1::engine_status::Kind::Exited, &detail));
}

/// Preserve ownership and retry after an OS wait failure.
async fn reap_after_error(child: &mut tokio::process::Child, game_id: &str) -> String {
    loop {
        let _ = child.start_kill();
        match child.wait().await {
            Ok(exit) => return format!("stopped ({exit})"),
            Err(err) => tracing::error!(game_id, %err, "engine reap failed; retaining ownership"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Signals one engine to stop. A game the agent does not have is not an error:
/// the engine may have exited on its own a moment earlier.
fn stop(running: &Running, game_id: &str) {
    let switch = running
        .lock()
        .expect("agent process table")
        .get_mut(game_id)
        .and_then(Option::take);
    if let Some(switch) = switch {
        let _ = switch.send(());
    } else {
        tracing::debug!(game_id, "stop for a game this agent is not running");
    }
}

#[cfg(test)]
mod tests {
    use super::{Ended, RETRY_MAX, RETRY_START, dial, redial_after};

    /// Start a harmless engine owned by the normal supervisor.
    #[cfg(target_os = "linux")]
    fn owned_child(running: &super::Running, game: &str) -> u32 {
        let child = tokio::process::Command::new("/bin/sleep")
            .arg("60")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        running.lock().unwrap().insert(game.into(), Some(tx));
        let (out, _received) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(super::supervise(
            child,
            rx,
            running.clone(),
            out,
            game.into(),
        ));
        pid
    }

    /// Stop does not make capacity available before reaping.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn stop_keeps_the_slot_owned_until_the_child_is_reaped() {
        let running: super::Running = std::sync::Arc::default();
        let pid = owned_child(&running, "game");
        super::stop(&running, "game");
        assert!(running.lock().unwrap().contains_key("game"));
        assert!(running.lock().unwrap()["game"].is_none());
        // A repeated Stop cannot erase the entry or permit a replacement Start.
        super::stop(&running, "game");
        assert!(running.lock().unwrap().contains_key("game"));
        tokio::time::timeout(std::time::Duration::from_secs(5), super::stop_all(&running))
            .await
            .unwrap();
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    /// A closed control link cannot retain live engine children.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn control_close_reaps_children_without_waiting_for_their_output_senders() {
        let running: super::Running = std::sync::Arc::default();
        // A control close must reap an existing owned engine before returning.
        let config = baylee_agent::AgentConfig {
            gateway: "http://127.0.0.1:1".into(),
            token: "test-placeholder".into(),
            name: "test".into(),
            capacity: 1,
            engine_bin: "/bin/cat".into(),
        };
        let pid = owned_child(&running, "game");
        let (a, b) = tokio::io::duplex(4096);
        let client = tokio_tungstenite::WebSocketStream::from_raw_socket(
            a,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;
        let mut peer = tokio_tungstenite::WebSocketStream::from_raw_socket(
            b,
            tokio_tungstenite::tungstenite::protocol::Role::Server,
            None,
        )
        .await;
        peer.close(None).await.unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            super::run(client, &config, &running),
        )
        .await
        .unwrap();
        assert!(matches!(result, Ok(Ended::Closed) | Err(_)));
        assert!(running.lock().unwrap().is_empty());
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    /// Session cancellation drops an idle writer even while senders survive.
    #[tokio::test]
    async fn cancelling_a_session_cancels_an_idle_writer_with_live_senders() {
        let (out, mut inbox) = tokio::sync::mpsc::unbounded_channel::<()>();
        let writer = tokio::spawn(async move { while inbox.recv().await.is_some() {} });
        let cancellation = writer.abort_handle();
        let tasks = super::SessionTasks { writer, beat: None };
        drop(tasks);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !cancellation.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(out.send(()).is_err());
    }

    /// Cleanup must not affect processes absent from the ownership table.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn shutdown_reaps_only_owned_children() {
        let running: super::Running = std::sync::Arc::default();
        let pid = owned_child(&running, "game");
        let mut unrelated = tokio::process::Command::new("/bin/sleep")
            .arg("60")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), super::stop_all(&running))
            .await
            .unwrap();
        let unrelated_alive = unrelated.try_wait().unwrap().is_none();
        unrelated.kill().await.unwrap();
        assert!(unrelated_alive);
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    /// The control socket is opened with Nagle's algorithm off.
    #[tokio::test]
    async fn the_gateway_is_dialled_with_nagle_off() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/agent/ws", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = tokio_tungstenite::accept_async(stream).await;
        });
        let ws = dial(&url).await.unwrap();
        let tokio_tungstenite::MaybeTlsStream::Plain(tcp) = ws.get_ref() else {
            panic!("a plain TCP socket");
        };
        assert!(tcp.nodelay().unwrap());
    }

    /// A refused agent backs off as a failed one does (#271), and a socket
    /// the gateway closed starts again from the first second.
    #[test]
    fn a_refusal_grows_the_wait_and_a_close_resets_it() {
        let mut wait = RETRY_START;
        let mut waited = Vec::new();
        for ended in [
            Ended::Refused,
            Ended::Refused,
            Ended::Refused,
            Ended::Closed,
            Ended::Failed,
        ] {
            let (now, next) = redial_after(&ended, wait);
            waited.push(now.as_secs());
            wait = next;
        }
        assert_eq!(waited, [1, 2, 4, 1, 2]);
        for _ in 0..8 {
            wait = redial_after(&Ended::Refused, wait).1;
        }
        assert_eq!(wait, RETRY_MAX, "and it stops growing at the ceiling");
    }
}
