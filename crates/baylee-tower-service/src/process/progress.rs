use crate::Error;
use baylee_worker::{ALIVE, BUSY, Budget, IDLE, Limits};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt as _};

/// Exactly one operation, independent heartbeat and no logged/free-form output.
pub(crate) async fn progress(
    mut pipe: impl AsyncRead + Unpin,
    limits: Limits,
) -> Result<(), Error> {
    let started = Instant::now();
    let mut budget = Budget::new(limits, Duration::ZERO);
    let mut ticks = tokio::time::interval(Duration::from_millis(100));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut phase = 0;
    let mut count = 0_u16;
    loop {
        tokio::select! {
            biased;
            _ = ticks.tick() => budget.check(started.elapsed(), 0).map_err(|_| Error::Budget)?,
            marker = pipe.read_u8() => {
                let marker = match marker {
                    Ok(marker) => marker,
                    Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof && phase == 3 => return Ok(()),
                    Err(_) => return Err(Error::Contract),
                };
                count += 1;
                if count > 512 {
                    return Err(Error::Contract);
                }
                phase = match (phase, marker) {
                    (0 | 2, IDLE) | (1, BUSY) => phase + 1,
                    (2 | 3, ALIVE) => phase,
                    _ => return Err(Error::Contract),
                };
                budget.observe(marker, started.elapsed()).map_err(|error| match error {
                    baylee_worker::Failure::Startup | baylee_worker::Failure::Operation | baylee_worker::Failure::Memory => Error::Budget,
                    _ => Error::Contract,
                })?;
            },
        }
    }
}
