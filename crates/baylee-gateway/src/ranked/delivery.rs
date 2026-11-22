//! One owner per database advertises compatible capacity and replays private
//! terminal evidence. No request from a player can create a result.
use super::{HostRequest, HostResponse, Ordering, Shared};
use std::time::Duration;
use tokio::time::{MissedTickBehavior, interval, timeout};

pub(crate) struct Worker(tokio::task::JoinHandle<()>);
impl Drop for Worker {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) fn worker(state: Shared) -> Worker {
    Worker(tokio::spawn(async move {
        let mut retry = interval(Duration::from_secs(10));
        retry.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            retry.tick().await;
            if state.central.get().is_none() {
                continue;
            }
            let Ok(Ok(Some(mut owner))) = timeout(
                Duration::from_secs(5),
                baylee_db::challenges::Owner::acquire_ranked(&state.db),
            )
            .await
            else {
                continue;
            };
            state.ranked.owner.store(true, Ordering::Release);
            tokio::select! {
                ()=monitor(&state,&mut owner)=>{},
                ()=advertise(&state)=>{},
                ()=reconcile(&state)=>{},
            }
            state.ranked.owner.store(false, Ordering::Release);
            state.ranked.waiting.lock().clear();
            let running: Vec<_> = state
                .lobby
                .lock()
                .running()
                .filter(|g| g.ranked.is_some())
                .map(|g| g.id.clone())
                .collect();
            for game in running {
                crate::engine::end_game(&state, &game);
            }
        }
    }))
}

async fn monitor(state: &Shared, owner: &mut baylee_db::challenges::Owner) {
    let mut check = interval(Duration::from_secs(5));
    check.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        check.tick().await;
        state.ranked.reap();
        if state.central.get().is_none()
            || !matches!(
                timeout(Duration::from_secs(3), owner.check()).await,
                Ok(Ok(()))
            )
        {
            return;
        }
    }
}

/// Occupied ranked games plus spare engine capacity, excluding casual/replay jobs.
fn capacity(state: &Shared) -> u8 {
    if state.admission.held()
        || !state.auth_mode.current().central()
        || state.registration != crate::registration::Registration::Open
    {
        return 0;
    }
    let active = state
        .lobby
        .lock()
        .running()
        .filter(|g| g.ranked.is_some())
        .count();
    let free = state.agents.lock().ranked_spare();
    u8::try_from((active + free).min(64)).unwrap_or(64)
}

async fn advertise(state: &Shared) {
    let mut beat = interval(Duration::from_secs(20));
    beat.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        beat.tick().await;
        let Some(bridge) = state.central.get() else {
            return;
        };
        let complete_catalog = timeout(Duration::from_secs(4), state.pool_table())
            .await
            .ok()
            .flatten()
            .is_some_and(|facts| {
                ["standard", "vintage", "commander"]
                    .iter()
                    .all(|key| facts.formats().iter().any(|available| available == key))
            });
        let capacity = if complete_catalog { capacity(state) } else { 0 };
        // A lost connection can recover; explicit revocation stops rated games.
        let ready = bridge.ranked(HostRequest::Ready { capacity }).await;
        if matches!(
            ready,
            Err(crate::StatusCode::FORBIDDEN | crate::StatusCode::UNAUTHORIZED)
        ) {
            return;
        }
    }
}

async fn reconcile(state: &Shared) {
    let mut timer = interval(Duration::from_secs(3));
    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        let Some(bridge) = state.central.get() else {
            return;
        };
        let Ok(batch) = baylee_db::ranked::pending(&state.db).await else {
            continue;
        };
        for run in batch {
            let id = run.game.to_string();
            let living = state.lobby.lock().games.get(&id).is_some_and(|g| {
                g.ranked.is_some_and(|b| b.id == run.match_id)
                    && (g.state == crate::LobbyState::Playing || g.record_finalizing)
            });
            let request = if let Some(outcome) = run.outcome {
                Some(HostRequest::Finish {
                    match_id: *run.match_id.as_bytes(),
                    game: *run.game.as_bytes(),
                    outcome,
                })
            } else if !living && run.absent_seconds >= 90 {
                Some(HostRequest::Void {
                    match_id: *run.match_id.as_bytes(),
                })
            } else {
                None
            };
            if let Some(request) = request {
                let answer = bridge.ranked(request).await;
                if matches!(answer, Ok(HostResponse::Done))
                    || (run.absent_seconds >= 24 * 60 * 60
                        && matches!(
                            answer,
                            Err(crate::StatusCode::CONFLICT | crate::StatusCode::FORBIDDEN)
                        ))
                {
                    if let Err(error) = baylee_db::ranked::delivered(&state.db, run.match_id).await
                    {
                        tracing::warn!(%error,"ranked delivery acknowledgement could not be stored");
                    }
                    continue;
                }
            }
            if let Err(error) = baylee_db::ranked::touch(&state.db, run.match_id, living).await {
                tracing::warn!(%error,"ranked delivery could not advance");
            }
        }
    }
}
