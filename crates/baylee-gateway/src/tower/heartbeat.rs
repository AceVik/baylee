//! Owned availability refresh; the authority expires lost hosts after sixty seconds.
use super::Shared;
use std::time::Duration;

pub(crate) struct Heartbeat(tokio::task::JoinHandle<()>);

impl Heartbeat {
    pub(crate) fn start(state: Shared) -> Option<Self> {
        state.tower.as_ref()?;
        Some(Self(tokio::spawn(async move {
            let mut ticks = tokio::time::interval(Duration::from_secs(20));
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticks.tick().await;
                let Some(tower) = &state.tower else { return };
                let Ok(_start) = tower.starts.try_lock() else {
                    continue;
                };
                // A failed publication cannot extend stale availability. No player,
                // token, origin or remote diagnostic belongs in the periodic log.
                if tower.advertise(&state).await.is_err() {
                    tracing::debug!("Tower availability refresh deferred");
                }
            }
        })))
    }
}

impl Drop for Heartbeat {
    fn drop(&mut self) {
        self.0.abort();
    }
}
