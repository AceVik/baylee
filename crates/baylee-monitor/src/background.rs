//! Slow sources refresh on their own threads. The 1 Hz sampler only takes the
//! newest finished result, so a slow sensor, a busy disk or a hung driver
//! delays nothing but its own value. Every view carries the time it was
//! taken, so the page can say when one has gone stale.

use std::{
    io,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// The newest finished result of one background source.
pub(crate) struct Latest<T>(Arc<Mutex<Arc<T>>>);

impl<T> Clone for Latest<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Latest<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(Arc::new(value))))
    }

    /// The newest value. Holds the lock only for a reference count.
    pub fn get(&self) -> Arc<T> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn set(&self, value: T) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Arc::new(value);
    }
}

/// Runs `refresh` on a named thread, at most once per `every`, publishing each
/// result to `latest`. A refresh that takes longer than `every` is followed
/// straight by the next one, without any catch-up burst.
pub(crate) fn spawn<T: Send + Sync + 'static>(
    name: &str,
    every: Duration,
    latest: Latest<T>,
    mut refresh: impl FnMut() -> T + Send + 'static,
) -> io::Result<()> {
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            loop {
                let started = Instant::now();
                latest.set(refresh());
                std::thread::sleep(every.saturating_sub(started.elapsed()));
            }
        })?;
    Ok(())
}

/// Unix milliseconds now, for the timestamps on the wire.
pub(crate) fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn a_reader_gets_the_newest_value_and_never_waits_for_a_refresh() {
        let latest = Latest::new(0_u32);
        let (release, gate) = mpsc::channel::<()>();
        let (done, finished) = mpsc::channel();
        let mut value = 0;
        spawn("test", Duration::ZERO, latest.clone(), move || {
            value += 1;
            done.send(value).unwrap();
            // The second refresh blocks, as a hung sensor would.
            if value == 2 {
                gate.recv().unwrap();
            }
            value
        })
        .unwrap();
        assert_eq!(finished.recv().unwrap(), 1);
        assert_eq!(finished.recv().unwrap(), 2);
        // The stuck refresh holds no lock: the first result stays readable.
        assert_eq!(*latest.get(), 1);
        release.send(()).unwrap();
        assert_eq!(finished.recv().unwrap(), 3);
    }
}
