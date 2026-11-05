//! The fill and what it has to give way to: the core's [`Fill`] behind a lock,
//! with a real clock.
//!
//! A wanted picture does not wait here. It says it is on its way ([`Courier::
//! wanted`]) and goes, on the thread `blocking` gave its load. Only the fill's
//! own workers sleep on this, and nothing ever waits on a frame.

use baylee_client_core::cachefill::{Answer, Fill, Job, Next};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// How long an idle worker sleeps at most before looking again; a change of
/// steering wakes it sooner.
const IDLE: Duration = Duration::from_secs(2);

/// The fill and its gate.
pub struct Courier {
    fill: Mutex<Fill>,
    /// Woken whenever a slot comes free or the steering changes.
    turned: Condvar,
    /// What the fill's clock counts from.
    started: Instant,
    /// Wanted pictures in flight, for a running fill request to read between
    /// two reads of its body without taking the lock.
    wanted: AtomicUsize,
    /// How far a test has moved the clock on, in milliseconds.
    #[cfg(test)]
    skipped: std::sync::atomic::AtomicU64,
}

impl Courier {
    /// A courier whose waits stretch by `spread` (`Backoff::new`).
    #[must_use]
    pub fn new(spread: f32) -> Self {
        Self {
            fill: Mutex::new(Fill::new(spread)),
            turned: Condvar::new(),
            started: Instant::now(),
            wanted: AtomicUsize::new(0),
            #[cfg(test)]
            skipped: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Fill> {
        self.fill.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[cfg(not(test))]
    fn now(&self) -> Duration {
        self.started.elapsed()
    }

    #[cfg(test)]
    fn now(&self) -> Duration {
        self.started.elapsed() + Duration::from_millis(self.skipped.load(Ordering::SeqCst))
    }

    /// Moves the clock on: a test need not sit out the settling.
    #[cfg(test)]
    pub(crate) fn skip(&self, span: Duration) {
        let millis = u64::try_from(span.as_millis()).expect("a short span");
        self.skipped.fetch_add(millis, Ordering::SeqCst);
        self.turned.notify_all();
    }

    /// Runs `act` on the fill at the time it is now, and wakes whoever waits.
    pub fn with<R>(&self, act: impl FnOnce(&mut Fill, Duration) -> R) -> R {
        let answer = act(&mut self.lock(), self.now());
        self.turned.notify_all();
        answer
    }

    /// A wanted picture is on its way, until the answer is dropped. It waits
    /// for nothing: this only tells the fill to give way.
    #[must_use]
    pub fn wanted(&self) -> Wanted<'_> {
        self.wanted.fetch_add(1, Ordering::SeqCst);
        self.lock().gate().wanted_started();
        Wanted { courier: self }
    }

    /// Whether a running fill request should stop for a wanted picture.
    #[must_use]
    pub fn giving_way(&self) -> bool {
        self.wanted.load(Ordering::SeqCst) > 0
    }

    /// What a worker does next.
    pub(crate) fn next(&self) -> Next {
        self.lock().next(self.now())
    }

    /// Hands back one fill request.
    pub(crate) fn finished(&self, job: &Job, answer: Answer) {
        self.with(|fill, now| fill.finished(job, answer, now));
    }

    /// Sleeps for `span`, or until something changes.
    pub(crate) fn sleep(&self, span: Duration) {
        let span = span.min(IDLE);
        let fill = self.lock();
        drop(
            self.turned
                .wait_timeout(fill, span)
                .unwrap_or_else(PoisonError::into_inner),
        );
    }
}

/// A wanted picture on its way, for as long as its request runs.
pub struct Wanted<'a> {
    courier: &'a Courier,
}

impl Drop for Wanted<'_> {
    /// Whichever way a request ended, the fill may look again.
    fn drop(&mut self) {
        self.courier
            .with(|fill, now| fill.gate().wanted_finished(now));
        self.courier.wanted.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The app's courier: the one [`crate::artreader::ArtReader`] tells and the
/// fill works from.
#[must_use]
pub fn shared() -> Arc<Courier> {
    /// One per process.
    static COURIER: LazyLock<Arc<Courier>> = LazyLock::new(|| Arc::new(Courier::new(spread())));
    COURIER.clone()
}

/// This client's own stretch of its waits, 0 to 1: a server that comes back up
/// should not be asked by every client in the same second.
fn spread() -> f32 {
    let mut byte = [0_u8; 1];
    if getrandom::fill(&mut byte).is_err() {
        return 0.5;
    }
    f32::from(byte[0]) / 255.0
}
