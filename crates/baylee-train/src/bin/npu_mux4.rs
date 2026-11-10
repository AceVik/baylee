//! `npu_mux4` — NPU Session-Multiplexer & Tiered RAM/SSD Session Pager benchmark & runner.
//!
//! Runs `N` concurrent Baylee matches multiplexed over `M` resident neural worker slots,
//! dispatching parallel waves (`observe_wave` / `act_wave`) across all `M` worker slots
//! simultaneously while pipelining keyed (masked, not encrypted) SSD prefetching
//! for the next wave and performing live zero-downtime key rotation.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Context as _;
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_train::housedeck::HouseDeck;
use baylee_train::netplay4::{BatchLimits4, PolicyMode4};
use baylee_train::npu_mux4::{
    AcceleratorKind, ActWaveItem4, HardwareConfig, NetMuxServiceExt as _, NpuMuxService4,
    ObserveWaveItem4, SessionUrgency4,
};
use baylee_train::selfplay::table_for;
use baylee_view::PlayerView;
use clap::Parser;

const VICTORY_DECK_TEXT: &str = include_str!("../../../../data/decks/victory.txt");
const ALLYTIFACT_DECK_TEXT: &str = include_str!("../../../../data/decks/allytifact.txt");

#[derive(Parser, Debug)]
#[command(
    name = "npu_mux4",
    about = "Multiplex hundreds of concurrent Baylee sessions over fixed neural worker slots with keyed SSD paging"
)]
struct Cli {
    /// Path to the Playernet v4/v5 ONNX model directory.
    #[arg(long)]
    model: PathBuf,

    /// Directory for keyed (masked, not encrypted) cold-tier SSD `.swap` files (`0700`).
    #[arg(long, default_value = "/tmp/baylee-npu-mux4-swap")]
    swap_dir: PathBuf,

    /// Number of resident neural worker slots kept in memory (`M`).
    #[arg(long, default_value_t = 4)]
    worker_slots: usize,

    /// Total concurrent game sessions to run simultaneously (`N`).
    #[arg(long, default_value_t = 128)]
    concurrent_sessions: usize,

    /// Maximum sessions held in Hot RAM before paging out to SSD (`< N` exercises SSD paging).
    #[arg(long, default_value_t = 32)]
    max_hot_sessions: usize,

    /// Maximum Hot RAM budget in MiB for session snapshots.
    #[arg(long, default_value_t = 64)]
    max_hot_ram_mib: usize,

    /// Number of decision steps to advance per session during the multiplexed run.
    #[arg(long, default_value_t = 16)]
    steps_per_session: usize,

    /// Batch size limit for the shared neural backend.
    #[arg(long, default_value_t = 64)]
    inference_batch: usize,

    /// Latent thinking depth `K` (`1, 2, 4, 8`).
    #[arg(long)]
    thinking_steps: Option<u8>,

    /// Target hardware execution provider (`auto`, `openvino`, `coreml`, `directml`, `cuda`, `cpu`).
    #[arg(long, default_value = "auto")]
    accelerator: AcceleratorKind,

    /// Intra-op threads per inference session (`0` = automatic topology sizing).
    #[arg(long, default_value_t = 0)]
    intra_threads: usize,

    /// Optional path to write the JSON summary report.
    #[arg(long)]
    json_out: Option<PathBuf>,
}

struct LiveTable {
    session_id: u64,
    engine: Session,
    decisions_done: usize,
    finished: bool,
}

struct WaveReady {
    table_idx: usize,
    session_id: u64,
    actor: PlayerId,
    pending: Pending,
    act_view: PlayerView,
    obs_views: [PlayerView; 2],
    post_urgency: SessionUrgency4,
}

fn dispatch_ready_wave(
    mux: &mut NpuMuxService4,
    tables: &mut [LiveTable],
    ready: &[WaveReady],
    next_prefetch: &[u64],
) -> anyhow::Result<()> {
    let teams: [Option<u8>; 2] = [None, None];
    let obs0_items: Vec<ObserveWaveItem4<'_>> = ready
        .iter()
        .map(|r| ObserveWaveItem4 {
            session_id: r.session_id,
            view: &r.obs_views[0],
            post_urgency: SessionUrgency4::Immediate,
        })
        .collect();
    mux.observe_wave(&obs0_items, next_prefetch)?;

    let obs1_items: Vec<ObserveWaveItem4<'_>> = ready
        .iter()
        .map(|r| ObserveWaveItem4 {
            session_id: r.session_id,
            view: &r.obs_views[1],
            post_urgency: SessionUrgency4::Immediate,
        })
        .collect();
    mux.observe_wave(&obs1_items, &[])?;

    let act_items: Vec<ActWaveItem4<'_>> = ready
        .iter()
        .map(|r| ActWaveItem4 {
            session_id: r.session_id,
            view: &r.act_view,
            pending: &r.pending,
            teams: &teams[..],
            post_urgency: r.post_urgency,
        })
        .collect();
    let answers = mux.act_wave(&act_items, &[])?;

    for (r, (_, ans)) in ready.iter().zip(answers) {
        let table = &mut tables[r.table_idx];
        if table.engine.act(r.actor, ans.action).is_err() {
            table.finished = true;
        } else {
            table.decisions_done += 1;
        }
    }
    Ok(())
}

fn step_all_tables(
    mux: &mut NpuMuxService4,
    tables: &mut [LiveTable],
    worker_slots: usize,
    steps_per_session: usize,
) -> anyhow::Result<()> {
    let wave_width = worker_slots.max(1);
    let first_wave: Vec<u64> = tables
        .iter()
        .take(wave_width)
        .map(|t| t.session_id)
        .collect();
    let _ = mux.prefetch_batch(&first_wave);

    for round in 0..steps_per_session {
        if round > 0 && round == steps_per_session / 2 {
            let _ = mux.rotate_swap_key()?;
        }
        let num_tables = tables.len();
        let mut start = 0;
        while start < num_tables {
            let end = (start + wave_width).min(num_tables);
            let next_start = end % num_tables;
            let next_end = (next_start + wave_width).min(num_tables);
            let next_prefetch: Vec<u64> = tables[next_start..next_end]
                .iter()
                .filter(|t| !t.finished)
                .map(|t| t.session_id)
                .collect();

            let mut ready = Vec::with_capacity(end - start);
            for (offset, table) in tables[start..end].iter_mut().enumerate() {
                let table_idx = start + offset;
                if table.finished || matches!(table.engine.pending(), Pending::GameOver(_)) {
                    table.finished = true;
                    continue;
                }
                let Some((actor, pending, act_view)) = [PlayerId::new(0), PlayerId::new(1)]
                    .into_iter()
                    .find_map(|pid| {
                        table
                            .engine
                            .view_for_with_context(pid)
                            .map(|(p, v)| (pid, p, v))
                    })
                else {
                    table.finished = true;
                    continue;
                };
                let obs0 = table
                    .engine
                    .observe_view_with_payment_context(PlayerId::new(0));
                let obs1 = table
                    .engine
                    .observe_view_with_payment_context(PlayerId::new(1));
                let post_urgency = if (round + table_idx).is_multiple_of(3) {
                    SessionUrgency4::Idle
                } else {
                    SessionUrgency4::Soon
                };
                ready.push(WaveReady {
                    table_idx,
                    session_id: table.session_id,
                    actor,
                    pending,
                    act_view,
                    obs_views: [obs0, obs1],
                    post_urgency,
                });
            }

            if !ready.is_empty() {
                dispatch_ready_wave(mux, tables, &ready, &next_prefetch)?;
            }
            start = end;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let max_hot_ram_bytes = cli.max_hot_ram_mib.saturating_mul(1024 * 1024);
    let hw = HardwareConfig {
        accelerator: cli.accelerator,
        intra_threads: cli.intra_threads,
        ..HardwareConfig::default()
    };

    let t_start = Instant::now();
    let mut mux = NpuMuxService4::open_with_hardware(
        &cli.model,
        cli.swap_dir.clone(),
        cli.worker_slots,
        cli.max_hot_sessions,
        max_hot_ram_bytes,
        BatchLimits4::for_rows(cli.inference_batch),
        PolicyMode4::Tactical,
        0.0,
        cli.thinking_steps,
        hw.clone(),
    )
    .context("initializing NpuMuxService4")?;
    let init_ms = t_start.elapsed().as_millis();

    let deck_a = HouseDeck::parse("victory", VICTORY_DECK_TEXT).context("parsing victory deck")?;
    let deck_b =
        HouseDeck::parse("allytifact", ALLYTIFACT_DECK_TEXT).context("parsing allytifact deck")?;

    let mut tables = Vec::with_capacity(cli.concurrent_sessions);
    for idx in 0..cli.concurrent_sessions {
        let session_id = (idx + 1) as u64;
        let preset = table_for(
            10_000 + session_id,
            &[&deck_a, &deck_b],
            &[AIProfile::STEADY; 2],
            &[],
        );
        let mut engine = Session::new(&preset).context("creating gamehost Session")?;
        engine.take_over(PlayerId::new(0));
        engine.take_over(PlayerId::new(1));
        let urgency = if idx < cli.max_hot_sessions / 2 {
            SessionUrgency4::Soon
        } else {
            SessionUrgency4::Idle
        };
        mux.start_session(session_id, &preset, 50_000 + session_id, urgency)?;
        tables.push(LiveTable {
            session_id,
            engine,
            decisions_done: 0,
            finished: false,
        });
    }

    let t_run = Instant::now();
    step_all_tables(
        &mut mux,
        &mut tables,
        cli.worker_slots,
        cli.steps_per_session,
    )?;
    let run_elapsed = t_run.elapsed();
    let summary = mux.summary();

    #[allow(clippy::cast_precision_loss)]
    let report = serde_json::json!({
        "model": cli.model,
        "resolved_model": hw.resolve_model_dir(&cli.model),
        "accelerator": hw.effective_accelerator().as_str(),
        "intra_threads": hw.recommended_intra_threads(cli.worker_slots),
        "init_ms": init_ms,
        "run_ms": run_elapsed.as_millis(),
        "decisions_per_sec": (summary.decisions_routed as f64) / run_elapsed.as_secs_f64().max(1e-6),
        "observations_per_sec": (summary.observations_routed as f64) / run_elapsed.as_secs_f64().max(1e-6),
        "summary": summary,
    });

    if let Some(out_path) = &cli.json_out {
        std::fs::write(out_path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("writing {}", out_path.display()))?;
    }
    println!("{}", serde_json::to_string_pretty(&report)?);

    for table in &tables {
        mux.close_session(table.session_id);
    }
    Ok(())
}
