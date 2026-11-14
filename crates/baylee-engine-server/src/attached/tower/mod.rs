//! Compiled native Tower attachment. Frozen policy; no ordinary preset fallback.
use super::{Attach, Envelope, engine_hello, reporter, wall_ms};
use baylee_engine_server::tower::durable::Table;
use baylee_protocol::v1::envelope::Msg;
use baylee_tower_api::engine::Control;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;

mod connection;
mod recovery;
#[cfg(test)]
mod tests;
use connection::Connection;
type Failure = Box<dyn std::error::Error + Send + Sync>;
type Progress = baylee_worker::Reporter<std::io::Stdout>;

pub(super) async fn play<S: AsyncRead + AsyncWrite + Unpin>(
    mut socket: WebSocketStream<S>,
    attach: Attach,
) -> Result<(), Failure> {
    super::send(&mut socket, &engine_hello(&attach)).await?;
    let mut connection = Connection::new(&mut socket);
    let (control, _) =
        tokio::time::timeout(Duration::from_secs(15), connection.control()).await??;
    let Control::Start { setup } = control else {
        return Err("Tower setup required".into());
    };
    if setup.game.to_string() != attach.game_id {
        return Err("Tower game binding refused".into());
    }
    let fence = setup.fence;
    let mut progress = reporter()?;
    let mut table = if setup.prefix.chunks == 0 {
        progress.work(|| {
            Table::new(
                setup.game.to_string(),
                &setup.package,
                &setup.catalog,
                fence,
                setup.byte_limit,
                setup.names,
            )
        })??
    } else {
        let restored = recovery::restore(&mut connection, *setup, progress).await?;
        progress = restored.1;
        restored.0
    };
    let mut frames = Vec::new();
    loop {
        if let Some(batch) = table.batch() {
            let chunks = u8::try_from(batch.chunks().count())?;
            for (index, data) in batch.chunks().enumerate() {
                connection
                    .send(
                        &Control::Append {
                            fence,
                            expected: batch.expected,
                            chunks,
                            index: u8::try_from(index)?,
                        },
                        data.to_vec(),
                    )
                    .await?;
            }
            let (control, _) =
                tokio::time::timeout(Duration::from_secs(20), connection.control()).await??;
            let Control::Acknowledged { acknowledgement } = control else {
                return Err("Tower commit acknowledgement required".into());
            };
            frames.extend(table.acknowledge(acknowledgement.fence, acknowledgement.prefix)?);
        }
        if table.finished() {
            let expected = table.acknowledged();
            connection
                .send(&Control::Seal { expected }, Vec::new())
                .await?;
            let (control, _) =
                tokio::time::timeout(Duration::from_secs(20), connection.control()).await??;
            let Control::Sealed { acknowledgement } = control else {
                return Err("Tower sealed acknowledgement required".into());
            };
            if acknowledgement.fence != fence || acknowledgement.prefix != expected {
                return Err("Tower sealed binding refused".into());
            }
            connection.frames(&frames).await?;
            connection.send(&Control::Finished {}, Vec::new()).await?;
            break;
        }
        connection.frames(&frames).await?;
        frames.clear();
        let arrival = table.entrance_deadline().map(|at| {
            tokio::time::Instant::now() + Duration::from_millis(at.saturating_sub(wall_ms()))
        });
        tokio::select! {
            biased;
            message = connection.next() => {
                let message = message?;
                if matches!(message.msg, Some(Msg::CloseGame(_))) { return Err("Tower interrupted by host".into()); }
                table.tell_time(wall_ms())?;
                frames = progress.work(|| table.handle(message, &[]))??;
            }
            () = super::deadline(arrival) => {
                table.tell_time(wall_ms())?;
                frames = progress.work(|| table.finish_entrance())??;
            }
            () = tokio::task::yield_now(), if table.needs_pump() => {
                table.tell_time(wall_ms())?;
                frames = progress.work(|| table.pump(&[]))??;
            }
        }
    }
    let _ = socket.close(None).await;
    Ok(())
}
