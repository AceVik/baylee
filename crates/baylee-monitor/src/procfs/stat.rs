//! `/proc/stat`: where the CPU's time went, and the scheduler's counters.

use serde::Serialize;

/// Cumulative ticks of the summary `cpu` line, in the kernel's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stat {
    pub cpu: CpuTimes,
    pub interrupts: u64,
    pub context_switches: u64,
    /// Processes and threads created since boot.
    pub forks: u64,
    pub running: u32,
    /// Waiting uninterruptibly, usually for a disk.
    pub blocked: u32,
    pub boot_secs: u64,
}

pub(crate) fn parse(text: &str) -> Stat {
    let mut stat = Stat::default();
    for line in text.lines() {
        let mut fields = line.split_ascii_whitespace();
        let Some(key) = fields.next() else { continue };
        let mut number = || fields.next().and_then(|v| v.parse().ok()).unwrap_or(0_u64);
        match key {
            "cpu" => {
                stat.cpu = CpuTimes {
                    user: number(),
                    nice: number(),
                    system: number(),
                    idle: number(),
                    iowait: number(),
                    irq: number(),
                    softirq: number(),
                    steal: number(),
                };
            }
            "intr" => stat.interrupts = number(),
            "ctxt" => stat.context_switches = number(),
            "processes" => stat.forks = number(),
            "procs_running" => stat.running = u32::try_from(number()).unwrap_or(u32::MAX),
            "procs_blocked" => stat.blocked = u32::try_from(number()).unwrap_or(u32::MAX),
            "btime" => stat.boot_secs = number(),
            _ => {}
        }
    }
    stat
}

/// Shares of the CPU's time between two readings, in percent of all CPUs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct CpuShares {
    pub user: f32,
    pub system: f32,
    pub iowait: f32,
    pub irq: f32,
    pub steal: f32,
    pub idle: f32,
}

#[allow(clippy::cast_precision_loss)] // Percentages.
pub(crate) fn shares(before: &CpuTimes, after: &CpuTimes) -> CpuShares {
    let delta = |pick: fn(&CpuTimes) -> u64| pick(after).saturating_sub(pick(before)) as f32;
    let user = delta(|t| t.user) + delta(|t| t.nice);
    let system = delta(|t| t.system);
    let iowait = delta(|t| t.iowait);
    let irq = delta(|t| t.irq) + delta(|t| t.softirq);
    let steal = delta(|t| t.steal);
    let idle = delta(|t| t.idle);
    let total = user + system + iowait + irq + steal + idle;
    if total <= 0.0 {
        return CpuShares::default();
    }
    let share = |part: f32| 100.0 * part / total;
    CpuShares {
        user: share(user),
        system: share(system),
        iowait: share(iowait),
        irq: share(irq),
        steal: share(steal),
        idle: share(idle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "cpu  100 10 50 800 20 5 5 10 0 0\n\
        cpu0 1 2 3 4 5 6 7 8 9 10\n\
        intr 42000 1 2 3\nctxt 123456\nbtime 1793692000\nprocesses 7777\n\
        procs_running 3\nprocs_blocked 2\nsoftirq 9 9 9\n";

    #[test]
    fn reads_the_summary_line_and_the_counters() {
        let stat = parse(FIXTURE);
        assert_eq!(stat.cpu.user, 100);
        assert_eq!(stat.cpu.iowait, 20);
        assert_eq!(stat.cpu.steal, 10);
        assert_eq!(stat.interrupts, 42_000);
        assert_eq!(stat.context_switches, 123_456);
        assert_eq!(stat.forks, 7_777);
        assert_eq!(stat.running, 3);
        assert_eq!(stat.blocked, 2);
        assert_eq!(stat.boot_secs, 1_793_692_000);
    }

    #[test]
    fn shares_cover_the_elapsed_ticks() {
        let before = CpuTimes::default();
        let after = parse(FIXTURE).cpu;
        let result = shares(&before, &after);
        assert!((result.user - 11.0).abs() < 0.01, "user and nice");
        assert!((result.iowait - 2.0).abs() < 0.01);
        assert!((result.idle - 80.0).abs() < 0.01);
        let sum =
            result.user + result.system + result.iowait + result.irq + result.steal + result.idle;
        assert!((sum - 100.0).abs() < 0.01);
    }

    #[test]
    fn no_elapsed_ticks_give_zero_not_nan() {
        let times = parse(FIXTURE).cpu;
        assert_eq!(shares(&times, &times), CpuShares::default());
    }
}
