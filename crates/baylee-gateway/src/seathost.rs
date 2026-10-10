//! Hosted language-model seats (`docs/protocol.md` §"Hosted language-model
//! seats"): the seat agents connected to this gateway, the profiles they
//! report, a host's order for one, and the console's view and changes.
//!
//! The gateway derives nothing here: a profile's state, games and spend are
//! what its seat agent last reported, kept in memory and served. It decides
//! only which seat agent takes an order, by those reports: of the ones that
//! say the profile is available and below its bound, the least busy.
//!
//! A hosted chair is a host's own chair ticket minted by the gateway: the
//! chair is marked with the order (`LobbySeat::hosted`), the ticket names
//! the order (`Grant::Chair::order`), and the seat agent's bridge redeems it
//! as the host's delegate, so everything a delegate is (the host answers
//! for it, the record names the host, it goes when the host does) carries
//! over. A key travels only over the unix socket, write-only, and is held
//! by nothing here past the request.

use crate::{ErrorBody, Shared, auth, authed_session, chair, err, err_saying, lobby, wsticket};
use axum::Extension;
use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use baylee_protocol::seathost::{
    ControlAction, Definition, Frame, Reported, SeatStatusKind, State as ProfileState, valid_name,
};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

/// How often a seat agent says it is there; two missed and it is gone.
pub(crate) const HEARTBEAT_SECS: u32 = 30;

/// How long a seat agent has to say who it is.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// How long an ordered chair may take to say ready before it is given up.
pub(crate) const READY_WITHIN: Duration = Duration::from_secs(60);

/// How long the console waits for a seat agent's answer.
const ANSWER_WITHIN: Duration = Duration::from_secs(10);

/// The largest deck text an order may carry.
const MAX_DECK_BYTES: usize = 64 * 1024;

/// The largest frame a seat agent may send: a report of many profiles.
const MAX_FRAME_BYTES: usize = 1 << 20;

/// Why a guest may not order one.
pub(crate) const GUEST_REFUSED: &str =
    "a guest cannot seat a hosted model; sign in with an account";

/// One connected seat agent.
pub(crate) struct SeatHost {
    /// Whether it came in on the unix socket, i.e. runs on this machine.
    pub(crate) local: bool,
    /// Its bound on bridges; 0 = none.
    pub(crate) capacity: u32,
    /// Frames to it.
    tx: mpsc::UnboundedSender<Frame>,
    /// What it last reported.
    pub(crate) profiles: Vec<Reported>,
    /// Since when it is connected.
    since: Instant,
    /// The console's requests waiting for its answer.
    waiting: HashMap<String, oneshot::Sender<Result<(), String>>>,
    /// Orders sent and not yet answered by a status, by profile.
    starting: HashMap<String, String>,
    /// This connection's identity, so a stale socket's end removes nothing.
    conn: u64,
}

impl SeatHost {
    fn starting_of(&self, profile: &str) -> u32 {
        u32::try_from(self.starting.values().filter(|p| *p == profile).count()).unwrap_or(u32::MAX)
    }

    fn live(&self) -> u32 {
        self.profiles.iter().map(|p| p.games).sum::<u32>()
            + u32::try_from(self.starting.len()).unwrap_or(u32::MAX)
    }
}

/// Where an order is.
#[derive(Clone)]
struct Placed {
    host: String,
    game_id: String,
    seat: usize,
}

/// Every seat agent connected now, and the orders sent to them.
#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) hosts: BTreeMap<String, SeatHost>,
    orders: HashMap<String, Placed>,
    next_conn: u64,
}

/// One profile as players see it: one row per id over every seat agent.
pub(crate) struct Row<'a> {
    pub(crate) id: &'a str,
    pub(crate) first: &'a Reported,
    pub(crate) state: ProfileState,
    pub(crate) until: Option<i64>,
    pub(crate) games: u32,
    pub(crate) max_games: Option<u32>,
    pub(crate) hosts: Vec<(&'a str, &'a Reported)>,
}

impl Row<'_> {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "label": self.first.label,
            "vendor": self.first.vendor,
            "kind": self.first.kind,
            "model": self.first.model,
            "state": self.state.word(),
            "until_unix": self.until,
            "games": self.games,
            "max_games": self.max_games,
            "available": self.state == ProfileState::Available,
        })
    }
}

impl Registry {
    /// Every profile, one row per id: available first, then the rest, each
    /// group by label (`docs/protocol.md` §"Which profiles a player sees").
    pub(crate) fn rows(&self) -> Vec<Row<'_>> {
        let mut by_id: BTreeMap<&str, Vec<(&str, &Reported)>> = BTreeMap::new();
        for (name, host) in &self.hosts {
            for profile in &host.profiles {
                by_id
                    .entry(profile.id.as_str())
                    .or_default()
                    .push((name.as_str(), profile));
            }
        }
        let mut rows: Vec<Row<'_>> = by_id
            .into_iter()
            .map(|(id, hosts)| {
                let state = hosts
                    .iter()
                    .map(|(_, p)| p.state)
                    .min()
                    .unwrap_or(ProfileState::Disabled);
                let until = hosts
                    .iter()
                    .filter(|(_, p)| p.state == state)
                    .filter_map(|(_, p)| p.until_unix)
                    .min();
                let games = hosts.iter().map(|(_, p)| p.games).sum();
                let max_games = hosts.iter().map(|(_, p)| p.max_games).sum::<Option<u32>>();
                Row {
                    id,
                    first: hosts[0].1,
                    state,
                    until,
                    games,
                    max_games,
                    hosts,
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            (a.state != ProfileState::Available)
                .cmp(&(b.state != ProfileState::Available))
                .then_with(|| a.first.label.cmp(&b.first.label))
                .then_with(|| a.id.cmp(b.id))
        });
        rows
    }

    /// The seat agent that should take an order for `profile`: of those
    /// that report it available, below its bound counting orders still
    /// starting, and within their capacity, the least busy (its games of
    /// the profile, then its bridges in all, then by name).
    pub(crate) fn pick(&self, profile: &str) -> Option<String> {
        self.hosts
            .iter()
            .filter_map(|(name, host)| {
                let p = host.profiles.iter().find(|p| p.id == profile)?;
                let games = p.games + host.starting_of(profile);
                let fits = p.state == ProfileState::Available
                    && p.max_games.is_none_or(|max| games < max)
                    && (host.capacity == 0 || host.live() < host.capacity);
                fits.then_some((games, host.live(), name))
            })
            .min()
            .map(|(_, _, name)| name.clone())
    }

    fn tell(&self, host: &str, frame: Frame) -> bool {
        self.hosts
            .get(host)
            .is_some_and(|h| h.tx.send(frame).is_ok())
    }
}

/// `BAYLEE_SEATHOST_TOKEN`, checked as the admin token is: 32 characters
/// at least, one word, none of the gateway's other secrets.
///
/// # Errors
/// A short token, one with whitespace, or one equal to another secret.
pub(crate) fn token_from_env(
    token: Option<&str>,
    others: &[Option<&str>],
) -> Result<Option<String>, String> {
    let Some(token) = token.filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if token.chars().count() < 32 {
        return Err("BAYLEE_SEATHOST_TOKEN: a token needs at least 32 characters".into());
    }
    if token.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("BAYLEE_SEATHOST_TOKEN: a token is one word of printable characters".into());
    }
    if others.iter().flatten().any(|other| *other == token) {
        return Err(
            "BAYLEE_SEATHOST_TOKEN: that token is already another secret of this gateway".into(),
        );
    }
    Ok(Some(token.to_string()))
}

// ------------------------------------------------------------------ socket

/// `GET /seathost/ws`: a seat agent offering hosted models.
pub(crate) async fn socket(
    State(state): State<Shared>,
    via_unix: Option<Extension<crate::engine::ViaUnix>>,
    ws: WebSocketUpgrade,
) -> Response {
    let local = via_unix.is_some();
    ws.max_message_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| run(state, socket, local))
}

async fn send(socket: &mut WebSocket, frame: &Frame) -> Result<(), axum::Error> {
    socket.send(Message::text(frame.to_text())).await
}

async fn run(state: Shared, mut socket: WebSocket, local: bool) {
    let hello = loop {
        match tokio::time::timeout(HELLO_TIMEOUT, socket.recv()).await {
            Ok(Some(Ok(Message::Text(text)))) => break Frame::parse(&text).ok(),
            Ok(Some(Ok(_))) => {}
            _ => return,
        }
    };
    let Some(Frame::Hello {
        token,
        name,
        protocol_version,
        capacity,
    }) = hello
    else {
        return;
    };
    let Some(expected) = state.seathost_token.as_deref() else {
        tracing::warn!("a seat agent connected but BAYLEE_SEATHOST_TOKEN is not set; refused");
        return;
    };
    if !auth::ct_eq(&auth::token_hash(&token), &auth::token_hash(expected)) {
        tracing::warn!("a seat agent presented the wrong token");
        return;
    }
    let refusal = baylee_protocol::version_refusal("This seat agent", protocol_version)
        .or_else(|| {
            (!valid_name(&name)).then(|| "a seat agent's name is 1-64 of A-Z a-z 0-9 . - _".into())
        })
        .or_else(|| {
            state
                .seathosts
                .lock()
                .hosts
                .contains_key(&name)
                .then(|| format!("a seat agent called «{name}» is connected already"))
        });
    if let Some(message) = refusal {
        tracing::warn!(name, message, "seat agent refused");
        let _ = send(&mut socket, &Frame::Refused { message }).await;
        return;
    }
    let (tx, mut rx) = mpsc::unbounded_channel();
    let conn = {
        let mut registry = state.seathosts.lock();
        registry.next_conn += 1;
        let conn = registry.next_conn;
        registry.hosts.insert(
            name.clone(),
            SeatHost {
                local,
                capacity,
                tx,
                profiles: Vec::new(),
                since: Instant::now(),
                waiting: HashMap::new(),
                starting: HashMap::new(),
                conn,
            },
        );
        conn
    };
    tracing::info!(name, local, capacity, "seat agent registered");
    let welcome = Frame::Welcome {
        heartbeat_secs: HEARTBEAT_SECS,
    };
    if send(&mut socket, &welcome).await.is_ok() {
        pump(&state, &mut socket, &mut rx, &name).await;
    }
    gone(&state, &name, conn);
}

/// One seat agent's conversation: orders out, reports in.
async fn pump(
    state: &Shared,
    socket: &mut WebSocket,
    rx: &mut mpsc::UnboundedReceiver<Frame>,
    name: &str,
) {
    let silence = Duration::from_secs(u64::from(HEARTBEAT_SECS) * 2);
    loop {
        tokio::select! {
            frame = rx.recv() => {
                let Some(frame) = frame else { return };
                if send(socket, &frame).await.is_err() {
                    return;
                }
            }
            message = tokio::time::timeout(silence, socket.recv()) => {
                let text = match message {
                    Err(_) => {
                        tracing::warn!(name, "a seat agent went quiet; dropped");
                        return;
                    }
                    Ok(Some(Ok(Message::Text(text)))) => text,
                    Ok(Some(Ok(Message::Close(_)) | Err(_)) | None) => return,
                    Ok(Some(Ok(_))) => continue,
                };
                match Frame::parse(&text) {
                    Ok(frame) => heard(state, name, frame),
                    Err(why) => tracing::debug!(name, why, "a seat agent frame not read"),
                }
            }
        }
    }
}

/// What a seat agent said.
fn heard(state: &Shared, name: &str, frame: Frame) {
    match frame {
        Frame::Profiles { profiles } => {
            if let Some(host) = state.seathosts.lock().hosts.get_mut(name) {
                host.profiles = profiles;
            }
        }
        Frame::Answer { request, ok, error } => {
            let waiting = state
                .seathosts
                .lock()
                .hosts
                .get_mut(name)
                .and_then(|h| h.waiting.remove(&request));
            if let Some(waiting) = waiting {
                let _ = waiting.send(if ok {
                    Ok(())
                } else {
                    Err(error.unwrap_or_else(|| "the seat agent refused".into()))
                });
            }
        }
        Frame::SeatStatus {
            order,
            kind,
            detail,
        } => {
            tracing::info!(name, order, ?kind, detail, "hosted seat status");
            if let Some(host) = state.seathosts.lock().hosts.get_mut(name) {
                host.starting.remove(&order);
            }
            if kind != SeatStatusKind::Started {
                let note = if detail.trim().is_empty() {
                    "the model's bridge ended".to_string()
                } else {
                    detail
                };
                let placed = state.seathosts.lock().orders.remove(&order);
                if let Some(placed) = placed {
                    give_up(state, &placed, &order, &note);
                }
            }
        }
        Frame::Heartbeat
        | Frame::Hello { .. }
        | Frame::Welcome { .. }
        | Frame::Refused { .. }
        | Frame::StartSeat { .. }
        | Frame::StopSeat { .. }
        | Frame::ProfileWrite { .. }
        | Frame::ProfileDelete { .. }
        | Frame::Control { .. }
        | Frame::KeyWrite { .. } => {}
    }
}

/// A seat agent's link ended: its profiles leave the offer, and every chair
/// still waiting for one of its bridges opens again.
fn gone(state: &Shared, name: &str, conn: u64) {
    let orphans: Vec<(String, Placed)> = {
        let mut registry = state.seathosts.lock();
        if registry.hosts.get(name).is_none_or(|h| h.conn != conn) {
            return;
        }
        registry.hosts.remove(name);
        let orphans: Vec<(String, Placed)> = registry
            .orders
            .iter()
            .filter(|(_, placed)| placed.host == name)
            .map(|(order, placed)| (order.clone(), placed.clone()))
            .collect();
        for (order, _) in &orphans {
            registry.orders.remove(order);
        }
        orphans
    };
    tracing::info!(name, "seat agent gone");
    for (order, placed) in orphans {
        give_up(state, &placed, &order, "the seat agent went away");
    }
    state.lobby_moved();
}

/// Opens a chair whose order failed, before its game, with `note`; a game
/// already playing keeps its chair (the house stands in).
fn give_up(state: &Shared, placed: &Placed, order: &str, note: &str) {
    let changed = {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(&placed.game_id) else {
            return;
        };
        if game.state != lobby::LobbyState::Waiting {
            return;
        }
        let Some(chair) = game.seats.get_mut(placed.seat) else {
            return;
        };
        let ours = chair
            .hosted
            .as_ref()
            .is_some_and(|h| h.order == order && h.standing());
        if ours {
            let mut hosted = chair.hosted.clone();
            chair.vacate();
            if let Some(hosted) = hosted.as_mut() {
                hosted.note = Some(note.chars().take(200).collect());
            }
            chair.hosted = hosted;
        }
        ours
    };
    revoke(state, order);
    if changed {
        tracing::info!(order, note, "a hosted chair opened again");
        state.lobby_moved();
    }
}

/// Drops the unspent chair ticket of `order`.
fn revoke(state: &Shared, order: &str) {
    state.chair_tickets.revoke(
        |grant| matches!(grant, wsticket::Grant::Chair { order: Some(o), .. } if o == order),
    );
}

/// Tells the seat agent running `order` to stop it, and forgets it.
fn stop(state: &Shared, order: &str) {
    let mut registry = state.seathosts.lock();
    if let Some(placed) = registry.orders.remove(order) {
        registry.tell(
            &placed.host,
            Frame::StopSeat {
                order: order.to_string(),
            },
        );
        if let Some(host) = registry.hosts.get_mut(&placed.host) {
            host.starting.remove(order);
        }
    }
}

/// The orders no chair stands for any more: its room is gone or over, or
/// its chair was rearranged, emptied or ordered again.
fn unseated(lobby: &lobby::Lobby, orders: &HashMap<String, Placed>) -> Vec<String> {
    let mut gone: Vec<String> = orders
        .iter()
        .filter(|(order, placed)| {
            let held = lobby.games.get(&placed.game_id).is_some_and(|game| {
                game.state != lobby::LobbyState::Over
                    && game.seats.get(placed.seat).is_some_and(|chair| {
                        chair
                            .hosted
                            .as_ref()
                            .is_some_and(|h| h.standing() && &h.order == *order)
                    })
            });
            !held
        })
        .map(|(order, _)| order.clone())
        .collect();
    gone.sort_unstable();
    gone
}

/// Sends `stop_seat` for every order whose chair was rearranged, taken
/// back or left with its host, and drops its unspent ticket. Called with
/// no lock held, after a change to a room's chairs.
pub(crate) fn stop_unseated(state: &Shared) {
    // One lock at a time, as everywhere here: the orders first, then the
    // lobby. An order placed in between is not in the copy, so not stopped.
    let orders = state.seathosts.lock().orders.clone();
    let gone = unseated(&state.lobby.lock(), &orders);
    for order in gone {
        tracing::info!(
            order,
            "a hosted chair was rearranged; its bridge is stopped"
        );
        stop(state, &order);
        revoke(state, &order);
    }
}

/// What a room's listing says of a chair's hosted model.
pub(crate) fn chair_json(chair: &lobby::LobbySeat) -> serde_json::Value {
    let Some(hosted) = &chair.hosted else {
        return serde_json::Value::Null;
    };
    let state = match (&hosted.note, &chair.delegate) {
        (Some(_), _) => "failed",
        (None, Some(delegate)) if delegate.ready => "ready",
        _ => "starting",
    };
    serde_json::json!({
        "profile": hosted.profile,
        "label": hosted.label,
        "vendor": hosted.vendor,
        "model": hosted.model,
        "state": state,
        "note": hosted.note,
    })
}

// ------------------------------------------------------------------ players

/// `GET /lobby/llm-profiles`: every hosted profile, available first.
pub(crate) async fn profiles(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    authed_session(&state, &headers).await?;
    let registry = state.seathosts.lock();
    let rows: Vec<serde_json::Value> = registry.rows().iter().map(Row::json).collect();
    Ok(Json(serde_json::json!({ "profiles": rows })))
}

/// What a host orders.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OrderBody {
    profile: String,
    #[serde(default)]
    deck_text: Option<String>,
}

/// `POST /lobby/games/{id}/chairs/{seat}/hosted`: the room's host seats a
/// hosted model.
pub(crate) async fn order(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
    Json(body): Json<OrderBody>,
) -> Result<Response, (StatusCode, Json<ErrorBody>)> {
    if state.seathost_token.is_none() || state.seathosts.lock().hosts.is_empty() {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "this gateway offers no hosted model now",
        ));
    }
    let session = authed_session(&state, &headers).await?;
    if session.guest {
        return Err(err(StatusCode::FORBIDDEN, GUEST_REFUSED));
    }
    if body
        .deck_text
        .as_ref()
        .is_some_and(|d| d.len() > MAX_DECK_BYTES)
    {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "that deck is too long"));
    }
    if !state
        .chair_limiter
        .allow(&format!("mint:{}", session.account_id))
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    // Which seat agent, and what the profile is: under the registry's lock,
    // released before the lobby's (the order is lobby, tickets, registry
    // nowhere; this takes them one at a time).
    let picked = match choose(&state.seathosts.lock(), &body.profile) {
        Ok(picked) => picked,
        Err(refused) => return Ok(*refused),
    };
    let order = picked.order.clone();
    let host_name = picked.host.clone();
    let ticket = seat_it(&state, &id, seat, &session.account_id, picked)?;
    let sent = {
        let mut registry = state.seathosts.lock();
        let sent = registry.tell(
            &host_name,
            Frame::StartSeat {
                order: order.clone(),
                game_id: id.clone(),
                seat: u32::try_from(seat).unwrap_or(u32::MAX),
                profile: body.profile.clone(),
                chair_ticket: ticket,
                gateway_url: state.seathost_bridge_url.clone(),
                deck_text: body.deck_text,
            },
        );
        if sent {
            registry.orders.insert(
                order.clone(),
                Placed {
                    host: host_name.clone(),
                    game_id: id.clone(),
                    seat,
                },
            );
            if let Some(host) = registry.hosts.get_mut(&host_name) {
                host.starting.insert(order.clone(), body.profile.clone());
            }
        }
        sent
    };
    if !sent {
        give_up(
            &state,
            &Placed {
                host: host_name,
                game_id: id,
                seat,
            },
            &order,
            "the seat agent went away",
        );
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "the seat agent went away; try again",
        ));
    }
    tracing::info!(
        order,
        game_id = id,
        seat,
        profile = body.profile,
        host = host_name,
        "hosted seat ordered"
    );
    state.lobby_moved();
    arm_ready_timer(&state, &order, &id, seat);
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "seat": seat,
            "profile": body.profile,
            "state": "starting",
        })),
    )
        .into_response())
}

/// The seat agent and profile an order for `profile` goes to, or the
/// `409` that says why none can take it.
fn choose(registry: &Registry, profile: &str) -> Result<lobby::HostedChair, Box<Response>> {
    let rows = registry.rows();
    let Some(row) = rows.iter().find(|row| row.id == profile) else {
        return Err(Box::new(unavailable("unknown", None)));
    };
    let Some(host) = registry.pick(profile) else {
        let word = if row.state == ProfileState::Available {
            ProfileState::Busy
        } else {
            row.state
        };
        return Err(Box::new(unavailable(word.word(), row.until)));
    };
    let reported = row
        .hosts
        .iter()
        .find(|(name, _)| *name == host)
        .map_or(row.first, |(_, p)| p);
    Ok(lobby::HostedChair {
        order: auth::new_id(),
        host,
        profile: profile.to_string(),
        label: reported.label.clone(),
        vendor: reported.vendor.clone(),
        model: reported.model.clone(),
        note: None,
    })
}

/// Marks chair `seat` of room `id` for `picked` and mints its ticket in
/// the host's name, under one hold of the lobby (as `chair::hand_over`
/// wants it); the chair is left as it was when either is refused.
fn seat_it(
    state: &Shared,
    id: &str,
    seat: usize,
    host: &str,
    picked: lobby::HostedChair,
) -> Result<String, (StatusCode, Json<ErrorBody>)> {
    let order = picked.order.clone();
    let mut lobby = state.lobby.lock();
    let game = lobby
        .games
        .get_mut(id)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
    if !game.hosted_by(host) {
        return Err(err(
            StatusCode::FORBIDDEN,
            "only the room's host seats a hosted model",
        ));
    }
    chair::open_chair(game, seat)?;
    game.seats[seat].hosted = Some(picked);
    let minted = chair::hand_over(
        &lobby,
        &state.chair_tickets,
        id,
        seat,
        host.to_string(),
        Some(order),
        Instant::now(),
    );
    if minted.is_err()
        && let Some(game) = lobby.games.get_mut(id)
    {
        game.seats[seat].hosted = None;
    }
    minted
}

/// Gives the order up when its chair is not ready within [`READY_WITHIN`].
fn arm_ready_timer(state: &Shared, order: &str, game_id: &str, seat: usize) {
    let timer = state.clone();
    let (order, game_id) = (order.to_string(), game_id.to_string());
    tokio::spawn(async move {
        tokio::time::sleep(READY_WITHIN).await;
        let late = {
            let lobby = timer.lobby.lock();
            lobby.games.get(&game_id).is_some_and(|game| {
                game.state == lobby::LobbyState::Waiting
                    && game.seats.get(seat).is_some_and(|chair| {
                        chair
                            .hosted
                            .as_ref()
                            .is_some_and(|h| h.order == order && h.standing())
                            && !chair.delegate.as_ref().is_some_and(|d| d.ready)
                    })
            })
        };
        if late {
            let placed = timer.seathosts.lock().orders.get(&order).cloned();
            stop(&timer, &order);
            if let Some(placed) = placed {
                give_up(&timer, &placed, &order, "the model did not answer in time");
            }
        }
    });
}

fn unavailable(state: &str, until: Option<i64>) -> Response {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "error": format!("that model is not available now ({state})"),
            "state": state,
            "until_unix": until,
        })),
    )
        .into_response()
}

/// `DELETE /lobby/games/{id}/chairs/{seat}/hosted`: the host takes a
/// hosted chair back before the game.
pub(crate) async fn cancel(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, seat)): Path<(String, usize)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let session = authed_session(&state, &headers).await?;
    let order = {
        let mut lobby = state.lobby.lock();
        let game = lobby
            .games
            .get_mut(&id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such game"))?;
        if !game.hosted_by(&session.account_id) {
            return Err(err(StatusCode::FORBIDDEN, "only the room's host may"));
        }
        if game.state != lobby::LobbyState::Waiting {
            return Err(err(StatusCode::CONFLICT, "game already started"));
        }
        let chair = game
            .seats
            .get_mut(seat)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such seat"))?;
        let Some(hosted) = chair.hosted.take() else {
            return Err(err(StatusCode::NOT_FOUND, "no hosted model at that chair"));
        };
        if hosted.standing() {
            chair.vacate();
        }
        hosted.order
    };
    stop(&state, &order);
    revoke(&state, &order);
    state.lobby_moved();
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------------ console

/// `GET /admin/llm/seathosts`.
pub(crate) fn seathosts_json(state: &crate::AppState) -> serde_json::Value {
    let registry = state.seathosts.lock();
    let hosts: Vec<serde_json::Value> = registry
        .hosts
        .iter()
        .map(|(name, host)| {
            serde_json::json!({
                "name": name,
                "local": host.local,
                "capacity": host.capacity,
                "games": host.live(),
                "connected_secs": host.since.elapsed().as_secs(),
                "profiles": host.profiles,
            })
        })
        .collect();
    serde_json::json!({ "seathosts": hosts })
}

/// `GET /admin/llm/profiles`.
pub(crate) fn profiles_json(state: &crate::AppState) -> serde_json::Value {
    let registry = state.seathosts.lock();
    let rows: Vec<serde_json::Value> = registry
        .rows()
        .iter()
        .map(|row| {
            let mut json = row.json();
            json["hosts"] = row
                .hosts
                .iter()
                .map(|(host, p)| {
                    serde_json::json!({
                        "host": host,
                        "state": p.state.word(),
                        "until_unix": p.until_unix,
                        "games": p.games,
                        "max_games": p.max_games,
                        "enabled": p.enabled,
                        "canary": p.canary,
                        "caps": p.caps,
                        "spent": p.spent,
                        "key": p.key,
                        "last_error": p.last_error,
                        "last_ok_unix": p.last_ok_unix,
                    })
                })
                .collect();
            json
        })
        .collect();
    serde_json::json!({ "profiles": rows })
}

/// What the console asks of a seat agent.
pub(crate) enum Ask {
    /// Write a definition.
    Write(Definition),
    /// Forget a profile.
    Delete,
    /// Probe, enable or disable.
    Control(ControlAction),
    /// Keep (`Some`) or forget a key. Never logged, never kept.
    Key(Option<String>),
}

/// Sends `ask` for `host`'s profile `id` and waits for its answer.
///
/// # Errors
/// `404` no such seat agent, `503` a key to one not on the unix socket,
/// `504` no answer in time, `400` its refusal.
pub(crate) async fn ask(
    state: &crate::AppState,
    host: &str,
    id: &str,
    admin: &str,
    ask: Ask,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if !valid_name(host) || !valid_name(id) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a seat agent's name and a profile's id are 1-64 of A-Z a-z 0-9 . - _",
        ));
    }
    let request = auth::new_id();
    let waiting_for = request.clone();
    let (tx, rx) = oneshot::channel();
    {
        let mut registry = state.seathosts.lock();
        let seathost = registry
            .hosts
            .get_mut(host)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such seat agent connected"))?;
        if matches!(ask, Ask::Key(_)) && !seathost.local {
            return Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                "keys travel only over the unix socket",
            ));
        }
        let (asked, id, admin) = (request.clone(), id.to_string(), admin.to_string());
        let request = asked;
        let frame = match ask {
            Ask::Write(definition) => Frame::ProfileWrite {
                request,
                id,
                definition,
                admin,
            },
            Ask::Delete => Frame::ProfileDelete { request, id, admin },
            Ask::Control(action) => Frame::Control {
                request,
                id,
                action,
                admin,
            },
            Ask::Key(key) => Frame::KeyWrite {
                request,
                id,
                key,
                admin,
            },
        };
        if seathost.tx.send(frame).is_err() {
            return Err(err(StatusCode::NOT_FOUND, "no such seat agent connected"));
        }
        seathost.waiting.insert(waiting_for.clone(), tx);
    }
    let answered = tokio::time::timeout(ANSWER_WITHIN, rx).await;
    if let Some(host) = state.seathosts.lock().hosts.get_mut(host) {
        host.waiting.remove(&waiting_for);
    }
    match answered {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(why))) => Err(err_saying(StatusCode::BAD_REQUEST, why)),
        _ => Err(err(
            StatusCode::GATEWAY_TIMEOUT,
            "the seat agent did not answer",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_protocol::seathost::{KeyKept, Spent};

    fn reported(id: &str, state: ProfileState, games: u32, max: Option<u32>) -> Reported {
        Reported {
            id: id.into(),
            label: id.to_uppercase(),
            vendor: "V".into(),
            kind: "api".into(),
            model: "m".into(),
            state,
            until_unix: None,
            games,
            max_games: max,
            enabled: true,
            canary: false,
            caps: None,
            spent: Spent::default(),
            key: KeyKept::Kept,
            last_error: None,
            last_ok_unix: None,
            definition: Definition {
                label: id.into(),
                vendor: "V".into(),
                enabled: true,
                max_games: max,
                caps: None,
                canary: false,
                profile: serde_json::json!({}),
            },
        }
    }

    fn host(profiles: Vec<Reported>, capacity: u32) -> SeatHost {
        SeatHost {
            local: true,
            capacity,
            tx: mpsc::unbounded_channel().0,
            profiles,
            since: Instant::now(),
            waiting: HashMap::new(),
            starting: HashMap::new(),
            conn: 0,
        }
    }

    /// A hosted chair rearranged, taken back or left with its room is an
    /// order no chair stands for: its bridge is told `stop_seat`. A chair
    /// still holding its order, waiting or playing, keeps it.
    #[test]
    fn an_order_whose_chair_was_rearranged_is_stopped() {
        let deck = crate::store::Deck {
            id: "deck".into(),
            account_id: "h".into(),
            kind: "account".into(),
            name: "Deck".into(),
            format: "freeform".into(),
            description: None,
            origin: None,
            version: 1,
            cards: vec!["60 Forest".into()],
            sideboard: vec![],
            commanders: vec![],
            sleeve: None,
            playmat: None,
            updated_at: 0,
            offered: true,
        };
        let hosted = |order: &str| lobby::HostedChair {
            order: order.into(),
            host: "a".into(),
            profile: "sonnet".into(),
            label: "Sonnet".into(),
            vendor: "V".into(),
            model: "m".into(),
            note: None,
        };
        let mut lobby = lobby::Lobby::default();
        for id in ["g", "over"] {
            let mut game = lobby::LobbyGame::room(
                id.into(),
                "h".into(),
                "Deck".into(),
                deck.clone(),
                4,
                id.into(),
                0,
            );
            game.seats[1].hosted = Some(hosted(&format!("{id}-kept")));
            game.seats[2].hosted = Some(hosted(&format!("{id}-moved")));
            game.seats[3].hosted = Some(hosted(&format!("{id}-failed")));
            lobby.games.insert(id.into(), game);
        }
        let mut orders = HashMap::new();
        for id in ["g", "over"] {
            for (order, seat) in [("kept", 1), ("moved", 2), ("failed", 3)] {
                orders.insert(
                    format!("{id}-{order}"),
                    Placed {
                        host: "a".into(),
                        game_id: id.into(),
                        seat,
                    },
                );
            }
        }
        orders.insert(
            "no-room".into(),
            Placed {
                host: "a".into(),
                game_id: "gone".into(),
                seat: 1,
            },
        );
        // Every chair holds its order; only the room that is gone is not.
        assert_eq!(unseated(&lobby, &orders), ["no-room"]);
        let game = lobby.games.get_mut("g").expect("room");
        game.seats[2].vacate();
        if let Some(h) = game.seats[3].hosted.as_mut() {
            h.note = Some("failed".into());
        }
        lobby.games.get_mut("over").expect("room").state = lobby::LobbyState::Over;
        assert_eq!(
            unseated(&lobby, &orders),
            [
                "g-failed",
                "g-moved",
                "no-room",
                "over-failed",
                "over-kept",
                "over-moved"
            ]
        );
        lobby.games.get_mut("g").expect("room").state = lobby::LobbyState::Playing;
        assert!(!unseated(&lobby, &orders).contains(&"g-kept".to_string()));
    }

    /// One profile on two seat agents is one row, its games summed; the
    /// least busy seat agent with room takes the next order, an order still
    /// starting counting as a game; an exhausted profile is listed last.
    #[test]
    fn profiles_are_one_row_per_id_and_the_least_busy_takes_the_order() {
        let mut registry = Registry::default();
        registry.hosts.insert(
            "a".into(),
            host(
                vec![
                    reported("sonnet", ProfileState::Available, 1, Some(2)),
                    reported("opus", ProfileState::Exhausted, 0, Some(1)),
                ],
                0,
            ),
        );
        registry.hosts.insert(
            "b".into(),
            host(
                vec![reported("sonnet", ProfileState::Available, 0, Some(2))],
                0,
            ),
        );
        let rows = registry.rows();
        let ids: Vec<&str> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, ["sonnet", "opus"], "available first");
        assert_eq!((rows[0].games, rows[0].max_games), (1, Some(4)));
        assert_eq!(rows[1].json()["available"], false);
        assert_eq!(registry.pick("sonnet").as_deref(), Some("b"));
        registry
            .hosts
            .get_mut("b")
            .unwrap()
            .starting
            .insert("o1".into(), "sonnet".into());
        assert_eq!(
            registry.pick("sonnet").as_deref(),
            Some("a"),
            "a tie on games breaks on the name"
        );
        registry
            .hosts
            .get_mut("a")
            .unwrap()
            .starting
            .insert("o2".into(), "sonnet".into());
        registry
            .hosts
            .get_mut("b")
            .unwrap()
            .starting
            .insert("o3".into(), "sonnet".into());
        assert_eq!(registry.pick("sonnet"), None, "both at their bound");
        assert_eq!(registry.pick("opus"), None, "exhausted is never picked");
        assert_eq!(registry.pick("nothing"), None);
    }

    #[test]
    fn a_seat_agent_at_capacity_takes_nothing() {
        let mut registry = Registry::default();
        registry.hosts.insert(
            "a".into(),
            host(vec![reported("p", ProfileState::Available, 1, None)], 1),
        );
        assert_eq!(registry.pick("p"), None);
    }

    #[test]
    fn the_seathost_token_is_its_own_secret() {
        assert_eq!(token_from_env(None, &[]), Ok(None));
        assert!(token_from_env(Some("short"), &[]).is_err());
        let token = "a-seathost-token-of-thirty-two-chars-x";
        assert!(token_from_env(Some(token), &[Some(token)]).is_err());
        assert_eq!(token_from_env(Some(token), &[None]), Ok(Some(token.into())));
    }
}
