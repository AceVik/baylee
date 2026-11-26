//! The gateway attachment, outside the deterministic rules driver.
use super::{Attach, wall_ms, ws_config};
#[cfg(not(feature = "tower"))]
use baylee_engine_server::variant::CompiledRunner as EngineRunner;
use baylee_protocol::v1::{self, Envelope};
use futures_util::SinkExt;
#[cfg(not(feature = "tower"))]
use futures_util::StreamExt;
use prost::Message as _;
use tokio_tungstenite::tungstenite::Message;

#[cfg(not(feature = "tower"))]
mod standard;
#[cfg(feature = "tower")]
mod tower;

/// Plays one game against the gateway and returns when it ends.
pub async fn run(attach: Attach) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _owner = if reports_to_owner() {
        Some(baylee_worker::watch_owner()?)
    } else {
        None
    };
    #[cfg(not(feature = "tower"))]
    let pump_budget = super::pump_config::from_env()?;
    #[cfg(not(feature = "tower"))]
    let runner = EngineRunner::default()
        .with_pump_budget(pump_budget)
        .with_foresight(foresight()?);
    // `unix:<path>` is the gateway's unix socket (`BAYLEE_UNIX_SOCKET`),
    // which an agent on the same machine hands its engines; the engine
    // plane is at the usual path on it.
    #[cfg(unix)]
    if let Some(path) = baylee_protocol::unix_socket(&attach.url) {
        let socket = tokio::net::UnixStream::connect(path).await?;
        let (ws, _) = tokio_tungstenite::client_async_with_config(
            "ws://localhost/engine/ws",
            socket,
            Some(link_config()),
        )
        .await?;
        #[cfg(not(feature = "tower"))]
        return Box::pin(standard::play(ws, attach, runner)).await;
        #[cfg(feature = "tower")]
        return Box::pin(tower::play(ws, attach)).await;
    }
    let ws = dial(&attach.url).await?;
    #[cfg(not(feature = "tower"))]
    return Box::pin(standard::play(ws, attach, runner)).await;
    #[cfg(feature = "tower")]
    return Box::pin(tower::play(ws, attach)).await;
}

/// What the house's search may spend here, from the operator's settings
/// (`baylee_engine_server::foresight`), said once in the log.
#[cfg(not(feature = "tower"))]
fn foresight()
-> Result<Option<baylee_gamehost::HostForesight>, Box<dyn std::error::Error + Send + Sync>> {
    let settings = baylee_engine_server::foresight::Settings::from_env()?;
    let host = settings.host()?;
    match (&host, settings.on) {
        (Some(_), _) => tracing::info!(
            searches = settings.searches,
            work_percent = settings.percent,
            wall_ms = settings.wall_ms,
            "the house searches within its levels' bounds"
        ),
        (None, true) if settings.slots.is_none() => tracing::warn!(
            "BAYLEE_FORESIGHT_SLOTS is not set (the agent sets it): the house answers \
             without searching"
        ),
        // A slots directory that is not there said so itself.
        (None, true) => {}
        (None, false) => {
            tracing::info!("BAYLEE_FORESIGHT=off: the house answers without searching");
        }
    }
    Ok(host)
}

/// The most one message from the gateway may be: a `GameSetup` that carries
/// the stored record of a game resumed after a restart (`resume_record`),
/// which the gateway keeps to 64 MiB (`MAX_RECORD_BYTES` there), and its
/// preset beside it.
pub(super) const MAX_LINK_MESSAGE: usize = 72 << 20;

const _: () = assert!(
    MAX_LINK_MESSAGE >= (64 << 20) + (4 << 20),
    "a stored record and a preset fit in one message"
);

/// The link to the gateway: [`ws_config`] with room for a resumed record.
/// Only on this side: the peer is the gateway its agent named, which
/// already decides every game this process plays, not a stranger, and an
/// ordinary message stays as small as before.
pub(super) fn link_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    ws_config()
        .max_message_size(Some(MAX_LINK_MESSAGE))
        .max_frame_size(Some(MAX_LINK_MESSAGE))
}

/// Dials the gateway over TCP with Nagle's algorithm off (`TCP_NODELAY`).
///
/// An engine sends its frames in bursts (a view, then the question about
/// it), and with Nagle on each after the first waits until the gateway
/// acknowledges the one before it, which Linux delays by up to 40 ms (the
/// gateway's `send_at_once` has the numbers).
pub(super) async fn dial(
    url: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Error,
> {
    let (ws, _) =
        tokio_tungstenite::connect_async_with_config(url, Some(link_config()), true).await?;
    Ok(ws)
}

fn engine_hello(attach: &Attach) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
            game_id: attach.game_id.clone(),
            token: attach.token.clone(),
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
            pid: std::process::id(),
            build: baylee_build::short().to_owned(),
            execution: baylee_engine_server::variant::identity().to_metadata(),
        })),
    }
}

/// This pipe is installed locally by the agent. Logs use stderr and the
/// metadata-only command returns before any reporter is constructed.
fn reporter() -> std::io::Result<baylee_worker::Reporter<std::io::Stdout>> {
    baylee_worker::Reporter::new(reports_to_owner().then(std::io::stdout))
}

fn reports_to_owner() -> bool {
    std::env::var_os(baylee_worker::REPORT_ENV).is_some_and(|value| value == "1")
}

/// Sleeps until a seat's deadline, or forever when nothing is on the clock.
async fn deadline(at: Option<tokio::time::Instant>) {
    match at {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

async fn send<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    envelope: &Envelope,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    ws.send(Message::Binary(envelope.encode_to_vec().into()))
        .await?;
    Ok(())
}

async fn send_batch<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    envelopes: &[Envelope],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    if envelopes.is_empty() {
        return Ok(());
    }
    for envelope in envelopes {
        ws.feed(Message::Binary(envelope.encode_to_vec().into()))
            .await?;
    }
    ws.flush().await?;
    Ok(())
}
