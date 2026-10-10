//! Server metrics for the admin console (`GET /admin/metrics`;
//! `docs/protocol.md` §"The admin console"): the host's load, CPU, memory,
//! disk and network, this process's open sockets and request rate, and the
//! CPU and memory of each engine process on this machine, sampled every
//! [`INTERVAL`] into a ring of the last hour ([`KEEP`] samples). Memory
//! only: nothing is written anywhere, and nothing here names a person — a
//! game's id and its process id are the most a sample says.
//!
//! The host is read off `/proc`, so those fields exist on Linux; elsewhere
//! (a developer's macOS) they are `null` and the rest — the disk through
//! `statvfs`, this process's uptime and request rate — is answered the
//! same. The reading happens on a blocking thread, never under the lobby
//! lock, which is held only to copy out which games have a local engine.
//!
//! Counters (CPU jiffies, network bytes, a process's ticks, requests) are
//! kept from the previous tick and only their rates enter the ring; a
//! counter that went backwards (a reused pid, a reset interface) reads as
//! zero rather than as a negative rate.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// How often a sample is taken.
pub const INTERVAL: Duration = Duration::from_secs(5);

/// How many samples are kept: an hour at [`INTERVAL`].
pub const KEEP: usize = 720;

/// The filesystem whose space is reported.
const DISK_PATH: &str = "/";

/// One engine process as read at one instant.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcReading {
    /// The game the process runs.
    pub game_id: String,
    pub pid: u32,
    /// `utime + stime`, in clock ticks.
    pub ticks: u64,
    /// Resident memory, in bytes.
    pub rss: u64,
}

/// Raw values at one instant, from which a sample is derived.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reading {
    /// CPU jiffies: `(busy, total)` over every core.
    pub cpu: Option<(u64, u64)>,
    /// The 1-, 5- and 15-minute load averages.
    pub load: Option<[f32; 3]>,
    /// Memory in bytes: `(total, available)`.
    pub mem: Option<(u64, u64)>,
    /// The disk at [`DISK_PATH`] in bytes: `(total, free)`.
    pub disk: Option<(u64, u64)>,
    /// Bytes `(received, sent)` over every interface but loopback.
    pub net: Option<(u64, u64)>,
    /// This process's open sockets.
    pub sockets: Option<u32>,
    /// The host's uptime in seconds.
    pub host_uptime: Option<u64>,
    /// The engine processes on this machine.
    pub procs: Vec<ProcReading>,
}

/// One engine process in a sample.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct GameSample {
    pub game_id: String,
    pub pid: u32,
    /// Cores used over the interval (0.5 is half a core); `None` on the
    /// first sample of a process, when there is no previous reading.
    pub cpu: Option<f32>,
    /// Resident memory, in bytes.
    pub rss: u64,
}

/// One point of the ring.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Sample {
    /// Unix seconds.
    pub at: u64,
    /// Share of all cores busy over the interval, 0 to 1.
    pub cpu: Option<f32>,
    /// The 1-minute load average.
    pub load1: Option<f32>,
    /// Bytes in use (total less available).
    pub mem_used: Option<u64>,
    /// Bytes in use on the disk.
    pub disk_used: Option<u64>,
    /// Bytes per second received.
    pub net_in: Option<f64>,
    /// Bytes per second sent.
    pub net_out: Option<f64>,
    pub sockets: Option<u32>,
    /// Requests per second on the public routes.
    pub requests: f64,
    pub games: Vec<GameSample>,
}

/// What does not change while the process runs.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Facts {
    pub platform: &'static str,
    pub cores: usize,
    /// Clock ticks per second, the unit of a process's CPU time.
    pub clock_ticks: u64,
}

/// The previous tick's raw values.
struct Last {
    mono: Instant,
    reading: Reading,
    requests: u64,
}

struct Inner {
    last: Option<Last>,
    ring: VecDeque<Sample>,
    /// The latest reading's gauges that the ring does not keep.
    mem_total: Option<u64>,
    disk_total: Option<u64>,
    host_uptime: Option<u64>,
    load: Option<[f32; 3]>,
}

/// The sampler's state: the ring and the request counter.
pub struct Metrics {
    /// Every request the public router served; bumped by [`count`].
    requests: AtomicU64,
    facts: Facts,
    inner: Mutex<Inner>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(Facts {
            platform: std::env::consts::OS,
            cores: std::thread::available_parallelism().map_or(1, usize::from),
            clock_ticks: clock_ticks(),
        })
    }
}

impl Metrics {
    fn new(facts: Facts) -> Self {
        Self {
            requests: AtomicU64::new(0),
            facts,
            inner: Mutex::new(Inner {
                last: None,
                ring: VecDeque::with_capacity(KEEP),
                mem_total: None,
                disk_total: None,
                host_uptime: None,
                load: None,
            }),
        }
    }

    /// One more request served.
    pub fn served(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Takes `reading` as the sample at `at` (unix seconds), `mono` after
    /// the previous one.
    pub fn tick(&self, reading: Reading, at: u64, mono: Instant) {
        let requests = self.requests.load(Ordering::Relaxed);
        let mut inner = self.inner.lock();
        let sample = derive(
            inner.last.as_ref(),
            &reading,
            at,
            mono,
            requests,
            self.facts.clock_ticks,
        );
        inner.mem_total = reading.mem.map(|(total, _)| total);
        inner.disk_total = reading.disk.map(|(total, _)| total);
        inner.host_uptime = reading.host_uptime;
        inner.load = reading.load;
        if inner.ring.len() == KEEP {
            inner.ring.pop_front();
        }
        inner.ring.push_back(sample);
        inner.last = Some(Last {
            mono,
            reading,
            requests,
        });
    }

    /// The answer of `GET /admin/metrics`.
    pub fn snapshot(&self, at: &str, process_uptime_secs: u64) -> serde_json::Value {
        let inner = self.inner.lock();
        let ring = &inner.ring;
        let column = |pick: fn(&Sample) -> serde_json::Value| -> Vec<serde_json::Value> {
            ring.iter().map(pick).collect()
        };
        let now = ring.back();
        let games: Vec<serde_json::Value> = now
            .map(|latest| {
                latest
                    .games
                    .iter()
                    .map(|game| {
                        fn same<'a>(s: &'a Sample, game: &GameSample) -> Option<&'a GameSample> {
                            s.games
                                .iter()
                                .find(|g| g.game_id == game.game_id && g.pid == game.pid)
                        }
                        let cpu: Vec<Option<f32>> = ring
                            .iter()
                            .map(|s| same(s, game).and_then(|g| g.cpu))
                            .collect();
                        let rss: Vec<Option<u64>> =
                            ring.iter().map(|s| same(s, game).map(|g| g.rss)).collect();
                        serde_json::json!({
                            "game_id": game.game_id,
                            "pid": game.pid,
                            "cpu": cpu,
                            "rss": rss,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        serde_json::json!({
            "at": at,
            "interval_secs": INTERVAL.as_secs(),
            "keep": KEEP,
            "platform": self.facts.platform,
            "cores": self.facts.cores,
            "process_uptime_secs": process_uptime_secs,
            "host": {
                "uptime_secs": inner.host_uptime,
                "mem_total": inner.mem_total,
                "disk_total": inner.disk_total,
                "disk_path": DISK_PATH,
                "load": inner.load,
            },
            "now": now,
            "history": {
                "at": column(|s| s.at.into()),
                "cpu": column(|s| s.cpu.into()),
                "load1": column(|s| s.load1.into()),
                "mem_used": column(|s| s.mem_used.into()),
                "disk_used": column(|s| s.disk_used.into()),
                "net_in": column(|s| s.net_in.into()),
                "net_out": column(|s| s.net_out.into()),
                "sockets": column(|s| s.sockets.into()),
                "requests": column(|s| s.requests.into()),
            },
            "games": games,
        })
    }
}

/// A counter as a float: a 53-bit mantissa holds any rate a server sees.
#[allow(clippy::cast_precision_loss)]
fn float(n: u64) -> f64 {
    n as f64
}

/// A rate from two counter readings, zero when the counter went backwards.
fn rate(before: u64, after: u64, secs: f64) -> f64 {
    if secs <= 0.0 {
        return 0.0;
    }
    float(after.saturating_sub(before)) / secs
}

/// The sample `reading` makes, given the previous tick.
fn derive(
    last: Option<&Last>,
    reading: &Reading,
    at: u64,
    mono: Instant,
    requests: u64,
    clock_ticks: u64,
) -> Sample {
    let secs = last.map_or(0.0, |l| {
        mono.saturating_duration_since(l.mono).as_secs_f64()
    });
    let prev = last.map(|l| &l.reading);
    let cpu = match (prev.and_then(|p| p.cpu), reading.cpu) {
        (Some((busy0, total0)), Some((busy1, total1))) if total1 > total0 => {
            Some((float(busy1.saturating_sub(busy0)) / float(total1 - total0)) as f32)
        }
        _ => None,
    };
    let net = match (prev.and_then(|p| p.net), reading.net) {
        (Some((rx0, tx0)), Some((rx1, tx1))) if secs > 0.0 => {
            Some((rate(rx0, rx1, secs), rate(tx0, tx1, secs)))
        }
        _ => None,
    };
    let games = reading
        .procs
        .iter()
        .map(|proc_| {
            let before = prev.and_then(|p| {
                p.procs
                    .iter()
                    .find(|q| q.pid == proc_.pid && q.game_id == proc_.game_id)
            });
            let cpu = before
                .filter(|_| secs > 0.0 && clock_ticks > 0)
                .map(|b| (rate(b.ticks, proc_.ticks, secs) / float(clock_ticks)) as f32);
            GameSample {
                game_id: proc_.game_id.clone(),
                pid: proc_.pid,
                cpu,
                rss: proc_.rss,
            }
        })
        .collect();
    Sample {
        at,
        cpu,
        load1: reading.load.map(|l| l[0]),
        mem_used: reading
            .mem
            .map(|(total, available)| total.saturating_sub(available)),
        disk_used: reading.disk.map(|(total, free)| total.saturating_sub(free)),
        net_in: net.map(|(rx, _)| rx),
        net_out: net.map(|(_, tx)| tx),
        sockets: reading.sockets,
        requests: last.map_or(0.0, |l| rate(l.requests, requests, secs)),
        games,
    }
}

/// Counts every request the public router serves; the admin console's own
/// listener is not behind it.
pub async fn count(
    axum::extract::State(state): axum::extract::State<crate::Shared>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    state.metrics.served();
    next.run(request).await
}

/// Samples every [`INTERVAL`] for as long as the process runs.
pub fn spawn(state: crate::Shared) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let games = local_engines(&state);
            let reading = tokio::task::spawn_blocking(move || read(&games))
                .await
                .unwrap_or_default();
            state
                .metrics
                .tick(reading, crate::auth::now_secs(), Instant::now());
        }
    });
}

/// The running games whose engine is a process on this machine.
fn local_engines(state: &crate::AppState) -> Vec<(String, u32)> {
    let lobby = state.lobby.lock();
    lobby
        .running()
        .filter(|game| game.engine_local)
        .filter_map(|game| game.engine_pid.map(|pid| (game.id.clone(), pid)))
        .collect()
}

/// Clock ticks per second, the unit of `/proc/<pid>/stat`'s times.
fn clock_ticks() -> u64 {
    #[cfg(unix)]
    {
        rustix::param::clock_ticks_per_second()
    }
    #[cfg(not(unix))]
    {
        100
    }
}

/// The page size, the unit of `/proc/<pid>/stat`'s resident set.
#[cfg(target_os = "linux")]
fn page_size() -> u64 {
    rustix::param::page_size() as u64
}

/// The disk at [`DISK_PATH`], in bytes `(total, free)`, as this process
/// could fill it.
#[cfg(unix)]
fn disk() -> Option<(u64, u64)> {
    let fs = rustix::fs::statvfs(DISK_PATH).ok()?;
    let frag = fs.f_frsize;
    Some((
        fs.f_blocks.saturating_mul(frag),
        fs.f_bavail.saturating_mul(frag),
    ))
}

#[cfg(not(unix))]
fn disk() -> Option<(u64, u64)> {
    None
}

/// Everything there is to read, right now.
#[cfg(target_os = "linux")]
fn read(games: &[(String, u32)]) -> Reading {
    let text = |path: &str| std::fs::read_to_string(path).ok();
    let page = page_size();
    Reading {
        cpu: text("/proc/stat").as_deref().and_then(cpu_of),
        load: text("/proc/loadavg").as_deref().and_then(load_of),
        mem: text("/proc/meminfo").as_deref().and_then(mem_of),
        disk: disk(),
        net: text("/proc/net/dev").as_deref().and_then(net_of),
        sockets: sockets_of_self(),
        host_uptime: text("/proc/uptime").as_deref().and_then(uptime_of),
        procs: games
            .iter()
            .filter_map(|(game_id, pid)| {
                let (ticks, rss_pages) = text(&format!("/proc/{pid}/stat"))
                    .as_deref()
                    .and_then(proc_of)?;
                Some(ProcReading {
                    game_id: game_id.clone(),
                    pid: *pid,
                    ticks,
                    rss: rss_pages.saturating_mul(page),
                })
            })
            .collect(),
    }
}

/// On anything but Linux: the disk, and nothing off `/proc`.
#[cfg(not(target_os = "linux"))]
fn read(_games: &[(String, u32)]) -> Reading {
    Reading {
        disk: disk(),
        ..Reading::default()
    }
}

/// How many of this process's descriptors are sockets.
#[cfg(target_os = "linux")]
fn sockets_of_self() -> Option<u32> {
    let entries = std::fs::read_dir("/proc/self/fd").ok()?;
    let sockets = entries
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_link(entry.path()).ok())
        .filter(|target| target.to_string_lossy().starts_with("socket:["))
        .count();
    u32::try_from(sockets).ok()
}

/// `/proc/stat`'s first line: `(busy, total)` jiffies over every core.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn cpu_of(stat: &str) -> Option<(u64, u64)> {
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map_while(|f| f.parse().ok())
        .collect();
    // user nice system idle iowait irq softirq steal …
    if fields.len() < 4 {
        return None;
    }
    let total: u64 = fields.iter().take(8).sum();
    let idle = fields[3] + fields.get(4).copied().unwrap_or(0);
    Some((total.saturating_sub(idle), total))
}

/// `/proc/loadavg`: the three averages.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn load_of(loadavg: &str) -> Option<[f32; 3]> {
    let mut parts = loadavg.split_whitespace();
    let mut next = || parts.next()?.parse::<f32>().ok();
    Some([next()?, next()?, next()?])
}

/// `/proc/meminfo`: `(MemTotal, MemAvailable)` in bytes.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn mem_of(meminfo: &str) -> Option<(u64, u64)> {
    let kib = |key: &str| -> Option<u64> {
        meminfo
            .lines()
            .find(|l| l.starts_with(key))?
            .split_whitespace()
            .nth(1)?
            .parse::<u64>()
            .ok()
    };
    Some((kib("MemTotal:")? * 1024, kib("MemAvailable:")? * 1024))
}

/// `/proc/net/dev`: bytes `(received, sent)` over every interface but `lo`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn net_of(dev: &str) -> Option<(u64, u64)> {
    let mut seen = false;
    let (mut rx, mut tx) = (0u64, 0u64);
    for line in dev.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        if name.trim() == "lo" {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        let (Some(r), Some(t)) = (fields.first(), fields.get(8)) else {
            continue;
        };
        rx = rx.saturating_add(r.parse().ok()?);
        tx = tx.saturating_add(t.parse().ok()?);
        seen = true;
    }
    seen.then_some((rx, tx))
}

/// `/proc/uptime`: seconds since boot.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn uptime_of(uptime: &str) -> Option<u64> {
    let secs: f64 = uptime.split_whitespace().next()?.parse().ok()?;
    Some(secs as u64)
}

/// `/proc/<pid>/stat`: `(utime + stime, rss)` in ticks and pages. The
/// command name in parentheses may hold a space or a `)`, so the fields are
/// counted from the last `)`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn proc_of(stat: &str) -> Option<(u64, u64)> {
    let (_, rest) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // `rest` starts at field 3 (state); utime is field 14, stime 15, rss 24.
    let at = |n: usize| -> Option<u64> { fields.get(n - 3)?.parse().ok() };
    Some((at(14)? + at(15)?, at(24)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &str = "cpu  1000 50 300 8000 200 0 10 0 0 0\ncpu0 500 25 150 4000 100 0 5 0 0 0\n";
    const MEMINFO: &str =
        "MemTotal:        8000000 kB\nMemFree:          100000 kB\nMemAvailable:    6000000 kB\n";
    const NET: &str = "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n    lo: 999999   10    0    0    0     0          0         0   999999   10    0    0    0     0       0          0\n  eth0: 1000   10    0    0    0     0          0         0   2000   10    0    0    0     0       0          0\n wlan0:  500    5    0    0    0     0          0         0    700    5    0    0    0     0       0          0\n";
    const PID_STAT: &str = "4242 (baylee-engine (x)) S 1 4242 4242 0 -1 4194560 1234 0 0 0 150 50 0 0 20 0 9 0 12345 123456789 2048 18446744073709551615 1 1 0 0 0 0 0 0 0 0 0 0 17 3 0 0 0 0 0 0 0 0 0 0 0 0 0";

    #[test]
    fn the_proc_files_are_read_as_the_kernel_writes_them() {
        assert_eq!(cpu_of(STAT), Some((1360, 9560)));
        assert_eq!(
            load_of("0.52 0.40 0.31 1/234 5678\n"),
            Some([0.52, 0.40, 0.31])
        );
        assert_eq!(mem_of(MEMINFO), Some((8_192_000_000, 6_144_000_000)));
        assert_eq!(net_of(NET), Some((1500, 2700)));
        assert_eq!(uptime_of("12345.67 23456.78\n"), Some(12345));
        // The command name holds a space and a `)`: fields are counted from
        // the last one.
        assert_eq!(proc_of(PID_STAT), Some((200, 2048)));
    }

    #[test]
    fn a_file_that_is_not_what_it_should_be_reads_as_nothing() {
        assert_eq!(cpu_of("intr 1 2 3\n"), None);
        assert_eq!(cpu_of("cpu 1 2\n"), None);
        assert_eq!(load_of("x y z"), None);
        assert_eq!(mem_of("MemTotal: 10 kB\n"), None);
        assert_eq!(net_of("Inter\n face\n"), None);
        assert_eq!(proc_of("4242 (short) S 1"), None);
        assert_eq!(proc_of(""), None);
    }

    fn reading(busy: u64, total: u64, rx: u64, tx: u64, ticks: u64) -> Reading {
        Reading {
            cpu: Some((busy, total)),
            load: Some([1.5, 1.0, 0.5]),
            mem: Some((1000, 400)),
            disk: Some((5000, 1000)),
            net: Some((rx, tx)),
            sockets: Some(7),
            host_uptime: Some(99),
            procs: vec![ProcReading {
                game_id: "g1".into(),
                pid: 10,
                ticks,
                rss: 4096,
            }],
        }
    }

    /// Rates come from the difference between two ticks; the first tick
    /// has none, and a counter that went backwards is a zero rate.
    #[test]
    fn rates_are_differences_and_the_first_tick_has_none() {
        let m = Metrics::new(Facts {
            platform: "test",
            cores: 2,
            clock_ticks: 100,
        });
        let t0 = Instant::now();
        m.tick(reading(100, 1000, 1000, 2000, 500), 1_700_000_000, t0);
        {
            let inner = m.inner.lock();
            let first = inner.ring.back().unwrap();
            assert_eq!(first.cpu, None);
            assert_eq!(first.net_in, None);
            assert_eq!(first.requests, 0.0);
            assert_eq!(first.games[0].cpu, None);
            assert_eq!(first.games[0].rss, 4096);
            assert_eq!(first.mem_used, Some(600));
            assert_eq!(first.disk_used, Some(4000));
            assert_eq!(first.load1, Some(1.5));
        }
        for _ in 0..30 {
            m.served();
        }
        // Ten seconds later: 450 of 900 jiffies busy, 5000 bytes in,
        // 1000 out, 250 ticks of one process, 30 requests.
        let t1 = t0 + Duration::from_secs(10);
        m.tick(reading(550, 1900, 6000, 3000, 750), 1_700_000_010, t1);
        let snap = m.snapshot("now", 10);
        let now = &snap["now"];
        assert!((now["cpu"].as_f64().unwrap() - 0.5).abs() < 1e-6, "{now}");
        assert!((now["net_in"].as_f64().unwrap() - 500.0).abs() < 1e-9);
        assert!((now["net_out"].as_f64().unwrap() - 100.0).abs() < 1e-9);
        assert!((now["requests"].as_f64().unwrap() - 3.0).abs() < 1e-9);
        assert!((now["games"][0]["cpu"].as_f64().unwrap() - 0.25).abs() < 1e-6);
        assert_eq!(snap["games"][0]["game_id"], "g1");
        assert_eq!(snap["games"][0]["cpu"][0], serde_json::Value::Null);
        assert_eq!(snap["games"][0]["rss"][1], 4096);
        assert_eq!(snap["history"]["at"].as_array().unwrap().len(), 2);
        assert_eq!(snap["host"]["mem_total"], 1000);
        assert_eq!(snap["host"]["disk_total"], 5000);
        assert_eq!(snap["cores"], 2);
        assert_eq!(snap["interval_secs"], 5);
        // Counters that went backwards (a reused pid, a reset interface).
        let t2 = t1 + Duration::from_secs(10);
        m.tick(reading(100, 2900, 10, 10, 1), 1_700_000_020, t2);
        let snap = m.snapshot("now", 20);
        assert_eq!(snap["now"]["net_in"], 0.0);
        assert_eq!(snap["now"]["games"][0]["cpu"], 0.0);
        assert_eq!(snap["now"]["cpu"], 0.0);
    }

    /// The ring keeps the last hour and no more.
    #[test]
    fn the_ring_holds_an_hour() {
        let m = Metrics::new(Facts {
            platform: "test",
            cores: 1,
            clock_ticks: 100,
        });
        let t0 = Instant::now();
        for i in 0..(KEEP as u64 + 5) {
            m.tick(
                Reading::default(),
                i,
                t0 + Duration::from_secs(i * INTERVAL.as_secs()),
            );
        }
        let snap = m.snapshot("now", 0);
        let at = snap["history"]["at"].as_array().unwrap();
        assert_eq!(at.len(), KEEP);
        assert_eq!(at[0], 5);
        assert_eq!(snap["now"]["cpu"], serde_json::Value::Null);
        assert_eq!(snap["games"], serde_json::json!([]));
    }

    /// Reading this machine answers something on every platform and
    /// nothing from `/proc` where there is none.
    #[test]
    fn reading_this_machine_degrades_rather_than_fails() {
        let got = read(&[("g".into(), std::process::id())]);
        if cfg!(target_os = "linux") {
            assert!(got.cpu.is_some());
            assert!(got.mem.is_some());
            assert!(got.load.is_some());
            assert!(got.host_uptime.is_some());
            assert_eq!(got.procs.len(), 1, "this process read off /proc");
            assert!(got.procs[0].rss > 0);
        } else {
            assert_eq!(got.cpu, None);
            assert!(got.procs.is_empty());
        }
        if cfg!(unix) {
            let (total, free) = got.disk.expect("statvfs on /");
            assert!(total >= free);
        }
    }
}
