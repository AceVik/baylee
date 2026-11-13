//! Bounded retry queue. Ambiguous network responses retain the immutable outcome.
use super::{AttemptState, Completion, HostRequest, HostResponse, Shared};
use std::time::Duration;

pub(crate) struct Worker(tokio::task::JoinHandle<()>);
impl Drop for Worker {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) fn start(state: Shared) -> Worker {
    Worker(tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_secs(10));
        every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            every.tick().await;
            if pass(&state).await.is_err() {
                tracing::warn!("challenge delivery unavailable; retained for retry");
            }
        }
    }))
}

async fn pass(state: &Shared) -> Result<(), ()> {
    let Some(bridge) = state.central.get().filter(|bridge| bridge.official()) else {
        return Ok(());
    };
    let runs = baylee_db::challenges::pending(&state.db)
        .await
        .map_err(|_| ())?;
    for run in runs {
        let game_id = run.game.to_string();
        let has_result = run.result.is_some();
        if let Some(result) = run.result {
            // Never recompute time, failures or result on retry.
            if let Ok(HostResponse::Attempt { attempt }) =
                bridge.challenge(HostRequest::Finish { result }).await
                && attempt.id == *run.attempt.as_bytes()
                && !attempt.state.active()
            {
                baylee_db::challenges::delivered(&state.db, run.attempt)
                    .await
                    .map_err(|_| ())?;
                continue;
            }
        }
        let active = state.lobby.lock().games.get(&game_id).is_some_and(|game| {
            game.state == crate::LobbyState::Playing || game.challenge_finalizing
        });
        // A just-reserved local binding may be awaiting Claim or engine launch.
        if !active && !has_result && run.idle < 120 {
            continue;
        }
        let status = bridge
            .challenge(HostRequest::Status {
                attempt: *run.attempt.as_bytes(),
            })
            .await;
        match status {
            Ok(HostResponse::Attempt { attempt }) if attempt.id == *run.attempt.as_bytes() => {
                if !attempt.state.active() {
                    if active {
                        crate::engine::end_game(state, &game_id);
                    }
                    baylee_db::challenges::delivered(&state.db, run.attempt)
                        .await
                        .map_err(|_| ())?;
                } else if !active
                    && !has_result
                    && attempt.state == AttemptState::Playing
                    && attempt.game == Some(*run.game.as_bytes())
                {
                    // Confirmed host failure after the grace period, never a player defeat.
                    let result = Completion {
                        attempt: attempt.id,
                        game: *run.game.as_bytes(),
                        state: AttemptState::Void,
                        turn: 0,
                        finished_at: crate::sessions::now(),
                        milliseconds: None,
                        record_digest: [0; 32],
                    };
                    baylee_db::challenges::complete(&state.db, &result)
                        .await
                        .map_err(|_| ())?;
                } else if !active && attempt.state == AttemptState::Reserved {
                    baylee_db::challenges::delivered(&state.db, run.attempt)
                        .await
                        .map_err(|_| ())?;
                } else {
                    baylee_db::challenges::touch(&state.db, run.attempt)
                        .await
                        .map_err(|_| ())?;
                }
            }
            _ => {
                baylee_db::challenges::touch(&state.db, run.attempt)
                    .await
                    .map_err(|_| ())?;
            }
        }
    }
    Ok(())
}
