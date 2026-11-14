//! Ordinary and retained v1 attachment, unchanged by Tower v2 transport.
use super::*;

/// The game itself, on whichever socket reached the gateway.
pub(super) async fn play<S>(
    mut ws: tokio_tungstenite::WebSocketStream<S>,
    attach: Attach,
    mut runner: EngineRunner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut progress = reporter()?;
    // The first frame proves this process is the one the gateway asked an
    // agent to start. Nothing else on this socket is authenticated,
    // because nothing else needs to be: the gateway closes it otherwise.
    send(&mut ws, &engine_hello(&attach)).await?;
    tracing::info!(game_id = attach.game_id, url = attach.url, "attached");

    // Each deadline is anchored to what it was armed for, so it restarts
    // when its seat is asked something new rather than every time a frame
    // from another seat wakes this task up.
    let mut armed = baylee_engine_server::Armed::default();
    // The curtain's deadline (#256). The table's rather than a seat's, so
    // not a `Clock`: armed on the first turn of this loop after the game
    // is built, whether or not any seat has attached, and dropped once
    // the curtain is up.
    let mut curtain_at = None;
    let measured_from = tokio::time::Instant::now();
    loop {
        runner.wait_from(u64::try_from(measured_from.elapsed().as_millis()).unwrap_or(u64::MAX));
        let now = tokio::time::Instant::now();
        armed.sync(&runner.clocks(), |clock| {
            now + std::time::Duration::from_secs(u64::from(clock.secs))
        });
        curtain_at =
            (runner.curtain_pending() && runner.entrance_deadline().is_none()).then(|| {
                curtain_at.unwrap_or_else(|| {
                    now + std::time::Duration::from_secs(u64::from(
                        baylee_engine_server::CURTAIN_SECS,
                    ))
                })
            });
        let arrival = runner
            .entrance_deadline()
            .map(|at| now + std::time::Duration::from_millis(at.saturating_sub(wall_ms())));
        // The record that has waited long enough goes to the gateway even
        // when nothing moves (#323). Wall time, like the entrance.
        let record_at = runner
            .record_deadline()
            .map(|at| now + std::time::Duration::from_millis(at.saturating_sub(wall_ms())));
        let next = armed.next();
        let needs_pump = runner.needs_pump();
        tokio::select! {
            biased;
            () = deadline(record_at) => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap_or(u64::MAX));
                runner.tell_time(wall_ms());
                send_batch(&mut ws, &progress.work(|| runner.flush_record_due())?).await?;
            }
            () = deadline(arrival) => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap_or(u64::MAX));
                runner.tell_time(wall_ms());
                send_batch(&mut ws, &progress.work(|| runner.finish_entrance())?).await?;
            }
            () = deadline(curtain_at) => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap_or(u64::MAX));
                tracing::info!(game_id = attach.game_id, "table preparation timed out");
                runner.tell_time(wall_ms());
                send_batch(&mut ws, &progress.work(|| runner.preparation_expired())?).await?;
            }
            () = deadline(next.map(|(_, at)| at)) => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap_or(u64::MAX));
                // The clock is the one thing the rules kernel must not
                // own: it is deterministic and may not read a wall clock.
                let Some((clock, _)) = next else { continue };
                tracing::info!(seat = clock.seat.get(), "decision timed out");
                armed.fired(clock);
                runner.tell_time(wall_ms());
                send_batch(&mut ws, &progress.work(|| runner.timeout(clock))?).await?;
            }
            frame = ws.next() => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap_or(u64::MAX));
                let Some(frame) = frame else { break };
                let frame = frame?;
                if !frame.is_binary() {
                    continue;
                }
                let envelope = Envelope::decode(frame.into_data())?;
                // The gateway's one word to an engine it will not keep
                // (#271): said as this process's failure, in its words.
                if let Some(v1::envelope::Msg::Error(refused)) = envelope.msg {
                    return Err(refused.message.into());
                }
                // Started for a replay job, not a game (`docs/protocol.md`
                // §"Replays"): project the record, send the tape, exit.
                if runner.session().is_none()
                    && let Some(v1::envelope::Msg::ProjectRecord(order)) = envelope.msg
                {
                    return project(ws, order, progress).await;
                }
                if let Some(v1::envelope::Msg::GameSetup(setup)) = &envelope.msg
                    && !baylee_engine_server::variant::accepts(&setup.execution) {
                    return Err("game setup does not match this engine artifact".into());
                }
                let remaining = remaining_now(&armed);
                runner.tell_time(wall_ms());
                send_batch(&mut ws, &progress.work(|| runner.handle(envelope, &remaining))?).await?;
            }
            () = tokio::task::yield_now(), if needs_pump => {
                runner.measure_until(u64::try_from(measured_from.elapsed().as_millis()).unwrap_or(u64::MAX));
                let remaining = remaining_now(&armed);
                runner.tell_time(wall_ms());
                send_batch(&mut ws, &progress.work(|| runner.pump_slice(&remaining))?).await?;
            }
        }
        if runner.finished() {
            break;
        }
    }
    // Everything queued is already flushed by `send`; this is the polite
    // close the gateway logs as an ordinary end rather than a drop.
    let _ = ws.close(None).await;
    Ok(())
}

/// Reads the currently armed clocks against `Instant::now()`.
fn remaining_now(
    armed: &baylee_engine_server::Armed<tokio::time::Instant>,
) -> Vec<(baylee_engine_server::Clock, u32)> {
    let read = tokio::time::Instant::now();
    armed.remaining(|at| {
        u32::try_from(at.saturating_duration_since(read).as_millis()).unwrap_or(u32::MAX)
    })
}

/// A replay job (`baylee_engine_server::projection`): the projection runs
/// on a blocking thread and its pieces go out as they are made; then the
/// socket closes and the process exits.
async fn project<S>(
    mut ws: tokio_tungstenite::WebSocketStream<S>,
    order: v1::ProjectRecord,
    mut progress: baylee_worker::Reporter<std::io::Stdout>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let job = order.job.clone();
    let worker = tokio::task::spawn_blocking(move || {
        progress.work(|| {
            baylee_engine_server::projection::run(&order, baylee_build::short(), &mut |piece| {
                let _ = tx.send(piece);
            })
        })
    });
    while let Some(piece) = rx.recv().await {
        send(
            &mut ws,
            &Envelope {
                msg: Some(v1::envelope::Msg::TapeChunk(piece)),
            },
        )
        .await?;
    }
    let stop = worker.await??;
    tracing::info!(job, stop = ?stop.stop, inputs = stop.inputs, "replay projected");
    let _ = ws.close(None).await;
    Ok(())
}
