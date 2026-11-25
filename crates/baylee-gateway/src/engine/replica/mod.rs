//! Protected engine links have a separate vocabulary. An ordinary `GameEnded`,
//! spectator stream or a player's raw socket input cannot change a rated result.
use super::{
    Envelope, Incoming, Shared, WebSocket, auth, execution, mpsc, next_envelope, send, v1,
};
use crate::ranked::replicate::{self, Run};
use baylee_protocol::v1::envelope::Msg;
use central_api::ranked::replica::{Head, Role};
use prost::Message as _;
use std::collections::VecDeque;

mod journal;
mod projections;

pub(super) async fn run(state: &Shared, mut socket: WebSocket, hello: &v1::EngineHello) {
    let Some(run) = replicate::by_job(state, &hello.game_id) else {
        return;
    };
    let (tx, local) = mpsc::unbounded_channel();
    {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(&run.setup.game_id) else {
            return;
        };
        if game.state != crate::LobbyState::Playing
            || game.engine_job.as_deref() != Some(hello.game_id.as_str())
            || game.engine.is_some()
            || !game
                .engine_token_hash
                .as_ref()
                .is_some_and(|hash| auth::ct_eq(hash, &auth::token_hash(&hello.token)))
            || baylee_protocol::version_refusal("ranked engine", hello.protocol_version).is_some()
            || !execution::matches(game.execution.as_ref(), hello)
            || game
                .ranked
                .and_then(|b| b.replica)
                .is_none_or(|r| r.generation() != run.work.fence.generation)
        {
            return;
        }
        game.engine = Some(tx);
        game.engine_pid = Some(hello.pid).filter(|pid| *pid != 0 && game.engine_local);
    }
    let (orders, send_order) = mpsc::channel(2);
    let (votes, mut vote) = mpsc::channel(1);
    let (snapshot, mut snapshots) = tokio::sync::watch::channel([0_u64; 2]);
    let channels = Channels {
        local,
        orders: send_order,
        votes,
        snapshots: snapshot,
    };
    let presence_changed = tokio::sync::Notify::new();
    let Some(bridge) = state.central.get() else {
        return;
    };
    // No journal or lease task outlives the authenticated engine socket. A stale engine's
    // cleanup cannot touch the table its successor has since reconstructed.
    tokio::select! {
        () = journal::drive(state, &bridge, &run, orders, &mut vote, &mut snapshots) => {}
        () = route(state, &run, &mut socket, channels, &presence_changed) => {}
        () = presence(state, &bridge, &run, &presence_changed), if run.work.role == Role::Primary => {}
        () = replicate::lease(state, &bridge, &run) => {}
    }
    run.dead.store(true, std::sync::atomic::Ordering::Release);
    if run.is_current(state) {
        run.fence(state);
    }
}

struct Channels {
    local: mpsc::UnboundedReceiver<Envelope>,
    orders: mpsc::Receiver<Msg>,
    votes: mpsc::Sender<v1::RankedEngineVote>,
    snapshots: tokio::sync::watch::Sender<[u64; 2]>,
}

async fn route(
    state: &Shared,
    run: &Run,
    socket: &mut WebSocket,
    mut channels: Channels,
    presence_changed: &tokio::sync::Notify,
) {
    let mut published = projections::Releases::default();
    loop {
        if !run.is_current(state) {
            return;
        }
        tokio::select! {
            order = channels.orders.recv() => {
                let Some(order) = order else { return };
                published.order(&order);
                if send(socket, &Envelope { msg: Some(order) }).await.is_err() {
                    return;
                }
            }
            local = channels.local.recv() => {
                let Some(frame) = local else { return };
                if matches!(frame.msg, Some(Msg::SeatAttached(_) | Msg::SeatDetached(_))) {
                    presence_changed.notify_one();
                }
                if let Some(seat) = snapshot_seat(frame) && seat < 2 {
                    channels.snapshots.send_modify(|seats| {
                        let counter = &mut seats[seat as usize];
                        *counter = counter.wrapping_add(1);
                    });
                }
            }
            incoming = socket.recv() => match next_envelope(incoming) {
                Incoming::Msg(Msg::RankedEngineVote(vote)) => {
                    if channels.votes.try_send(vote).is_err() { return; }
                }
                Incoming::Msg(Msg::SeatFrame(frame)) => forward(state, run, &published, frame),
                Incoming::Closed => return,
                _ => {}
            }
        }
    }
}

async fn presence(
    state: &Shared,
    bridge: &crate::central::Bridge,
    run: &Run,
    changed: &tokio::sync::Notify,
) {
    use central_api::ranked::replica::HostRequest;
    while run.is_current(state) {
        if run.deadline().is_some() && run.engine_head.lock().is_some() {
            let attached = state
                .lobby
                .lock()
                .games
                .get(&run.setup.game_id)
                .map_or(0, |game| {
                    game.seats
                        .iter()
                        .take(2)
                        .enumerate()
                        .fold(0, |mask, (i, seat)| {
                            mask | (u8::from(seat.socket_owner.is_some()) << i)
                        })
                });
            let _ = replicate::rpc(
                bridge,
                HostRequest::Presence {
                    fence: run.work.fence,
                    attached,
                },
            )
            .await;
        }
        tokio::select! {
            () = tokio::time::sleep(std::time::Duration::from_secs(4)) => {}
            () = changed.notified() => {}
        }
    }
}

fn snapshot_seat(frame: Envelope) -> Option<u32> {
    match frame.msg? {
        Msg::SeatAttached(attached) => Some(attached.seat),
        Msg::SeatFrame(frame)
            if matches!(
                Envelope::decode(frame.envelope.clone()).ok()?.msg,
                Some(Msg::SeatReady(_) | Msg::Resume(_))
            ) =>
        {
            Some(frame.seat)
        }
        _ => None,
    }
}

fn forward(state: &Shared, run: &Run, published: &projections::Releases, frame: v1::SeatFrame) {
    if frame.seat >= 2 || run.work.role != Role::Primary || run.deadline().is_none() {
        return;
    }
    let Some(Msg::RankedSeatProjection(view)) = Envelope::decode(frame.envelope.clone())
        .ok()
        .and_then(|e| e.msg)
    else {
        return;
    };
    if view.generation != run.work.fence.generation
        || !published.permits(frame.seat, view.position, &view.chain)
    {
        return;
    }
    let mut lobby = state.lobby.lock();
    let Some(game) = lobby.games.get_mut(&run.setup.game_id) else {
        return;
    };
    if game.engine_job.as_deref() == Some(run.job.as_str())
        && game.ranked.is_some_and(|b| {
            b.replica.is_some_and(|r| r.generation() == view.generation) && b.seats_live()
        })
    {
        let _ = game.outbox(frame.seat as usize).send(frame.envelope);
    }
}
