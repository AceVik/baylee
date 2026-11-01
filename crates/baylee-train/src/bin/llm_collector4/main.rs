//! Multi-table LLM self-play replay collector and single-gateway/single-agent host supervisor.
mod admin_cleanup;
mod audit;
mod compare;
mod processes;
mod quota_hold;
mod schedule;
mod table;
mod teacher;

use std::io::Read as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail, ensure};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore};

use crate::audit::write_atomic_json;
use crate::schedule::{Scheduler, now_unix};
use crate::table::{GatewayClient, LaunchPacer, SlotAccount, run_attempt};
use crate::teacher::Sits;

#[derive(Parser)]
#[command(
    about = "Multi-table LLM self-play collector over a single baylee-gateway and baylee-agent"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single high-capacity baylee-gateway + single baylee-agent serving many concurrent tables.
    Serve {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        bin_dir: PathBuf,
        #[arg(long, default_value = "127.0.0.1:28768")]
        bind: String,
        /// Optional loopback admin listener; shared environment token or generated memory-only secret.
        #[arg(long)]
        admin_bind: Option<String>,
        #[arg(
            long,
            default_value = "postgresql://ace@localhost/baylee_llm_teacher_20261028?host=/var/run/postgresql"
        )]
        database_url: String,
        #[arg(long, default_value_t = 96)]
        agent_capacity: usize,
        #[arg(long, default_value_t = 20)]
        db_pool: usize,
        #[arg(long, default_value_t = 32)]
        replay_jobs: usize,
    },
    /// Run N concurrent LLM self-play tables against a single gateway + agent.
    Collect {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value = "http://127.0.0.1:28768")]
        gateway: String,
        /// Dedicated loopback admin endpoint; token comes only from `BAYLEE_COLLECTOR_ADMIN_TOKEN`.
        #[arg(long)]
        admin_url: String,
        #[arg(
            long,
            default_value = "postgresql://ace@localhost/baylee_llm_teacher_20261028?host=/var/run/postgresql"
        )]
        database_url: String,
        #[arg(long)]
        seat_bin: PathBuf,
        #[arg(long)]
        gateway_bin: PathBuf,
        #[arg(long)]
        profiles_config: PathBuf,
        /// JSON array of `[seat0_profile, seat1_profile]` pairs, frozen at startup.
        /// An entry `house:<level>` (novice, casual, steady, sharp, expert)
        /// seats the house at that level instead of a profile, to measure a
        /// teacher against it; it reserves nothing and is never a teacher's
        /// answer. The house against the house is refused.
        #[arg(long)]
        pairings_file: Option<PathBuf>,
        #[arg(long)]
        deck_manifest: PathBuf,
        #[arg(long, default_value_t = 50)]
        concurrency: usize,
        /// Maximum assigned attempts for this controller run, including failures.
        #[arg(long)]
        max_attempts: Option<usize>,
        #[arg(long, default_value_t = 90)]
        call_timeout_seconds: u64,
        #[arg(long, default_value_t = 3600)]
        game_timeout_seconds: u64,
        #[arg(long, default_value_t = 2200)]
        launch_interval_ms: u64,
        /// Only the first this many games of the root file their seats'
        /// reports with the feedback service; later games keep them in
        /// `reports.jsonl`, so a large run cannot flood the triage.
        #[arg(long, default_value_t = 200)]
        report_games: usize,
    },
    /// Profile against profile over the admitted games of collector roots:
    /// wins with a Wilson 95 % interval, against the house apart, how often a
    /// fallback took over, and the minds' answers, think time and tokens.
    Compare {
        /// Collector roots, each holding `attempts/`.
        #[arg(required = true)]
        roots: Vec<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Serve {
            root,
            bin_dir,
            bind,
            admin_bind,
            database_url,
            agent_capacity,
            db_pool,
            replay_jobs,
        } => {
            run_serve(
                &root,
                &bin_dir,
                &bind,
                admin_bind.as_deref(),
                &database_url,
                agent_capacity,
                db_pool,
                replay_jobs,
            )
            .await
        }
        Commands::Collect {
            root,
            gateway,
            admin_url,
            database_url,
            seat_bin,
            gateway_bin,
            profiles_config,
            pairings_file,
            deck_manifest,
            concurrency,
            max_attempts,
            call_timeout_seconds,
            game_timeout_seconds,
            launch_interval_ms,
            report_games,
        } => {
            run_collect(
                root,
                gateway,
                admin_url,
                database_url,
                seat_bin,
                gateway_bin,
                profiles_config,
                pairings_file,
                deck_manifest,
                concurrency,
                max_attempts,
                call_timeout_seconds,
                game_timeout_seconds,
                launch_interval_ms,
                report_games,
            )
            .await
        }
        Commands::Compare { roots } => {
            print!("{}", compare::report(&roots)?);
            Ok(())
        }
    }
}

async fn kill_process_group(pgid: i32, child: &mut tokio::process::Child) -> anyhow::Result<()> {
    ensure!(
        child.id().and_then(|id| i32::try_from(id).ok()) == Some(pgid),
        "Owned process group identity lost"
    );
    processes::cleanup(child).await
}

fn controller_lock(directory: &Path) -> anyhow::Result<std::fs::File> {
    std::fs::create_dir_all(directory)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("controller.lock"))?;
    lock.try_lock()
        .context("Another controller already owns this root")?;
    Ok(lock)
}

/// The feedback service a gateway files its players' reports with
/// (`docs/feedback.md`): the LLM seats' reports, one bug report each,
/// reach it only when the collector was started with these. Only their
/// names are written here.
const FEEDBACK_ENV: [&str; 3] = [
    "BAYLEE_FEEDBACK_URL",
    "BAYLEE_FEEDBACK_TOKEN",
    "BAYLEE_FEEDBACK_KEY",
];

fn forward_feedback(command: &mut Command) {
    for name in FEEDBACK_ENV {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

/// The bug reports each account of this private gateway may file an hour
/// (`BAYLEE_REPORTS_PER_HOUR`, `docs/feedback.md`; 64 where it is not set,
/// as on the official gateway): the operator's choice for its own seat
/// accounts. Each language-model seat here files one bug report a report
/// its model makes, at most 25 a game (`baylee_seat::reports::MOST`), and
/// one teacher account plays many tables an hour, so 64 would turn most of
/// them away (`429`) and leave them only in the reports files. The same
/// budget for every account of the gateway, never one tied to what a
/// report says of itself.
const REPORTS_PER_HOUR: &str = "2000";

/// Gives `command`, the private gateway, its accounts' report budget
/// ([`REPORTS_PER_HOUR`]).
fn report_budget(command: &mut Command) {
    command.env("BAYLEE_REPORTS_PER_HOUR", REPORTS_PER_HOUR);
}

fn isolated_command(binary: &Path) -> Command {
    let mut command = Command::new(binary);
    command.env_clear();
    for name in ["PATH", "HOME", "USER", "LANG", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
}

struct AttemptBudget {
    limit: Option<usize>,
    assigned: AtomicUsize,
}

impl AttemptBudget {
    fn new(limit: Option<usize>) -> Self {
        Self {
            limit,
            assigned: AtomicUsize::new(0),
        }
    }

    fn reserve(&self) -> bool {
        self.assigned
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                if self.limit.is_some_and(|limit| count >= limit) {
                    None
                } else {
                    count.checked_add(1)
                }
            })
            .is_ok()
    }

    fn exhausted(&self) -> bool {
        self.limit
            .is_some_and(|limit| self.assigned.load(Ordering::SeqCst) >= limit)
    }
}

/// DRAIN is an assignment boundary only; active attempts receive STOP alone.
fn assignment_halted(
    stop: &AtomicBool,
    drain: &AtomicBool,
    stop_file: &Path,
    drain_file: &Path,
) -> bool {
    stop.load(Ordering::SeqCst)
        || drain.load(Ordering::SeqCst)
        || stop_file.exists()
        || drain_file.exists()
}

/// Slot `slot_id`'s account that seats `sits` ([`SlotAccount::init`]),
/// tried up to three times where a failure left nothing behind, a longer
/// pause before each try; `None` where assignment halts first. A failure
/// whose cleanup is unproved drains the run.
async fn slot_account(
    own: &teacher::OwnDatabase,
    gateway: &str,
    slot_id: usize,
    sits: Sits,
    halted: &(dyn Fn() -> bool + Sync),
    drain: &AtomicBool,
) -> anyhow::Result<Option<SlotAccount>> {
    for attempt in 1..=3 {
        if halted() {
            return Ok(None);
        }
        let error = match SlotAccount::init(own, gateway, slot_id, sits).await {
            Ok(account) => return Ok(Some(account)),
            Err(error) => error,
        };
        eprintln!(
            "[slot {slot_id:02}] {sits:?} account init attempt {attempt}/3 failed: {error:#}"
        );
        let retry_safe = table::account_init_retry_safe(&error);
        if !retry_safe {
            drain.store(true, Ordering::SeqCst);
        }
        if attempt == 3 || !retry_safe {
            return Err(error).context(format!(
                "slot {slot_id:02} account initialization failed at attempt {attempt}; retries \
                 exhausted or cleanup unproved"
            ));
        }
        for _ in 0..attempt * 10 {
            if halted() {
                return Ok(None);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    bail!("Account initialization exhausted without a result")
}

fn private_bind(bind: &str) -> anyhow::Result<SocketAddr> {
    let address: SocketAddr = bind
        .parse()
        .context("--bind must be a numeric IP and port")?;
    ensure!(
        address.ip().is_loopback(),
        "--bind must use a private loopback address"
    );
    ensure!(address.port() != 0, "--bind requires a fixed nonzero port");
    Ok(address)
}

fn health_ready(health: &Value) -> bool {
    health.get("ok").and_then(Value::as_bool) == Some(true)
        && health.get("database").and_then(Value::as_bool) == Some(true)
        && health
            .pointer("/agents/connected")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
}

async fn wait_ready(
    client: &GatewayClient,
    mut children: Option<(&mut tokio::process::Child, &mut tokio::process::Child)>,
    deadline: tokio::time::Instant,
) -> anyhow::Result<()> {
    loop {
        if let Some((gateway, agent)) = children.as_mut() {
            ensure!(
                processes::peek_exit(gateway)?.is_none(),
                "Gateway exited during startup; inspect gateway logs"
            );
            ensure!(
                processes::peek_exit(agent)?.is_none(),
                "Agent exited during startup; inspect agent logs"
            );
        }
        ensure!(
            tokio::time::Instant::now() < deadline,
            "Gateway + agent startup deadline exceeded"
        );
        let health = tokio::time::timeout_at(deadline, client.get_json("/health", None, 2)).await;
        if let Ok(Ok(health)) = health
            && health_ready(&health)
        {
            // A different listener must not mask an owned child's failed startup.
            if let Some((gateway, agent)) = children.as_mut() {
                ensure!(
                    processes::peek_exit(gateway)?.is_none(),
                    "Gateway exited during startup"
                );
                ensure!(
                    processes::peek_exit(agent)?.is_none(),
                    "Agent exited during startup"
                );
            }
            return Ok(());
        }
        tokio::time::sleep_until(
            deadline.min(tokio::time::Instant::now() + Duration::from_millis(250)),
        )
        .await;
    }
}

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
async fn run_serve(
    root: &Path,
    bin_dir: &Path,
    bind: &str,
    admin_bind: Option<&str>,
    database_url: &str,
    agent_capacity: usize,
    db_pool: usize,
    replay_jobs: usize,
) -> anyhow::Result<()> {
    let gw_dir = root.join("gateway");
    let _controller_lock = controller_lock(&gw_dir)?;
    std::fs::create_dir_all(&gw_dir)?;
    let gw_bin = bin_dir.join("baylee-gateway");
    let ag_bin = bin_dir.join("baylee-agent");
    let eng_bin = bin_dir.join("baylee-engine-server");
    for b in [&gw_bin, &ag_bin, &eng_bin] {
        ensure!(b.is_file(), "Missing binary {}", b.display());
    }
    let address = private_bind(bind)?;
    let admin_address = admin_bind.map(private_bind).transpose()?;
    if let Some(admin) = admin_address {
        ensure!(
            admin.port() != 0 && admin != address,
            "Admin bind must use a distinct nonzero loopback port"
        );
    }
    ensure!(db_pool > 0, "--db-pool must be positive");
    ensure!(replay_jobs <= 64, "--replay-jobs must be 0..=64");
    let mut random = [0_u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
    let secret = random.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    });
    let bind = address.to_string();
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let gw_out = std::fs::File::create(gw_dir.join("gateway.stdout.log"))?;
    let gw_err = std::fs::File::create(gw_dir.join("gateway.stderr.log"))?;
    let mut gw_cmd = isolated_command(&gw_bin);
    gw_cmd
        .current_dir(&gw_dir)
        .stdin(Stdio::null())
        .stdout(gw_out)
        .stderr(gw_err)
        .process_group(0)
        .env("DATABASE_URL", database_url)
        .env("BAYLEE_BIND", address.ip().to_string())
        .env("PORT", address.port().to_string())
        .env("BAYLEE_AGENT_TOKEN", &secret)
        .env("BAYLEE_ENGINE_URL", format!("ws://{bind}/engine/ws"))
        .env("BAYLEE_DB_POOL", db_pool.to_string())
        .env("BAYLEE_REPLAY_JOBS", replay_jobs.to_string())
        .env("BAYLEE_TRUSTED_PROXIES", "127.0.0.1")
        .env("BAYLEE_CHAIR_TICKET_SECS", "600")
        .env("BAYLEE_ART_PATH", "off")
        .env("BAYLEE_DECK_IMAGE_PATH", "off")
        // Nobody signs in here: the collector writes its teachers into this
        // gateway's database (`teacher.rs`), and guests stay out.
        .env("BAYLEE_AUTH_MODE", "central")
        .env("BAYLEE_REGISTRATION", "on")
        .env("BAYLEE_TERMS_PATH", "off")
        .env("BAYLEE_SOCIAL", "off")
        .env("STORE_PATH", gw_dir.join("gateway-store.json"))
        .env("BAYLEE_EP", "cpu")
        .env("BAYLEE_ENGINE_PUMP_STEPS", "1")
        .env("MKL_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1")
        .env("CUDA_VISIBLE_DEVICES", "")
        .env("RUST_BACKTRACE", "1");
    if let Some(admin) = admin_address {
        let credential =
            admin_cleanup::AdminCleanup::gateway_credentials(&format!("http://{admin}"))?;
        gw_cmd.env("BAYLEE_ADMIN_BIND", admin.to_string());
        credential.configure_gateway(&mut gw_cmd);
    }
    forward_feedback(&mut gw_cmd);
    report_budget(&mut gw_cmd);
    let ag_out = std::fs::File::create(gw_dir.join("agent.stdout.log"))?;
    let ag_err = std::fs::File::create(gw_dir.join("agent.stderr.log"))?;
    let mut gw_child = gw_cmd.spawn().context("spawning baylee-gateway")?;
    let gw_pgid = i32::try_from(gw_child.id().unwrap_or(0)).unwrap_or(0);

    let base_url = format!("http://{bind}");
    let mut ag_cmd = isolated_command(&ag_bin);
    ag_cmd
        .current_dir(&gw_dir)
        .stdin(Stdio::null())
        .stdout(ag_out)
        .stderr(ag_err)
        .process_group(0)
        .env("BAYLEE_GATEWAY", &base_url)
        .env("BAYLEE_AGENT_TOKEN", &secret)
        .env("BAYLEE_AGENT_NAME", "llm-teacher-multi-agent")
        .env("BAYLEE_AGENT_CAPACITY", agent_capacity.to_string())
        .env("BAYLEE_ENGINE_BIN", &eng_bin)
        .env("BAYLEE_EP", "cpu")
        .env("BAYLEE_ENGINE_PUMP_STEPS", "1")
        .env("MKL_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1")
        .env("CUDA_VISIBLE_DEVICES", "")
        .env("RUST_BACKTRACE", "1");
    let mut ag_child = match ag_cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            kill_process_group(gw_pgid, &mut gw_child).await?;
            return Err(error).context("spawning baylee-agent");
        }
    };
    let ag_pgid = i32::try_from(ag_child.id().unwrap_or(0)).unwrap_or(0);
    let client = GatewayClient::new(&base_url, 0);
    if let Err(error) = wait_ready(
        &client,
        Some((&mut gw_child, &mut ag_child)),
        tokio::time::Instant::now() + Duration::from_secs(30),
    )
    .await
    {
        let agent_cleanup = kill_process_group(ag_pgid, &mut ag_child).await;
        let gateway_cleanup = kill_process_group(gw_pgid, &mut gw_child).await;
        agent_cleanup?;
        gateway_cleanup?;
        return Err(error);
    }

    let serving_result: anyhow::Result<()> = async {
        write_atomic_json(
            &gw_dir.join("gateway.json"),
            &json!({
                "status": "ready",
                "gateway": base_url,
                "admin_url": admin_address.map(|address| format!("http://{address}")),
                "gateway_pid": gw_pgid,
                "agent_pid": ag_pgid,
                "agent_capacity": agent_capacity,
                "db_pool": db_pool,
                "replay_jobs": replay_jobs,
                "started_unix": now_unix(),
            }),
        )?;

        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                () = tokio::time::sleep(Duration::from_secs(1)) => {
                    if processes::peek_exit(&mut gw_child)?.is_some() || processes::peek_exit(&mut ag_child)?.is_some() {
                        bail!("Owned gateway or agent exited; inspect component logs");
                    }
                }
            }
        }

        Ok(())
    }
    .await;
    let agent_cleanup = kill_process_group(ag_pgid, &mut ag_child).await;
    let gateway_cleanup = kill_process_group(gw_pgid, &mut gw_child).await;
    agent_cleanup?;
    gateway_cleanup?;
    let _ = write_atomic_json(
        &gw_dir.join("gateway.json"),
        &json!({
            "status": "stopped",
            "gateway": base_url,
            "stopped_unix": now_unix(),
        }),
    );
    serving_result
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn run_collect(
    root: PathBuf,
    gateway: String,
    admin_url: String,
    database_url: String,
    seat_bin: PathBuf,
    gateway_bin: PathBuf,
    profiles_config: PathBuf,
    pairings_file: Option<PathBuf>,
    deck_manifest: PathBuf,
    concurrency: usize,
    max_attempts: Option<usize>,
    call_timeout_seconds: u64,
    game_timeout_seconds: u64,
    launch_interval_ms: u64,
    report_games: usize,
) -> anyhow::Result<()> {
    table::gateway_endpoint(&gateway)?;
    let admin_cleanup = Arc::new(admin_cleanup::AdminCleanup::from_env(&admin_url)?);
    admin_cleanup.preflight().await?;
    let _controller_lock = controller_lock(&root)?;
    ensure!(concurrency >= 1, "--concurrency must be >= 1");
    quota_hold::check_startup(&root, now_unix())?;
    // Opened as the gateway left it, never migrated from here: a migration
    // is a write, and nothing is written before the proof below.
    let db = baylee_db::open(&database_url, 4)
        .await
        .context("connecting to PostgreSQL for record export")?;
    let probe_client = GatewayClient::new(&gateway, 0);
    wait_ready(
        &probe_client,
        None,
        tokio::time::Instant::now() + Duration::from_secs(30),
    )
    .await?;
    let own = Arc::new(teacher::OwnDatabase::prove(&admin_cleanup, db.clone()).await?);
    let sched = Scheduler::new(
        root.clone(),
        gateway.clone(),
        profiles_config,
        pairings_file,
        &deck_manifest,
        call_timeout_seconds,
        game_timeout_seconds,
    )?
    .with_report_games(report_games);
    let initial_summary = sched.summarize()?;
    let seats_the_house = sched.seats_the_house();
    let sched = Arc::new(Mutex::new(sched));
    let pacer = Arc::new(LaunchPacer::new(Duration::from_millis(launch_interval_ms)));
    let replay_sem = Arc::new(Semaphore::new(8));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let budget = Arc::new(AttemptBudget::new(max_attempts));
    let drain_flag = Arc::new(AtomicBool::new(budget.exhausted()));
    let stop_file = root.join("STOP");
    let drain_file = root.join("DRAIN");

    write_atomic_json(
        &root.join("controller.json"),
        &json!({
            "pid": std::process::id(),
            "status": "running",
            "max_attempts": max_attempts,
            "concurrency": concurrency,
            "gateway": gateway,
            "started_unix": now_unix(),
        }),
    )?;
    write_atomic_json(
        &root.join("status.json"),
        &json!({
            "phase": "collecting",
            "concurrency": concurrency,
            "updated_unix": now_unix(),
            "summary": initial_summary,
        }),
    )?;

    let stop_signal = Arc::clone(&stop_flag);
    tokio::spawn(async move {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        let mut sigint =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
        stop_signal.store(true, Ordering::SeqCst);
    });

    let mut workers = tokio::task::JoinSet::new();
    for slot_id in 0..concurrency {
        let admin_c = Arc::clone(&admin_cleanup);
        let root_c = root.clone();
        let gw_c = gateway.clone();
        let seat_c = seat_bin.clone();
        let gw_bin_c = gateway_bin.clone();
        let db_c = db.clone();
        let sched_c = Arc::clone(&sched);
        let pacer_c = Arc::clone(&pacer);
        let rep_c = Arc::clone(&replay_sem);
        let own_c = Arc::clone(&own);
        let stop_c = Arc::clone(&stop_flag);
        let drain_c = Arc::clone(&drain_flag);
        let budget_c = Arc::clone(&budget);
        let stop_file_c = stop_file.clone();
        let drain_file_c = drain_file.clone();

        workers.spawn(async move {
            // No serialised setup since the cutover: a teacher is two rows
            // in the database and its presence socket, with no password to
            // hash at the gateway.
            let halted = || assignment_halted(&stop_c, &drain_c, &stop_file_c, &drain_file_c);
            let Some(mut account) =
                slot_account(&own_c, &gw_c, slot_id, Sits::Model, &halted, &drain_c).await?
            else {
                return Ok(());
            };
            // The house sits in an account of its own, named for it, where
            // a pairing seats it (`table::run_attempt`).
            let mut house = None;
            if seats_the_house {
                match slot_account(&own_c, &gw_c, slot_id, Sits::House, &halted, &drain_c).await {
                    Ok(Some(made)) => house = Some(made),
                    Ok(None) => return account.shutdown().await,
                    Err(error) => {
                        if let Err(cleanup) = account.shutdown().await {
                            eprintln!("[slot {slot_id:02}] account cleanup failed: {cleanup:#}");
                        }
                        return Err(error);
                    }
                }
            }
            let mut worker_error = None;
            let mut consecutive_failures = 0_usize;
            while !assignment_halted(&stop_c, &drain_c, &stop_file_c, &drain_file_c) {
                let assignment = {
                    let mut guard = sched_c.lock().await;
                    if assignment_halted(&stop_c, &drain_c, &stop_file_c, &drain_file_c) {
                        break;
                    }
                    if !budget_c.reserve() {
                        drain_c.store(true, Ordering::SeqCst);
                        break;
                    }
                    if budget_c.exhausted() {
                        drain_c.store(true, Ordering::SeqCst);
                    }
                    match guard.next_assignment(slot_id) {
                        Ok(a) => a,
                        Err(e) => {
                            // No assignment exists yet; only actual attempt failures consume the budget.
                            budget_c.assigned.fetch_sub(1, Ordering::SeqCst);
                            eprintln!("[slot {slot_id:02}] schedule error: {e:#}");
                            drain_c.store(true, Ordering::SeqCst);
                            worker_error = Some(e);
                            break;
                        }
                    }
                };
                println!(
                    "[slot {slot_id:02}] starting {} ({}: {} [{}] vs {} [{}])",
                    assignment.attempt_id,
                    assignment.format,
                    assignment.decks[0].name,
                    assignment.models[0],
                    assignment.decks[1].name,
                    assignment.models[1],
                );
                let t0 = Instant::now();
                // The house's account hosts where the house plays chair 0,
                // and joins chair 1 where it plays that.
                let (host, chair1_house) = match (assignment.house_seat(), house.as_mut()) {
                    (Some(0), Some(house)) => (house, None),
                    (Some(_), house) => (&mut account, house),
                    (None, _) => (&mut account, None),
                };
                let attempt_res = run_attempt(
                    &assignment,
                    host,
                    chair1_house,
                    &seat_c,
                    &gw_bin_c,
                    &db_c,
                    &pacer_c,
                    &rep_c,
                    &stop_c,
                    &stop_file_c,
                    &admin_c,
                )
                .await;

                let mut quota_backoff = false;
                {
                    let mut guard = sched_c.lock().await;
                    // Serialize provider hold detection with assignment reservation.
                    // This is independent of success, GameOver, replay or admission.
                    match quota_hold::inspect_attempt(&assignment.attempt_dir, now_unix()) {
                        Ok(evidence) if !evidence.is_empty() => {
                            drain_c.store(true, Ordering::SeqCst);
                            eprintln!("[slot {slot_id:02}] Individual provider quota hold: draining new assignments; inspect quota-hold.json");
                            if let Err(error) = quota_hold::persist(&root_c, evidence, now_unix()) {
                                worker_error = Some(error.context("Persisting provider quota hold"));
                            }
                        }
                        Err(error) => {
                            drain_c.store(true, Ordering::SeqCst);
                            worker_error = Some(error.context("Inspecting provider quota evidence"));
                        }
                        Ok(_) => {}
                    }
                    match attempt_res {
                        Ok(technical) => {
                            let rec_sha = technical
                                .get("record_sha256")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            if guard.register_record_sha(rec_sha) {
                                quota_backoff = technical
                                    .get("quota_limited")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false);
                                consecutive_failures = usize::from(quota_backoff);
                                let receipt = json!({
                                    "status": "admitted",
                                    "request": assignment.request,
                                    "technical": technical,
                                    "admission": "BC-only; technical verification; no semantic-strength gate",
                                });
                                let _ = write_atomic_json(
                                    &assignment.attempt_dir.join("receipt.json"),
                                    &receipt,
                                );
                                println!(
                                    "[slot {slot_id:02}] ADMITTED {} in {:.1}s (winner={})",
                                    assignment.attempt_id,
                                    t0.elapsed().as_secs_f64(),
                                    technical
                                        .get("terminal")
                                        .and_then(|t| t.get("winner_seat"))
                                        .unwrap_or(&Value::Null),
                                );
                            } else {
                                consecutive_failures += 1;
                                let _ = write_atomic_json(
                                    &assignment.attempt_dir.join("receipt.json"),
                                    &json!({
                                        "status": "error",
                                        "request": assignment.request,
                                        "error": "Duplicate complete record SHA-256",
                                    }),
                                );
                            }
                        }
                        Err(err) => {
                            consecutive_failures += 1;
                            let err_msg = format!("{err:#}");
                            let is_stop = stop_c.load(Ordering::SeqCst) || stop_file_c.exists();
                            let status = if is_stop {
                                "incomplete"
                            } else if err_msg.contains("deadline exceeded") {
                                "timeout"
                            } else {
                                "error"
                            };
                            let _ = write_atomic_json(
                                &assignment.attempt_dir.join("technical-result.json"),
                                &json!({
                                    "schema_version": 1,
                                    "status": "failed",
                                    "error_message": err_msg,
                                    "bc_only": true,
                                    "ppo_eligible": false,
                                }),
                            );
                            let _ = write_atomic_json(
                                &assignment.attempt_dir.join("receipt.json"),
                                &json!({
                                    "status": status,
                                    "request": assignment.request,
                                    "error": err_msg,
                                }),
                            );
                            eprintln!(
                                "[slot {slot_id:02}] FAILED {} ({status}) in {:.1}s: {err_msg}",
                                assignment.attempt_id,
                                t0.elapsed().as_secs_f64()
                            );
                        }
                    }
                    if let Err(error) = schedule::attempt_closed(&assignment.attempt_dir) {
                        eprintln!("[slot {slot_id:02}] unsafe attempt cleanup: {error:#}");
                        drain_c.store(true, Ordering::SeqCst);
                        worker_error = Some(error);
                    }
                    if let Ok(summary) = guard.complete_assignment(&assignment.attempt_id) {
                        let _ = write_atomic_json(
                            &root_c.join("status.json"),
                            &json!({
                                "phase": "collecting",
                                "concurrency": concurrency,
                                "last_attempt": assignment.attempt_id,
                                "updated_unix": now_unix(),
                                "summary": summary,
                            }),
                        );
                    }
                }

                let delay_secs = if quota_backoff {
                    120_u64
                } else if consecutive_failures > 0 {
                    std::cmp::min(180, 15 * (consecutive_failures as u64))
                } else {
                    2
                };
                for _ in 0..delay_secs {
                    if assignment_halted(&stop_c, &drain_c, &stop_file_c, &drain_file_c) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
            for account in std::iter::once(&mut account).chain(house.as_mut()) {
                if let Err(error) = account.shutdown().await {
                    eprintln!("[slot {slot_id:02}] account cleanup failed: {error:#}");
                    drain_c.store(true, Ordering::SeqCst);
                    worker_error = Some(error);
                }
            }
            match worker_error {
                Some(error) => Err(error),
                None => Ok(()),
            }
        });
    }

    let mut worker_failures = Vec::new();
    while let Some(result) = workers.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => worker_failures.push(format!("{error:#}")),
            Err(error) => {
                stop_flag.store(true, Ordering::SeqCst);
                worker_failures.push(format!("Collector worker failed: {error}"));
            }
        }
    }
    let failed = !worker_failures.is_empty();

    let final_summary = sched.lock().await.summarize()?;
    write_atomic_json(
        &root.join("status.json"),
        &json!({
            "phase": if failed { "failed" } else { "stopped" },
            "worker_failures": worker_failures,
            "max_attempts": max_attempts,
            "assigned_attempts": budget.assigned.load(Ordering::SeqCst),
            "concurrency": concurrency,
            "updated_unix": now_unix(),
            "summary": final_summary,
        }),
    )?;
    write_atomic_json(
        &root.join("controller.json"),
        &json!({
            "pid": std::process::id(),
            "status": if failed { "failed" } else { "exited" },
            "ended_unix": now_unix(),
        }),
    )?;
    ensure!(!failed, "Collector workers failed; inspect status.json");
    Ok(())
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    #[test]
    fn attempt_budget_caps_parallel_reservations_and_never_refunds_failures() {
        let budget = Arc::new(AttemptBudget::new(Some(7)));
        let threads: Vec<_> = (0..50)
            .map(|_| {
                let budget = Arc::clone(&budget);
                std::thread::spawn(move || usize::from(budget.reserve()))
            })
            .collect();
        let reserved: usize = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .sum();
        assert_eq!(reserved, 7);
        assert!(budget.exhausted());
        assert!(!budget.reserve());
        let unlimited = AttemptBudget::new(None);
        for _ in 0..100 {
            assert!(unlimited.reserve());
        }
        assert!(!unlimited.exhausted());
        assert!(!AttemptBudget::new(Some(0)).reserve());
    }

    #[test]
    fn drain_halts_assignments_without_requesting_active_attempt_cancellation() {
        let dir = crate::audit::tests::TestDir::new("collector-drain");
        let stop = AtomicBool::new(false);
        let drain = AtomicBool::new(false);
        let stop_file = dir.path().join("STOP");
        let drain_file = dir.path().join("DRAIN");
        assert!(!assignment_halted(&stop, &drain, &stop_file, &drain_file));
        std::fs::write(&drain_file, b"").unwrap();
        assert!(assignment_halted(&stop, &drain, &stop_file, &drain_file));
        assert!(!stop.load(Ordering::SeqCst));
        assert!(!stop_file.exists());
        std::fs::remove_file(&drain_file).unwrap();
        assert!(!assignment_halted(&stop, &drain, &stop_file, &drain_file));
        drain.store(true, Ordering::SeqCst);
        assert!(assignment_halted(&stop, &drain, &stop_file, &drain_file));
        assert!(!stop.load(Ordering::SeqCst));
        assert!(!stop_file.exists());
        drain.store(false, Ordering::SeqCst);
        std::fs::write(&stop_file, b"").unwrap();
        assert!(assignment_halted(&stop, &drain, &stop_file, &drain_file));
    }

    #[test]
    fn singleton_lock_is_exclusive_and_released_on_drop() {
        const CHILD_ROOT: &str = "BAYLEE_TEST_COLLECTOR_LOCK_ROOT";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            assert!(controller_lock(Path::new(&root)).is_err());
            return;
        }
        let dir = crate::audit::tests::TestDir::new("controller-lock");
        let first = controller_lock(dir.path()).unwrap();
        assert!(controller_lock(dir.path()).is_err());
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "startup_tests::singleton_lock_is_exclusive_and_released_on_drop",
            ])
            .env(CHILD_ROOT, dir.path())
            .status()
            .unwrap();
        assert!(status.success());
        drop(first);
        assert!(controller_lock(dir.path()).is_ok());
    }

    #[test]
    fn private_children_clear_inherited_environment() {
        let command = isolated_command(Path::new("baylee-gateway"));
        let environment: Vec<_> = command.as_std().get_envs().collect();
        assert!(environment.iter().all(|(key, _)| {
            ["PATH", "HOME", "USER", "LANG", "LD_LIBRARY_PATH"]
                .iter()
                .any(|name| *key == std::ffi::OsStr::new(name))
        }));
    }

    #[test]
    fn only_the_feedback_variables_reach_the_gateway_beside_its_own() {
        let mut command = isolated_command(Path::new("baylee-gateway"));
        forward_feedback(&mut command);
        let names: Vec<_> = command
            .as_std()
            .get_envs()
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|name| {
            ["PATH", "HOME", "USER", "LANG", "LD_LIBRARY_PATH"]
                .iter()
                .chain(&FEEDBACK_ENV)
                .any(|allowed| name == allowed)
        }));
    }

    /// The private gateway's accounts may file far more than a public
    /// gateway's 64 reports an hour, by the operator's own setting, and
    /// the setting is a budget the gateway takes (a whole number past 64).
    #[test]
    fn the_private_gateway_s_accounts_file_their_reports_by_the_thousands() {
        let mut command = isolated_command(Path::new("baylee-gateway"));
        report_budget(&mut command);
        let budget = command
            .as_std()
            .get_envs()
            .find(|(key, _)| *key == std::ffi::OsStr::new("BAYLEE_REPORTS_PER_HOUR"))
            .and_then(|(_, value)| value)
            .and_then(|value| value.to_str())
            .and_then(|value| value.parse::<usize>().ok());
        assert_eq!(budget, Some(2000));
    }

    #[test]
    fn private_host_requires_fixed_loopback_address() {
        for bind in ["127.0.0.1:28768", "[::1]:28768"] {
            assert!(private_bind(bind).is_ok());
        }
        for bind in [
            "0.0.0.0:28768",
            "192.168.1.1:28768",
            "127.0.0.1:0",
            "localhost:28768",
        ] {
            assert!(private_bind(bind).is_err());
        }
    }

    #[test]
    fn health_requires_database_and_connected_agent() {
        assert!(health_ready(
            &json!({"ok":true,"database":true,"agents":{"connected":1}})
        ));
        for health in [
            json!({"ok":true}),
            json!({"ok":true,"database":false,"agents":{"connected":1}}),
            json!({"ok":true,"database":true,"agents":{"connected":0}}),
        ] {
            assert!(!health_ready(&health));
        }
    }
}
