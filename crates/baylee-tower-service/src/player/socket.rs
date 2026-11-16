//! The socket changes transport, not authorization, ordering or mutation receipts.
use super::{
    Failure, Players,
    live::{Change, Changes},
};
use axum::{
    extract::{
        OriginalUri, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use baylee_tower_api::{
    self as api, Request,
    socket::{Refusal, Server},
};
use central_api::{Token, tower_access::Identity};
use sha2::{Digest as _, Sha256};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, broadcast};
use uuid::Uuid;

const HEARTBEAT: Duration = Duration::from_secs(15);
const FLUSH: Duration = Duration::from_millis(300);

pub(super) async fn upgrade(
    State(players): State<Arc<Players>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    let admission = admit(&players, &headers, uri.query().is_some()).await;
    let (token, identity, lease) = match admission {
        Ok(admission) => admission,
        Err(error) => return crate::http::reply(Err(error)),
    };
    let changed = players
        .live
        .get_or_init(|| Changes::start(players.store.clone()))
        .subscribe();
    upgrade
        .max_message_size(api::socket::MAX_CLIENT_BYTES)
        .max_frame_size(api::socket::MAX_CLIENT_BYTES)
        .write_buffer_size(0)
        .max_write_buffer_size(api::socket::MAX_SERVER_BYTES + 1024)
        .on_upgrade(move |socket| serve(socket, players, token, identity, changed, lease))
        .into_response()
}

struct Lease {
    players: Arc<Players>,
    account: Uuid,
    _slot: OwnedSemaphorePermit,
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut connections) = self.players.connections.lock()
            && let Some(count) = connections.get_mut(&self.account)
        {
            *count -= 1;
            if *count == 0 {
                connections.remove(&self.account);
            }
        }
    }
}

async fn admit(
    players: &Arc<Players>,
    headers: &HeaderMap,
    query: bool,
) -> Result<(Token, Identity, Lease), Failure> {
    if query
        || headers.contains_key(header::ORIGIN)
        || headers.contains_key(header::COOKIE)
        || headers.contains_key(header::CONTENT_ENCODING)
    {
        return Err(Failure::Invalid);
    }
    let _operation = players.slots.try_acquire().map_err(|_| Failure::Busy)?;
    if !players
        .limits
        .lock()
        .map_err(|_| Failure::Unavailable)?
        .global(Instant::now())
    {
        return Err(Failure::Busy);
    }
    let slot = Arc::clone(&players.sockets)
        .try_acquire_owned()
        .map_err(|_| Failure::Busy)?;
    let token = Token::from_bearer(
        crate::http::one(headers, "authorization").ok_or(Failure::Unauthorized)?,
    )
    .map_err(|_| Failure::Unauthorized)?;
    let identity = players.central.authenticate(&token).await?;
    let account = Uuid::from_bytes(identity.account);
    let mut connections = players
        .connections
        .lock()
        .map_err(|_| Failure::Unavailable)?;
    let count = connections.entry(account).or_default();
    if *count >= 2 {
        return Err(Failure::Busy);
    }
    *count += 1;
    Ok((
        token,
        identity,
        Lease {
            players: Arc::clone(players),
            account,
            _slot: slot,
        },
    ))
}

/// Only read selectors and a 32-byte equality digest are retained per subscription.
struct Watch {
    request: Request,
    digest: [u8; 32],
    dirty: bool,
}

fn digest(response: &api::Response) -> Result<[u8; 32], Failure> {
    let bytes = serde_json::to_vec(response).map_err(|_| Failure::Unavailable)?;
    if bytes.len() > api::MAX_RESPONSE_BYTES {
        return Err(Failure::Unavailable);
    }
    Ok(Sha256::digest(&bytes).into())
}

fn watch(
    watches: &mut Vec<Watch>,
    request: &Request,
    response: &api::Response,
) -> Result<(), Failure> {
    let selector = match response {
        api::Response::Run { run } => Request::Run { run: run.id },
        api::Response::Ranking { board } => Request::Ranking {
            cohort: board.cohort,
            lane: board.lane,
        },
        api::Response::Hosts { run, .. } => Request::Hosts {
            run: *run,
            before: None,
        },
        api::Response::Cohorts { .. } => request.clone(),
        _ => return Ok(()),
    };
    let hash = digest(response)?;
    let class = std::mem::discriminant(&selector);
    if let Some(old) = watches
        .iter_mut()
        .find(|w| std::mem::discriminant(&w.request) == class)
    {
        *old = Watch {
            request: selector,
            digest: hash,
            dirty: false,
        };
    } else if watches.len() < 4 {
        watches.push(Watch {
            request: selector,
            digest: hash,
            dirty: false,
        });
    }
    Ok(())
}

fn dirty(watches: &mut [Watch], change: Change) {
    for watch in watches {
        watch.dirty |= match (&watch.request, change) {
            (Request::Run { run }, Change::Run(changed)) => *run == changed,
            (_, Change::Resync)
            | (Request::Ranking { .. }, Change::Ranking)
            | (Request::Cohorts { .. }, Change::Catalog) => true,
            _ => false,
        };
    }
}

async fn authorized(players: &Players, token: &Token, account: Uuid) -> Result<Identity, Failure> {
    let identity = players.central.authenticate(token).await?;
    if identity.account != *account.as_bytes() {
        return Err(Failure::Unauthorized);
    }
    Ok(identity)
}

fn name(identity: &Identity) -> Option<api::ranking::Name> {
    identity
        .display_name
        .clone()
        .zip(identity.tag)
        .map(|(display, tag)| api::ranking::Name { display, tag })
}

async fn send(socket: &mut WebSocket, frame: &Server) -> Result<(), Failure> {
    let bytes = serde_json::to_vec(frame).map_err(|_| Failure::Unavailable)?;
    if bytes.len() > api::socket::MAX_SERVER_BYTES {
        return Err(Failure::Unavailable);
    }
    tokio::time::timeout(
        Duration::from_secs(5),
        socket.send(Message::Binary(bytes.into())),
    )
    .await
    .map_err(|_| Failure::Unavailable)?
    .map_err(|_| Failure::Unavailable)
}

async fn serve(
    mut socket: WebSocket,
    players: Arc<Players>,
    token: Token,
    identity: Identity,
    mut changes: broadcast::Receiver<Change>,
    _lease: Lease,
) {
    let account = Uuid::from_bytes(identity.account);
    if send(
        &mut socket,
        &Server::Ready {
            version: api::VERSION.into(),
        },
    )
    .await
    .is_err()
    {
        return;
    }
    let mut watches: Vec<Watch> = Vec::with_capacity(4);
    let mut heartbeat = tokio::time::interval(HEARTBEAT);
    let mut flush = tokio::time::interval(FLUSH);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    heartbeat.tick().await;
    let mut last_seen = Instant::now();
    loop {
        let result: Result<(), Failure> = tokio::select! {
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Binary(bytes))) => {
                        last_seen = Instant::now();
                        exchange(&mut socket, &players, &token, account, &bytes, &mut watches).await
                    }
                    Some(Ok(Message::Text(text))) => {
                        last_seen = Instant::now();
                        exchange(&mut socket, &players, &token, account, text.as_bytes(), &mut watches).await
                    }
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => { last_seen = Instant::now(); Ok(()) }
                    _ => break,
                }
            }
            change = changes.recv() => {
                dirty(&mut watches, change.unwrap_or(Change::Resync));
                Ok(())
            }
            _ = flush.tick(), if watches.iter().any(|w| w.dirty) => {
                refresh(&mut socket, &players, &token, account, &mut watches).await
            }
            _ = heartbeat.tick() => {
                if last_seen.elapsed() > HEARTBEAT * 3 { break; }
                // Renew scope checks even on an idle connection. Availability is
                // time-sensitive, so re-read only that small list, never the shop.
                for watch in &mut watches {
                    watch.dirty |= matches!(watch.request, Request::Hosts { .. });
                }
                match players.slots.try_acquire() {
                    Ok(_slot) => match authorized(&players, &token, account).await {
                        Ok(_) => tokio::time::timeout(Duration::from_secs(5), socket.send(Message::Ping(Vec::new().into())))
                            .await.map_err(|_| Failure::Unavailable).and_then(|r| r.map_err(|_| Failure::Unavailable)),
                        Err(error) => Err(error),
                    },
                    Err(_) => Ok(()),
                }
            }
        };
        if result.is_err() {
            break;
        }
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), socket.send(Message::Close(None))).await;
}

async fn exchange(
    socket: &mut WebSocket,
    players: &Players,
    token: &Token,
    account: Uuid,
    bytes: &[u8],
    watches: &mut Vec<Watch>,
) -> Result<(), Failure> {
    let frame = api::socket::decode(bytes).map_err(|_| Failure::Invalid)?;
    let result = execute(players, token, account, &frame.request).await;
    if let Ok(response) = &result {
        watch(watches, &frame.request, response)?;
    }
    let terminal = matches!(result, Err(Failure::Unauthorized | Failure::Terms));
    send(
        socket,
        &Server::Reply {
            serial: frame.serial,
            result: result.map_err(refusal),
        },
    )
    .await?;
    if terminal {
        return Err(Failure::Unauthorized);
    }
    Ok(())
}

async fn execute(
    players: &Players,
    token: &Token,
    account: Uuid,
    request: &Request,
) -> Result<api::Response, Failure> {
    let _slot = players.slots.try_acquire().map_err(|_| Failure::Busy)?;
    {
        let mut limits = players.limits.lock().map_err(|_| Failure::Unavailable)?;
        if !limits.global(Instant::now()) || !limits.account(account, Instant::now()) {
            return Err(Failure::Busy);
        }
    }
    let identity = authorized(players, token, account).await?;
    // Dispatch is awaited to completion even when the peer has gone: the durable
    // receipt resolves a lost ACK, never a timeout pretending the purchase failed.
    players
        .dispatch(account, request.clone(), name(&identity).as_ref())
        .await
}

async fn refresh(
    socket: &mut WebSocket,
    players: &Players,
    token: &Token,
    account: Uuid,
    watches: &mut [Watch],
) -> Result<(), Failure> {
    let Ok(_slot) = players.slots.try_acquire() else {
        return Ok(());
    };
    let identity = authorized(players, token, account).await?;
    let name = name(&identity);
    for watch in watches.iter_mut().filter(|w| w.dirty) {
        let response = players
            .dispatch(account, watch.request.clone(), name.as_ref())
            .await?;
        let hash = digest(&response)?;
        if hash != watch.digest {
            send(socket, &Server::Changed { response }).await?;
            watch.digest = hash;
        }
        watch.dirty = false;
    }
    Ok(())
}

fn refusal(failure: Failure) -> Refusal {
    match failure {
        Failure::Invalid | Failure::Version | Failure::Oversized => Refusal::Invalid,
        Failure::Unauthorized => Refusal::Unauthorized,
        Failure::Terms => Refusal::Terms,
        Failure::Missing => Refusal::Missing,
        Failure::Conflict => Refusal::Conflict,
        Failure::Busy => Refusal::Busy,
        Failure::Unavailable => Refusal::Unavailable,
    }
}
