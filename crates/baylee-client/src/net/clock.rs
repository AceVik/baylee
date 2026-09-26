//! Lowest-RTT server clock estimate, measured against a monotonic local clock.

#[derive(Default)]
pub(super) struct Clock {
    best: Option<(u64, f64)>,
}

impl Clock {
    pub(super) fn sample(&mut self, sent: u64, received: u64, server: u64) {
        if server == 0 || sent > received {
            return;
        }
        let rtt = received - sent;
        if self.best.is_none_or(|(best, _)| rtt < best) {
            self.best = Some((rtt, server as f64 - (sent as f64 + rtt as f64 * 0.5)));
        }
    }

    pub(super) fn ready(&self) -> bool {
        self.best.is_some()
    }

    pub(super) fn until(&self, local: u64, server: u64) -> Option<f64> {
        self.best
            .map(|(_, offset)| (server as f64 - local as f64 - offset) / 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_clocks_depart_together_and_ignore_slower_samples() {
        let mut a = Clock::default();
        let mut b = Clock::default();
        a.sample(100, 140, 10_020);
        b.sample(8100, 8180, 10_040);
        assert_eq!(a.until(600, 11_500), Some(1.0));
        assert_eq!(b.until(8600, 11_500), Some(1.0));
        a.sample(200, 1200, 10_220);
        assert_eq!(a.until(600, 11_500), Some(1.0));
    }

    #[test]
    fn invalid_samples_cannot_enable_readiness() {
        let mut clock = Clock::default();
        clock.sample(5, 4, 900);
        clock.sample(4, 5, 0);
        assert!(!clock.ready());
        assert_eq!(clock.until(100, 100), None);
    }
}
