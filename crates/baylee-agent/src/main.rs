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
type Running = Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>;

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

    // Children outlive one connection on purpose: a gateway restart must not
    // kill the games it is about to ask for again. They talk to the gateway
    // over their own sockets, not this one.
    let running: Running = Arc::new(Mutex::new(HashMap::new()));
    let mut backoff = RETRY_START;
    loop {
        match session(&config, &running).await {
            Ok(()) => {
                tracing::info!("gateway closed the control socket");
                backoff = RETRY_START;
            }
            Err(err) => tracing::warn!(%err, "control socket failed"),
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(RETRY_MAX);
    }
}

/// One connection to the gateway, from hello to close.
async fn session(
    config: &AgentConfig,
    running: &Running,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = config.control_url();
    let (ws, _) = tokio_tungstenite::connect_async_with_config(&url, Some(ws_config()), false)
        .await
        .map_err(|err| format!("dial {url}: {err}"))?;
    let (mut sink, mut stream) = ws.split();

    // Everything that writes to the gateway does it through this queue: the
    // read loop, the heartbeat and every child's waiter task all report, and
    // a single writer keeps the frames whole.
    let (out, mut outbox) = mpsc::unbounded_channel::<Envelope>();
    out.send(config.hello())?;
    let writer = tokio::spawn(async move {
        while let Some(envelope) = outbox.recv().await {
            if sink
                .send(Message::Binary(envelope.encode_to_vec().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let mut beat: Option<tokio::task::JoinHandle<()>> = None;
    let result = loop {
        let Some(frame) = stream.next().await else {
            break Ok(());
        };
        let frame = match frame {
            Ok(frame) => frame,
            Err(err) => break Err(Box::from(err.to_string())),
        };
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
                    && let Some(old) = beat.replace(tokio::spawn(beating(
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
            Order::Nothing => {}
        }
    };

    if let Some(beat) = beat {
        beat.abort();
    }
    drop(out);
    let _ = writer.await;
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
        running.insert(game_id.to_string(), stop_tx);
    }

    let child = tokio::process::Command::new(&config.engine_bin)
        .args(engine_argv(gateway_url, game_id, engine_token))
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
            Err(err) => err.to_string(),
        },
        _ = stop => {
            // The game is over and the gateway said so. Ask first; the engine
            // closes its socket and leaves on its own.
            let _ = child.start_kill();
            match child.wait().await {
                Ok(exit) => format!("stopped ({exit})"),
                Err(err) => err.to_string(),
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

/// Signals one engine to stop. A game the agent does not have is not an error:
/// the engine may have exited on its own a moment earlier.
fn stop(running: &Running, game_id: &str) {
    let switch = running.lock().expect("agent process table").remove(game_id);
    if let Some(switch) = switch {
        let _ = switch.send(());
    } else {
        tracing::debug!(game_id, "stop for a game this agent is not running");
    }
}
