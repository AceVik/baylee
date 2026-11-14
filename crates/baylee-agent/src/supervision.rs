//! Own a child until it has actually been reaped.
use super::{Running, status};
use baylee_protocol::v1::{self, Envelope};
use tokio::sync::{mpsc, oneshot};

/// Waits for one engine to exit — or stops it — and reports either way.
pub(super) async fn supervise(
    mut child: tokio::process::Child,
    stop: oneshot::Receiver<()>,
    running: Running,
    out: mpsc::UnboundedSender<Envelope>,
    game_id: String,
) {
    let detail = tokio::select! {
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
        }
    };
    running
        .lock()
        .expect("agent process table")
        .remove(&game_id);
    tracing::info!(game_id, detail, "engine gone");
    let _ = out.send(status(&game_id, v1::engine_status::Kind::Exited, &detail));
}

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
