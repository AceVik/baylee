//! Discover fenced assignments, restore the opening and request an exact engine.
use super::{
    Arc, Duration, Failure, Head, Instant, JOB_PREFIX, LEASE, Mutex, Phase, Placement, Role, Run,
    Shared, StatusCode, Uuid, Work, err, identity, invalid, reconstruct, replica, replica_table,
    rpc, v1,
};
use futures_util::{StreamExt as _, stream};
use replica::{HostRequest as Request, HostResponse as Response};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use tokio::time::{MissedTickBehavior, interval};

pub(in crate::ranked) async fn poll(state: &Shared) {
    let mut beat = interval(Duration::from_secs(2));
    let mut last_reap: Option<Instant> = None;
    beat.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        beat.tick().await;
        let compatible = state.agents.lock().ranked_replica_capacity().is_some();
        if !compatible && state.ranked.replicas.lock().is_empty() {
            continue;
        }
        let Some(bridge) = state.central.get() else {
            return;
        };
        let Ok(Response::Work { items, pending }) = rpc(&bridge, Request::Work).await else {
            continue;
        };
        if items.len() + pending.len() < 64
            && last_reap.is_none_or(|at| at.elapsed() >= Duration::from_secs(60))
        {
            let assigned: Vec<_> = items
                .iter()
                .map(|w| w.fence.match_id)
                .chain(pending.iter().map(|w| w.fence.match_id))
                .map(Uuid::from_bytes)
                .collect();
            replica_table::reconcile_absent(state, &assigned).await;
            last_reap = Some(Instant::now());
        }
        let reservations = pending.into_iter().map(|work| work.fence).chain(
            items
                .iter()
                .filter(|work| work.game.is_none())
                .map(|work| work.fence),
        );
        stream::iter(reservations.take(64))
            .for_each_concurrent(8, |fence| {
                let bridge = &bridge;
                async move {
                    let _ = rpc(bridge, Request::Pulse { fence }).await;
                }
            })
            .await;
        let assigned: HashSet<Uuid> = items
            .iter()
            .filter_map(|w| w.game.map(Uuid::from_bytes))
            .collect();
        let missing: Vec<_> = state
            .ranked
            .replicas
            .lock()
            .iter()
            .filter(|(game, r)| !assigned.contains(game) && r.status.lock().0.elapsed() >= LEASE)
            .map(|(_, r)| Arc::clone(r))
            .collect();
        for run in missing {
            retire(state, &run).await;
        }
        stream::iter(items.into_iter().filter(|w| w.game.is_some()).take(64))
            .for_each_concurrent(8, |work| {
                let bridge = &bridge;
                async move {
                    if let Err((code, _)) = sync(state, bridge, work).await {
                        tracing::debug!(%code, "ranked replica awaiting compatible resources");
                    }
                }
            })
            .await;
    }
}

/// Executed only by the single advisory-lock owner/inbox driver.
async fn retire(state: &Shared, run: &Arc<Run>) {
    {
        let mut registry = state.ranked.replicas.lock();
        if !run.registered(&registry) {
            return;
        }
        replica_table::fence(state, run.placement.game, run.placement.generation);
        registry.remove(&run.placement.game);
    }
    crate::engine::end_game(state, &run.setup.game_id);
    if run.terminal.load(std::sync::atomic::Ordering::Acquire) {
        let _ = replica_table::finish(state, &run.placement).await;
    } else {
        replica_table::withdraw(state, &run.placement);
    }
}

async fn sync(state: &Shared, bridge: &crate::central::Bridge, work: Work) -> Result<(), Failure> {
    let game = Uuid::from_bytes(work.game.ok_or_else(invalid)?);
    if work.game != Some(work.fence.match_id) || work.fence.generation == 0 {
        return Err(invalid());
    }
    let current = state.ranked.replicas.lock().get(&game).cloned();
    if matches!(work.phase, Phase::Finished | Phase::Void | Phase::Disputed) {
        if let Some(run) = current {
            // A final certified projection may already be in flight on the socket.
            // Keep its engine briefly for delivery of that exact prefix.
            if work.phase != Phase::Finished || run.status.lock().0.elapsed() >= LEASE {
                retire(state, &run).await;
            }
        }
        return Ok(());
    }
    if let Some(run) = current {
        if run.work.fence == work.fence && run.work.role == work.role {
            let playing = state
                .lobby
                .lock()
                .games
                .get(&run.setup.game_id)
                .is_some_and(|g| g.state == crate::LobbyState::Playing && g.agent_id.is_some());
            if playing && !run.dead.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(());
            }
        }
        retire(state, &run).await;
    }
    restore(state, bridge, work, game).await
}

async fn restore(
    state: &Shared,
    bridge: &crate::central::Bridge,
    work: Work,
    game: Uuid,
) -> Result<(), Failure> {
    let execution = identity(state, work.compatibility).ok_or_else(invalid)?;
    let Response::Page {
        opening,
        setup: Some(body),
        pools: Some(pools),
        ..
    } = rpc(
        bridge,
        Request::Read {
            fence: work.fence,
            after: 0,
        },
    )
    .await
    .map_err(|s| err(s, "ranked opening unavailable"))?
    else {
        return Err(invalid());
    };
    if Head::opening(
        work.fence.match_id,
        *game.as_bytes(),
        work.compatibility,
        work.seed,
        &body,
        &pools,
    ) != opening
    {
        return Err(invalid());
    }
    let preset = match reconstruct(state, &work, &body, &pools).await {
        Ok(preset) => preset,
        Err(failure) => {
            if matches!(failure.0, StatusCode::BAD_REQUEST | StatusCode::CONFLICT) {
                // An invalid frozen registration cannot produce a game state.
                // A negative opening attestation suspends this match neutrally.
                let _ = rpc(
                    bridge,
                    Request::Attest {
                        fence: work.fence,
                        head: opening,
                        state: [0; 32],
                        accepted: false,
                        awaited: 0,
                        decisions: [None; 2],
                        terminal: None,
                    },
                )
                .await;
            }
            return Err(failure);
        }
    };
    let placement = Placement {
        match_id: Uuid::from_bytes(work.fence.match_id),
        game,
        format: work.format,
        accounts: work.accounts,
        decks: work.decks,
        generation: work.fence.generation,
        primary: work.role == Role::Primary,
    };
    let setup = v1::GameSetup {
        game_id: game.to_string(),
        preset_json: serde_json::to_vec(&preset).map_err(|_| invalid())?,
        seat_names: crate::ranked::seat_names(),
        seat_guests: vec![false; 2],
        seat_accounts: vec![true; 2],
        execution: execution.to_metadata(),
    };
    if !replica_table::ensure(state, &placement, &preset).await? {
        return Err(invalid());
    }
    {
        let mut lobby = state.lobby.lock();
        let table = lobby.games.get_mut(&setup.game_id).ok_or_else(invalid)?;
        table.execution = Some(execution);
    }
    // Setup fetching and deck hydration can outlast the original observation.
    // Only the attached engine's own new Pulse starts a player-facing lease.
    let initial = (Instant::now(), Phase::Preparing);
    let run = Arc::new(Run {
        job: format!("{JOB_PREFIX}{}", Uuid::now_v7()),
        work,
        setup,
        opening,
        placement,
        status: Mutex::new(initial),
        engine_head: Mutex::default(),
        dead: AtomicBool::new(false),
        terminal: AtomicBool::new(false),
    });
    state.ranked.replicas.lock().insert(game, Arc::clone(&run));
    if crate::engine::start_replica(state, &run.setup.game_id, &run.job).is_err() {
        retire(state, &run).await;
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "no compatible ranked engine",
        ));
    }
    Ok(())
}

/// Per-engine pulses run independently of opening validation and work-list latency.
/// Closing the authenticated engine link drops this lease task too.
pub(crate) async fn lease(state: &Shared, bridge: &crate::central::Bridge, run: &Run) {
    let mut beat = interval(Duration::from_secs(3));
    beat.set_missed_tick_behavior(MissedTickBehavior::Skip);
    while run.is_current(state) && !run.dead.load(std::sync::atomic::Ordering::Acquire) {
        beat.tick().await;
        renew(state, bridge, run, &run.work).await;
    }
}

async fn renew(state: &Shared, bridge: &crate::central::Bridge, run: &Run, work: &Work) {
    let started = Instant::now();
    let Ok(Response::Assignment { work: latest }) =
        rpc(bridge, Request::Pulse { fence: work.fence }).await
    else {
        return;
    };
    if latest.fence != work.fence
        || latest.role != work.role
        || latest.game != work.game
        || latest.compatibility != run.work.compatibility
        || latest.seed != run.work.seed
        || latest.accounts != run.work.accounts
        || latest.decks != run.work.decks
        || latest.format != run.work.format
        || !run.is_current(state)
    {
        run.status.lock().1 = Phase::Paused;
        run.fence(state);
        return;
    }
    let previous = std::mem::replace(&mut *run.status.lock(), (started, latest.phase));
    if latest.phase != Phase::Playing && previous.1 == Phase::Playing {
        run.fence(state);
    }
    let head = *run.engine_head.lock();
    if head == Some(latest.head) {
        if latest.role == Role::Standby && latest.phase == Phase::Paused {
            let _ = rpc(bridge, Request::Promote { fence: work.fence }).await;
        }
        run.activate(state, latest.head);
    }
}
