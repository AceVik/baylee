//! Bounded frames and seat outboxes shared by the agent and engine planes.
use super::{HELLO_TIMEOUT_SECS, Shared};
use axum::extract::ws::{Message, WebSocket};
use baylee_protocol::v1::{self, Envelope};
use bytes::Bytes;
use prost::Message as _;
use tokio::sync::{broadcast, mpsc};

/// Seat `seat`'s channel in `game_id`, from `cache` or the lobby. `None` for
/// a seat the table does not have, or a game no longer here.
pub(super) fn outbox_of<'a>(
    state: &Shared,
    game_id: &str,
    cache: &'a mut Vec<Option<broadcast::Sender<Bytes>>>,
    seat: u32,
) -> Option<&'a broadcast::Sender<Bytes>> {
    let seat = usize::try_from(seat).ok()?;
    if cache.get(seat).is_none_or(Option::is_none) {
        let outbox = state
            .lobby
            .lock()
            .games
            .get_mut(game_id)
            .filter(|game| seat < game.seats.len())
            .map(|game| game.outbox(seat))?;
        if cache.len() <= seat {
            cache.resize(seat + 1, None);
        }
        cache[seat] = Some(outbox);
    }
    cache[seat].as_ref()
}

/// The public spectator channel in `game_id`, from `cache` or the lobby.
pub(super) fn spectator_outbox_of<'a>(
    state: &Shared,
    game_id: &str,
    cache: &'a mut Option<broadcast::Sender<Bytes>>,
) -> Option<&'a broadcast::Sender<Bytes>> {
    if cache.is_none() {
        *cache = state
            .lobby
            .lock()
            .games
            .get(game_id)
            .map(|g| g.spectator_outbox.clone());
    }
    cache.as_ref()
}

/// The one frame a refused agent or engine is sent before its socket closes.
pub(super) fn refusal(why: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: why.to_string(),
        })),
    }
}

// ------------------------------------------------------------------ frames

/// Reads the first frame of a control or engine socket.
///
/// Whatever identifies the peer has to be the first thing it says: a socket
/// that opens and then waits is not one this gateway keeps.
pub(super) async fn hello_of(socket: &mut WebSocket) -> Option<v1::envelope::Msg> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(HELLO_TIMEOUT_SECS);
    loop {
        let frame = tokio::time::timeout_at(deadline, socket.recv())
            .await
            .ok()?;
        match frame? {
            Ok(Message::Binary(data)) => return Envelope::decode(data).ok()?.msg,
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => {}
        }
    }
}

/// What one received frame amounts to.
///
/// "Nothing to act on" and "the socket is done" are different answers and only
/// one of them ends the loop, which is why this is three cases and not an
/// `Option`.
pub(super) enum Incoming {
    /// A message this plane may act on.
    Msg(v1::envelope::Msg),
    /// A frame that carried nothing — not a reason to hang up.
    Ignored,
    /// The socket is gone.
    Closed,
}

/// Reads one received frame.
pub(super) fn next_envelope(frame: Option<Result<Message, axum::Error>>) -> Incoming {
    match frame {
        Some(Ok(Message::Binary(data))) => Envelope::decode(data)
            .ok()
            .and_then(|e| e.msg)
            .map_or(Incoming::Ignored, Incoming::Msg),
        Some(Ok(Message::Close(_)) | Err(_)) | None => Incoming::Closed,
        // Pings are answered by axum; other frame kinds are not this protocol.
        Some(Ok(_)) => Incoming::Ignored,
    }
}

/// Sends one envelope, or reports that the socket is gone.
pub(super) async fn send(socket: &mut WebSocket, envelope: &Envelope) -> Result<(), ()> {
    futures_util::SinkExt::send(socket, Message::Binary(envelope.encode_to_vec().into()))
        .await
        .map_err(|_| ())
}

/// Sends `first` and any queued envelopes on `rx` in one flush, or reports
/// that the socket is gone.
pub(super) async fn send_queued(
    socket: &mut WebSocket,
    first: &Envelope,
    rx: &mut mpsc::UnboundedReceiver<Envelope>,
) -> Result<(), ()> {
    use futures_util::SinkExt as _;
    socket
        .feed(Message::Binary(first.encode_to_vec().into()))
        .await
        .map_err(|_| ())?;
    while let Ok(next) = rx.try_recv() {
        socket
            .feed(Message::Binary(next.encode_to_vec().into()))
            .await
            .map_err(|_| ())?;
    }
    socket.flush().await.map_err(|_| ())
}
