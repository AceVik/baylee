//! Own a child until it has actually been reaped.
use super::{Running, status};
use baylee_protocol::v1::{self, Envelope};
use tokio::sync::{mpsc, oneshot};

mod watchdog;

/// Waits for one engine to exit — or stops it — and reports either way.
pub(super) async fn supervise(
    child: tokio::process::Child,
    stop: oneshot::Receiver<()>,
    running: Running,
    out: mpsc::UnboundedSender<Envelope>,
    game_id: String,
) {
    with_limits(
        child,
        stop,
        running,
        out,
        game_id,
        baylee_worker::Limits::default(),
    )
    .await;
}

async fn with_limits(
    mut child: tokio::process::Child,
    stop: oneshot::Receiver<()>,
    running: Running,
    out: mpsc::UnboundedSender<Envelope>,
    game_id: String,
    limits: baylee_worker::Limits,
) {
    // Child::wait closes its stdin automatically. Keep this separate so the
    // worker's owner-liveness thread sees EOF only when this supervisor ends.
    let _owner_pipe = child.stdin.take();
    let reports = child.stdout.take();
    let watched = watchdog::watch(reports, child.id(), limits);
    let detail = tokio::select! {
        biased;
        exit = child.wait() => match exit {
            Ok(exit) => exit.to_string(),
            Err(err) => {
                tracing::error!(game_id, %err, "engine wait failed; retaining ownership");
                reap_after_error(&mut child, &game_id).await
            },
        },
        _ = stop => {
            // Stop only this owned process, then confirm its exit before
            // releasing the slot. The gateway has already ended the game.
            let _ = child.start_kill();
            match child.wait().await {
                Ok(exit) => format!("stopped ({exit})"),
                Err(err) => {
                    tracing::error!(game_id, %err, "engine stop wait failed; retaining ownership");
                    reap_after_error(&mut child, &game_id).await
                },
            }
        },
        failure = watched => {
            // Resource failures carry no game outcome. Release the slot and
            // report only after the actual child, not a PID lookup, is reaped.
            let stopped = reap_after_error(&mut child, &game_id).await;
            format!("technical interruption: {failure}; {stopped}")
        },
    };
    running
        .lock()
        .expect("agent process table")
        .remove(&game_id);
    tracing::info!(game_id, detail, "engine gone");
    let _ = out.send(status(&game_id, v1::engine_status::Kind::Exited, &detail));
}

#[cfg(test)]
#[path = "supervision/tests.rs"]
mod tests;

/// Preserve ownership and retry after an OS wait failure.
async fn reap_after_error(child: &mut tokio::process::Child, game_id: &str) -> String {
    loop {
        let _ = child.start_kill();
        match child.wait().await {
            Ok(exit) => return format!("stopped ({exit})"),
            Err(err) => tracing::error!(game_id, %err, "engine reap failed; retaining ownership"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
