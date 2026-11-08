//! Telemetry and latency metrics for the NPU session multiplexer and RAM/SSD pager.

use serde::{Deserialize, Serialize};

/// Cumulative operational and security counters for [`super::NpuMuxService4`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuxMetrics4 {
    /// Total sessions ever registered with the multiplexer.
    pub sessions_created: u64,
    /// Total sessions explicitly closed and wiped.
    pub sessions_closed: u64,
    /// Total worker context switches executed (`restore` + step + `snapshot`).
    pub context_switches: u64,
    /// Decisions served directly from Hot RAM without touching SSD.
    pub hot_ram_hits: u64,
    /// Cold SSD sessions paged in on demand at decision/observation time.
    pub cold_ssd_page_ins: u64,
    /// Cold SSD sessions paged in proactively via `prefetch` before their turn.
    pub prefetch_page_ins: u64,
    /// Calls to `prefetch` where the session was already resident in Hot RAM.
    pub prefetch_already_hot: u64,
    /// Hot RAM sessions evicted and sealed to SSD (`XChaCha20-Poly1305`).
    pub ssd_page_outs: u64,
    /// Cumulative compressed + encrypted bytes written to SSD across all page-outs.
    pub ssd_bytes_written: u64,
    /// Cumulative compressed + encrypted bytes read from SSD across all page-ins.
    pub ssd_bytes_read: u64,
    /// Cumulative tamper, rollback, symlink, or corrupt-snapshot rejections caught.
    pub security_rejections: u64,
    /// Total passive observations (`K = 0`) routed through worker slots.
    pub observations_routed: u64,
    /// Total active decisions (`act_detailed`) routed through worker slots.
    pub decisions_routed: u64,
    /// Microsecond latency samples of context switches (capped ring for p50/p95).
    #[serde(skip)]
    switch_latencies_us: Vec<u64>,
    /// Microsecond latency samples of SSD page-out operations.
    #[serde(skip)]
    page_out_latencies_us: Vec<u64>,
    /// Microsecond latency samples of SSD page-in operations.
    #[serde(skip)]
    page_in_latencies_us: Vec<u64>,
}

const MAX_LATENCY_SAMPLES: usize = 16_384;

impl MuxMetrics4 {
    pub(super) fn record_switch_us(&mut self, us: u64) {
        if self.switch_latencies_us.len() < MAX_LATENCY_SAMPLES {
            self.switch_latencies_us.push(us);
        }
    }

    pub(super) fn record_page_out_us(&mut self, us: u64) {
        if self.page_out_latencies_us.len() < MAX_LATENCY_SAMPLES {
            self.page_out_latencies_us.push(us);
        }
    }

    pub(super) fn record_page_in_us(&mut self, us: u64) {
        if self.page_in_latencies_us.len() < MAX_LATENCY_SAMPLES {
            self.page_in_latencies_us.push(us);
        }
    }

    /// Computes a point-in-time summary combining live pool state and latency percentiles.
    #[must_use]
    pub fn summarize(
        &self,
        worker_slots: usize,
        hot_sessions: usize,
        cold_sessions: usize,
        hot_ram_bytes: usize,
        cold_ssd_bytes: u64,
    ) -> MuxSummary4 {
        MuxSummary4 {
            worker_slots,
            hot_sessions,
            cold_sessions,
            hot_ram_bytes,
            cold_ssd_bytes,
            sessions_created: self.sessions_created,
            sessions_closed: self.sessions_closed,
            context_switches: self.context_switches,
            hot_ram_hits: self.hot_ram_hits,
            cold_ssd_page_ins: self.cold_ssd_page_ins,
            prefetch_page_ins: self.prefetch_page_ins,
            prefetch_already_hot: self.prefetch_already_hot,
            ssd_page_outs: self.ssd_page_outs,
            ssd_bytes_written: self.ssd_bytes_written,
            ssd_bytes_read: self.ssd_bytes_read,
            security_rejections: self.security_rejections,
            observations_routed: self.observations_routed,
            decisions_routed: self.decisions_routed,
            switch_p50_us: percentile(&self.switch_latencies_us, 50),
            switch_p95_us: percentile(&self.switch_latencies_us, 95),
            page_out_p50_us: percentile(&self.page_out_latencies_us, 50),
            page_out_p95_us: percentile(&self.page_out_latencies_us, 95),
            page_in_p50_us: percentile(&self.page_in_latencies_us, 50),
            page_in_p95_us: percentile(&self.page_in_latencies_us, 95),
        }
    }
}

/// Serializable snapshot of multiplexer state, paging counters, and latency percentiles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuxSummary4 {
    /// Number of resident neural worker slots.
    pub worker_slots: usize,
    /// Sessions currently resident in Hot RAM.
    pub hot_sessions: usize,
    /// Sessions currently swapped out to Cold SSD.
    pub cold_sessions: usize,
    /// Current approximate Hot RAM usage of resident session snapshots (bytes).
    pub hot_ram_bytes: usize,
    /// Current on-disk usage of active Cold SSD swap files (bytes).
    pub cold_ssd_bytes: u64,
    /// Total sessions created.
    pub sessions_created: u64,
    /// Total sessions closed.
    pub sessions_closed: u64,
    /// Total neural context switches executed.
    pub context_switches: u64,
    /// Decisions/observations served from Hot RAM.
    pub hot_ram_hits: u64,
    /// On-demand Cold SSD page-ins.
    pub cold_ssd_page_ins: u64,
    /// Proactive `prefetch` Cold SSD page-ins.
    pub prefetch_page_ins: u64,
    /// `prefetch` calls that found the session already in Hot RAM.
    pub prefetch_already_hot: u64,
    /// Total Hot-to-Cold SSD evictions.
    pub ssd_page_outs: u64,
    /// Total encrypted bytes written to SSD.
    pub ssd_bytes_written: u64,
    /// Total encrypted bytes read from SSD.
    pub ssd_bytes_read: u64,
    /// Total cryptographic, rollback, or validation rejections.
    pub security_rejections: u64,
    /// Total passive observations routed.
    pub observations_routed: u64,
    /// Total active decisions routed.
    pub decisions_routed: u64,
    /// Median context-switch latency (microseconds).
    pub switch_p50_us: u64,
    /// p95 context-switch latency (microseconds).
    pub switch_p95_us: u64,
    /// Median SSD page-out latency (microseconds).
    pub page_out_p50_us: u64,
    /// p95 SSD page-out latency (microseconds).
    pub page_out_p95_us: u64,
    /// Median SSD page-in latency (microseconds).
    pub page_in_p50_us: u64,
    /// p95 SSD page-in latency (microseconds).
    pub page_in_p95_us: u64,
}

fn percentile(samples: &[u64], pct: usize) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let idx = ((sorted.len() - 1) * pct) / 100;
    sorted[idx]
}
