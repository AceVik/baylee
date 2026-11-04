//! How full a filesystem is, and how fast it is filling.

use crate::types::Usage;
use std::collections::VecDeque;

/// A sample every half minute for half an hour: long enough to see through a
/// burst, short enough that a runaway job shows within minutes.
const SPACING_SECS: f64 = 30.0;
const KEEP: usize = 60;
/// Below this span a trend is mostly noise.
const MIN_SPAN_SECS: f64 = 300.0;
/// Slower filling than this is not worth a forecast, in bytes per second.
const MIN_RATE: f64 = 16.0 * 1024.0;

/// Reads `statvfs`. Blocks if the filesystem does, so it runs on the storage
/// thread only, and network filesystems are never asked.
pub(super) fn read(path: &str) -> Option<Usage> {
    let vfs = rustix::fs::statvfs(path).ok()?;
    Some(from_counts(
        vfs.f_frsize,
        vfs.f_blocks,
        vfs.f_bfree,
        vfs.f_bavail,
        vfs.f_files,
        vfs.f_ffree,
    ))
}

#[allow(clippy::cast_precision_loss)] // Percentages.
fn from_counts(
    block: u64,
    blocks: u64,
    free: u64,
    available: u64,
    inodes: u64,
    inodes_free: u64,
) -> Usage {
    let used = blocks.saturating_sub(free).saturating_mul(block);
    let available = available.saturating_mul(block);
    // Like `df`: reserved blocks count as neither used nor available.
    let usable = used.saturating_add(available);
    Usage {
        total: blocks.saturating_mul(block),
        used,
        available,
        used_pct: if usable == 0 {
            0.0
        } else {
            100.0 * used as f32 / usable as f32
        },
        // NTFS, FAT and btrfs report no fixed inode count.
        inodes_pct: (inodes > 0)
            .then(|| 100.0 * inodes.saturating_sub(inodes_free) as f32 / inodes as f32),
        filling_bps: None,
        full_in_secs: None,
    }
}

/// Free space over the last half hour of one filesystem.
#[derive(Default)]
pub(super) struct Growth {
    samples: VecDeque<(f64, f64)>,
}

impl Growth {
    /// Notes the free bytes at `at_secs` and fills in the trend on `usage`.
    #[allow(clippy::cast_precision_loss)] // A trend does not need exact bytes.
    pub fn apply(&mut self, at_secs: f64, usage: &mut Usage) {
        let due = self
            .samples
            .back()
            .is_none_or(|(last, _)| at_secs - last >= SPACING_SECS);
        if due {
            if self.samples.len() == KEEP {
                self.samples.pop_front();
            }
            self.samples.push_back((at_secs, usage.available as f64));
        }
        if let Some(filling) = self.filling_rate() {
            usage.filling_bps = Some(filling);
            if filling > MIN_RATE {
                usage.full_in_secs = Some((usage.available as f64 / filling) as u64);
            }
        }
    }

    /// The least-squares slope of the free space, negated: bytes per second
    /// the filesystem is filling at. Negative while it is emptying.
    #[allow(clippy::cast_precision_loss)] // Sixty samples.
    fn filling_rate(&self) -> Option<f64> {
        let (first, _) = self.samples.front()?;
        let (last, _) = self.samples.back()?;
        if last - first < MIN_SPAN_SECS {
            return None;
        }
        let count = self.samples.len() as f64;
        let mean_time = self.samples.iter().map(|(t, _)| t).sum::<f64>() / count;
        let mean_free = self.samples.iter().map(|(_, free)| free).sum::<f64>() / count;
        let (mut covariance, mut variance) = (0.0, 0.0);
        for (time, free) in &self.samples {
            covariance += (time - mean_time) * (free - mean_free);
            variance += (time - mean_time) * (time - mean_time);
        }
        (variance > 0.0).then(|| -covariance / variance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn usage(available: u64) -> Usage {
        Usage {
            available,
            ..Usage::default()
        }
    }

    #[test]
    fn usage_counts_like_df() {
        // 100 blocks, 10 reserved for root: 40 used, 50 available to users.
        let usage = from_counts(4096, 100, 60, 50, 1000, 750);
        assert_eq!(usage.total, 100 * 4096);
        assert_eq!(usage.used, 40 * 4096);
        assert_eq!(usage.available, 50 * 4096);
        assert!((usage.used_pct - 100.0 * 40.0 / 90.0).abs() < 0.01);
        assert_eq!(usage.inodes_pct, Some(25.0));
    }

    #[test]
    fn a_filesystem_without_inode_counts_says_so() {
        assert_eq!(from_counts(4096, 100, 60, 60, 0, 0).inodes_pct, None);
        assert!(from_counts(4096, 0, 0, 0, 0, 0).used_pct.abs() < f32::EPSILON);
    }

    #[test]
    fn a_steady_fill_forecasts_when_the_disk_is_full() {
        let mut growth = Growth::default();
        let mut free = 100 * GIB;
        let mut last = usage(free);
        // 1 MiB/s for ten minutes, seen at the storage thread's five seconds.
        for tick in 0_u32..120 {
            last = usage(free);
            growth.apply(f64::from(tick) * 5.0, &mut last);
            free -= 5 << 20;
        }
        let rate = last.filling_bps.unwrap();
        assert!((rate - f64::from(1 << 20)).abs() < 1000.0, "{rate}");
        let expected = last.available >> 20;
        let forecast = last.full_in_secs.unwrap();
        assert!(forecast.abs_diff(expected) < 10, "{forecast} vs {expected}");
    }

    #[test]
    fn no_forecast_before_five_minutes_of_samples() {
        let mut growth = Growth::default();
        let mut last = usage(0);
        for tick in 0_u32..50 {
            last = usage(u64::from(100 - tick) * GIB);
            growth.apply(f64::from(tick) * 5.0, &mut last);
        }
        assert_eq!(last.filling_bps, None);
        assert_eq!(last.full_in_secs, None);
    }

    #[test]
    fn a_stable_or_emptying_disk_has_no_forecast() {
        for step in [0_i64, 1 << 30] {
            let mut growth = Growth::default();
            let mut last = usage(0);
            for tick in 0_i32..120 {
                last = usage((100 * (1 << 30) + i64::from(tick) * step) as u64);
                growth.apply(f64::from(tick) * 5.0, &mut last);
            }
            assert!(last.filling_bps.unwrap() <= 0.0);
            assert_eq!(last.full_in_secs, None);
        }
    }

    #[test]
    fn old_samples_leave_the_window() {
        let mut growth = Growth::default();
        for tick in 0_u32..500 {
            growth.apply(f64::from(tick) * 30.0, &mut usage(GIB));
        }
        assert_eq!(growth.samples.len(), KEEP);
    }
}
