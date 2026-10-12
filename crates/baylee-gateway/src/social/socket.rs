//! `GET /social/ws` (§8.1): one socket per device, JSON text frames.
//!
//! Opened with a ticket for `{"socket":"social"}`, stating the protocol and
//! the social version. The client's first frame is `hello`, naming one of
//! its account's devices; nothing else is accepted before it. A frame over
//! [`wire::MAX_SOCIAL_FRAME`] or past the socket's allowance (40 at once,
//! then 10 a second) closes it with 1008. No frame, ticket or tag reaches a
//! log line.

use super::route::{self, Sending, Window};
use super::{OUT_QUEUE, Out, millis, person, presence, uid};
use crate::{
    ErrorBody, Json, Message, Query, RecvError, Shared, State, StatusCode, TICKET_REFUSED,
    WebSocket, WebSocketUpgrade, auth, db_down, err, spend_ticket, store, wsticket,
};
use axum::extract::ws::{CloseFrame, close_code};
use baylee_protocol::social::{
    self as wire, Down, Friend, GroupInfo, GroupInvite, Member, Person, Snapshot, SocialSettings,
    Up, WhispersFrom, Why,
};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// What `/social/ws` is opened with. Never logged; no `Debug`.
#[derive(Deserialize)]
pub(crate) struct SocialParams {
    #[serde(default)]
    ticket: Option<String>,
    #[serde(default)]
    protocol: u32,
    #[serde(default)]
    social: u32,
}

/// How often a quiet socket is pinged, and how long it may go without a
/// word: as the lobby feed.
const PING_EVERY: Duration = Duration::from_secs(20);
const QUIET: Duration = Duration::from_secs(50);

/// How long a socket may take to say `hello`.
const HELLO_WITHIN: Duration = Duration::from_secs(30);

/// The socket's frame allowance: 40 at once, then 10 a second.
const FRAME_RATE: u32 = 10;
const FRAME_BURST: u32 = 40;

/// What the transport reads at most: a frame between this and
/// [`wire::MAX_SOCIAL_FRAME`] is read and refused with 1008, anything
/// larger ends the connection in the transport.
const TRANSPORT_LIMIT: usize = 64 * 1024;

pub(crate) async fn social_ws(
    State(state): State<Shared>,
    Query(params): Query<SocialParams>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorBody>)> {
    if !state.social.on() {
        return Err(err(StatusCode::FORBIDDEN, "this gateway has no chat"));
    }
    let ticket = params
        .ticket
        .as_deref()
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing ticket"))?;
    let wsticket::Grant::Social {
        account_id,
        session,
    } = spend_ticket(&state, ticket, &wsticket::Door::Social)?
    else {
        return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED));
    };
    match store::resolve_digest(&state.db, session, auth::now_secs())
        .await
        .map_err(|e| db_down(&e))?
    {
        Some(session) if session.account_id == account_id => {}
        _ => return Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED)),
    }
    let why = baylee_protocol::version_refusal("This client", params.protocol)
        .or_else(|| wire::social_version_refusal("This client", params.social));
    if let Some(why) = why {
        return Ok(ws.on_upgrade(move |socket| refuse(socket, why)));
    }
    Ok(ws
        .max_message_size(TRANSPORT_LIMIT)
        .max_frame_size(TRANSPORT_LIMIT)
        .on_upgrade(move |socket| run(state, account_id, socket)))
}

/// The one frame a socket of another version is sent before it closes.
async fn refuse(mut socket: WebSocket, why: String) {
    let frame = Down::Refused {
        req: None,
        why: Why::Protocol,
        missing: Vec::new(),
        extra: Vec::new(),
        message: Some(why),
    };
    if send(&mut socket, &frame).await {
        let _ = socket.send(Message::Close(None)).await;
    }
}

/// Sends one frame; whether it went.
async fn send(socket: &mut WebSocket, frame: &Down) -> bool {
    let Ok(text) = serde_json::to_string(frame) else {
        return false;
    };
    socket.send(Message::Text(text.into())).await.is_ok()
}

/// Closes with `code` and `reason`.
async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

/// Waits for the client's `hello`.
async fn hello(socket: &mut WebSocket) -> Option<(String, wire::PresenceSet)> {
    let deadline = tokio::time::sleep(HELLO_WITHIN);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            () = &mut deadline => return None,
            frame = socket.recv() => match frame {
                Some(Ok(Message::Text(text))) => {
                    if text.len() > wire::MAX_SOCIAL_FRAME {
                        return None;
                    }
                    return match serde_json::from_str::<Up>(&text) {
                        Ok(Up::Hello { device, presence }) => Some((device, presence)),
                        _ => None,
                    };
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                _ => return None,
            },
        }
    }
}

/// The whole social state of `account` (whose devices see `me`).
///
/// # Errors
///
/// When the database fails.
pub(crate) async fn snapshot(state: &Shared, account: &str) -> anyhow::Result<Snapshot> {
    let Some(id) = uid(account) else {
        return Ok(Snapshot::default());
    };
    let db = &state.db;
    let friends = baylee_db::social::friends(db, id).await?;
    let friends = friends
        .iter()
        .map(|(who, since)| Friend {
            person: person(who),
            since: millis(*since),
            presence: presence::presence_now(state, &who.id.to_string()),
        })
        .collect();
    let people = |list: Vec<baylee_db::social::Who>| list.iter().map(person).collect::<Vec<_>>();
    let incoming = people(baylee_db::social::incoming(db, id).await?);
    let outgoing = people(baylee_db::social::outgoing(db, id).await?);
    let blocked = people(baylee_db::social::blocked(db, id).await?);
    let group_ids = baylee_db::social::group_ids_of(db, id).await?;
    let groups = baylee_db::social::groups(db, &group_ids)
        .await?
        .iter()
        .map(group_info)
        .collect();
    let group_invites = baylee_db::social::group_invites_of(db, id)
        .await?
        .iter()
        .map(|invite| GroupInvite {
            group: invite.group.to_string(),
            name: invite.name.clone(),
            inviter: person(&invite.inviter),
            at: millis(invite.at),
        })
        .collect();
    let settings = baylee_db::social::settings_of(db, &[id])
        .await?
        .first()
        .map(|(_, s)| SocialSettings {
            whispers_from: WhispersFrom::from_code(s.whispers_from),
            requests_open: s.requests_open,
            share_table: s.share_table,
        })
        .unwrap_or_default();
    Ok(Snapshot {
        friends,
        incoming,
        outgoing,
        blocked,
        groups,
        group_invites,
        tables: presence::tables_of(state, account).await,
        invites: state
            .social
            .invites
            .lock()
            .open_for(account, Instant::now()),
        settings,
    })
}

/// A group as the wire carries it.
pub(crate) fn group_info(group: &baylee_db::social::Group) -> GroupInfo {
    GroupInfo {
        id: group.id.to_string(),
        name: group.name.clone(),
        members: group
            .members
            .iter()
            .map(|(who, owner)| Member {
                person: person(who),
                owner: *owner,
            })
            .collect(),
        invited: group.invited.iter().map(person).collect(),
    }
}

/// One device's socket, from `hello` until it goes.
async fn run(state: Shared, account: String, mut socket: WebSocket) {
    let mut departed = state.departed.subscribe();
    let Ok(Some(row)) = store::account(&state.db, &account).await else {
        let _ = socket.send(Message::Close(None)).await;
        return;
    };
    let Some((device, status)) = hello(&mut socket).await else {
        close(&mut socket, close_code::POLICY, "hello first").await;
        return;
    };
    let mine = match baylee_db::social::device_by_id(&state.db, &device).await {
        Ok(found) => found.is_some_and(|d| Some(d.account_id) == uid(&account)),
        Err(e) => {
            tracing::error!("social: {e}");
            close(&mut socket, close_code::AGAIN, "database unavailable").await;
            return;
        }
    };
    if !mine {
        let frame = Down::Refused {
            req: None,
            why: Why::Device,
            missing: Vec::new(),
            extra: Vec::new(),
            message: None,
        };
        if send(&mut socket, &frame).await {
            let _ = socket.send(Message::Close(None)).await;
        }
        return;
    }
    let me = Person {
        tag: crate::handle::tag_text(row.tag),
        handle: crate::handle::handle(&row.display_name, row.tag),
    };
    let guest = row.guest;
    let Some(id) = uid(&account) else {
        return;
    };
    let share_table = baylee_db::social::settings_of(&state.db, &[id])
        .await
        .ok()
        .and_then(|s| s.first().map(|(_, s)| s.share_table))
        .unwrap_or(true);
    let friends = super::friends_of(&state, id).await.unwrap_or_default();
    // Counted present for as long as the socket is open (`GET
    // /lobby/stats`, the room grace), as a lobby socket is.
    let present = state.presence.enter(&account);
    let (tx, mut rx) = mpsc::channel::<Out>(OUT_QUEUE);
    let (conn, _) = state.social.attach(
        &account,
        &device,
        me.clone(),
        &status,
        share_table,
        friends,
        tx,
    );
    if let Err(e) = baylee_db::social::touch_device(&state.db, &device).await {
        tracing::warn!("social: {e}");
    }
    serve(
        &state,
        &account,
        &device,
        &me,
        guest,
        &mut socket,
        &mut rx,
        &mut departed,
    )
    .await;
    if let Some((me, friends)) = state.social.detach(&account, &device, conn) {
        presence::went_offline(&state.social, &me, &friends, &account);
    }
    drop(present);
    crate::departure::socket_closed(&state, &account);
}

/// The socket's life after `hello`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn serve(
    state: &Shared,
    account: &str,
    device: &str,
    me: &Person,
    guest: bool,
    socket: &mut WebSocket,
    rx: &mut mpsc::Receiver<Out>,
    departed: &mut tokio::sync::broadcast::Receiver<String>,
) {
    let welcome = Down::Welcome {
        tag: me.tag.clone(),
        handle: me.handle.clone(),
        device: device.to_owned(),
        guest,
    };
    if !send(socket, &welcome).await {
        return;
    }
    match snapshot(state, account).await {
        Ok(snap) => {
            if !send(socket, &Down::Snapshot(snap)).await {
                return;
            }
        }
        Err(e) => {
            tracing::error!("social: no snapshot: {e:#}");
            close(socket, close_code::AGAIN, "database unavailable").await;
            return;
        }
    }
    let mut window = Window::default();
    if !refill(state, device, &mut window, socket).await {
        return;
    }
    // This account's presence and tables, now that it is here.
    presence::publish(state).await;
    let opened = Instant::now();
    let mut allowance = crate::seatrate::Allowance::new(opened, FRAME_RATE, FRAME_BURST);
    let mut ping = tokio::time::interval(PING_EVERY);
    ping.tick().await;
    let mut heard = tokio::time::Instant::now();
    loop {
        tokio::select! {
            frame = socket.recv() => {
                let Some(Ok(frame)) = frame else { return };
                heard = tokio::time::Instant::now();
                let text = match frame {
                    Message::Text(text) => text,
                    Message::Close(_) => return,
                    Message::Ping(_) | Message::Pong(_) => continue,
                    Message::Binary(_) => {
                        close(socket, close_code::UNSUPPORTED, "text frames only").await;
                        return;
                    }
                };
                if !allowance.admit(Instant::now()) {
                    tracing::debug!("social: a socket over its frame allowance was closed");
                    close(socket, close_code::POLICY, "too many frames").await;
                    return;
                }
                if text.len() > wire::MAX_SOCIAL_FRAME {
                    close(socket, close_code::POLICY, "frame too large").await;
                    return;
                }
                let Ok(up) = serde_json::from_str::<Up>(&text) else {
                    close(socket, close_code::POLICY, "not a frame").await;
                    return;
                };
                match up {
                    Up::Hello { .. } => {}
                    Up::Send { req, conv, sealed } => {
                        let sending = Sending {
                            account,
                            tag: &me.tag,
                            handle: &me.handle,
                            device,
                            guest,
                        };
                        let answer = route::send(state, &sending, req, &conv, sealed).await;
                        if !send(socket, &answer).await {
                            return;
                        }
                    }
                    Up::Ack { upto } => {
                        let acked = window.acked(upto);
                        if let Err(e) = baylee_db::social::ack(&state.db, device, &acked).await {
                            tracing::warn!("social: an ack was not stored: {e}");
                        }
                        if window.behind && !refill(state, device, &mut window, socket).await {
                            return;
                        }
                    }
                    Up::Presence(set) => {
                        state.social.set_status(account, &set);
                        presence::publish(state).await;
                    }
                }
            }
            out = rx.recv() => match out {
                None => return,
                Some(Out::Frame(frame)) => {
                    if !send(socket, &frame).await {
                        return;
                    }
                }
                Some(Out::Deliver { seq, frame }) => {
                    if window.take(seq) && !send(socket, &frame).await {
                        return;
                    }
                }
                Some(Out::Close(code, reason)) => {
                    close(socket, code, reason).await;
                    return;
                }
            },
            gone = departed.recv() => match gone {
                Ok(id) if id != account => {}
                Ok(_) | Err(RecvError::Lagged(_) | RecvError::Closed) => {
                    close(socket, close_code::NORMAL, "account deleted").await;
                    return;
                }
            },
            _ = ping.tick() => {
                if heard.elapsed() > QUIET
                    || socket.send(Message::Ping(Vec::new().into())).await.is_err()
                {
                    return;
                }
            }
        }
    }
}

/// Sends what the database holds for `device` that the window has room
/// for; whether the socket is still there.
async fn refill(state: &Shared, device: &str, window: &mut Window, socket: &mut WebSocket) -> bool {
    match route::pull(&state.db, device, window).await {
        Ok(frames) => {
            for frame in frames {
                if !send(socket, &frame).await {
                    return false;
                }
            }
            true
        }
        Err(e) => {
            tracing::warn!("social: deliveries not read: {e:#}");
            true
        }
    }
}
