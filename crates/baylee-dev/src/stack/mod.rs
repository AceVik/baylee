//! Local interactive stack and native PostgreSQL gate wrapper (`cargo dev stack`).
//!
//! Keeps `baylee-dev` free of heavy database and ORM dependencies while giving
//! developers and agents a single command for starting/stopping an interactive
//! local Baylee stack or running the gates against an owned native PostgreSQL 18
//! instance without Docker.
#![forbid(unsafe_code)]

mod postgres;
mod services;

#[cfg(test)]
mod tests;

use crate::fixture;
use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Subcommand};
use postgres::{LocalPostgres, create_private_dir, free_loopback_port};
use services::{
    ManagedChild, StackState, build_and_stage, build_and_stage_client, probe_http_200, staged_exe,
    wait_http_ready, wait_tcp,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const DEFAULT_STATE_DIR: &str = "target/local-stack";
const DEFAULT_GATEWAY_PORT: u16 = 28766;
const DEFAULT_MONITOR: i64 = 2;
const STATE_FILE: &str = "stack.json";
const STOP_FILE: &str = "stop";

/// CLI options for `cargo dev stack`.
#[derive(Args)]
pub(crate) struct Options {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Build and supervise local PostgreSQL 18, feedback, gateway, agent and
    /// interactive desktop client (built without `dev-control`).
    Up {
        /// Display index for the interactive client window (`BAYLEE_DEV_MONITOR`).
        #[arg(long, default_value_t = DEFAULT_MONITOR)]
        monitor: i64,
        /// Start only backend services without opening a client window.
        #[arg(long)]
        no_client: bool,
        /// Loopback port for `baylee-gateway`.
        #[arg(long, default_value_t = DEFAULT_GATEWAY_PORT)]
        gateway_port: u16,
        /// Optional loopback Central native endpoint (`http://127.0.0.1:28790`).
        #[arg(long)]
        central_endpoint: Option<String>,
        /// Optional Central website origin (`http://127.0.0.1:28794`) for browser links.
        #[arg(long)]
        central_web_origin: Option<String>,
        /// Optional `0600` gateway-Central bridge JSON file when bridging Central.
        #[arg(long)]
        central_bridge_file: Option<PathBuf>,
        /// Directory holding `pgdata`, staged binaries, logs and `stack.json`.
        #[arg(long, default_value = DEFAULT_STATE_DIR)]
        state_dir: PathBuf,
    },
    /// Stop a running `cargo dev stack up` supervisor and verify its ports closed.
    Down {
        #[arg(long, default_value = DEFAULT_STATE_DIR)]
        state_dir: PathBuf,
    },
    /// Check liveness of the local stack recorded in `stack.json`.
    Status {
        #[arg(long, default_value = DEFAULT_STATE_DIR)]
        state_dir: PathBuf,
    },
    /// Launch an interactive `baylee-client` against a running `cargo dev stack`.
    Client {
        #[arg(long, default_value_t = DEFAULT_MONITOR)]
        monitor: i64,
        #[arg(long, default_value = DEFAULT_STATE_DIR)]
        state_dir: PathBuf,
    },
    /// Run `cargo dev gate` (or `features`) against an ephemeral native PostgreSQL 18.
    Gate {
        /// Run `cargo dev gate features` instead of `cargo dev gate`.
        #[arg(long)]
        features: bool,
        /// Test every crate (`--workspace`).
        #[arg(long)]
        workspace: bool,
        /// Include slow path-scoped tests (`--all`).
        #[arg(long)]
        all: bool,
    },
}

#[derive(Clone)]
struct UpConfig {
    monitor: i64,
    no_client: bool,
    gateway_port: u16,
    central_endpoint: Option<String>,
    central_web_origin: Option<String>,
    central_bridge_file: Option<PathBuf>,
    state_dir: PathBuf,
}

/// Dispatches `cargo dev stack` subcommands.
pub(crate) fn run(options: Options) -> Result<i32> {
    match options.command {
        Action::Up {
            monitor,
            no_client,
            gateway_port,
            central_endpoint,
            central_web_origin,
            central_bridge_file,
            state_dir,
        } => up(&UpConfig {
            monitor,
            no_client,
            gateway_port,
            central_endpoint,
            central_web_origin,
            central_bridge_file,
            state_dir,
        }),
        Action::Down { state_dir } => down(&state_dir),
        Action::Status { state_dir } => status(&state_dir),
        Action::Client { monitor, state_dir } => launch_extra_client(&state_dir, monitor),
        Action::Gate {
            features,
            workspace,
            all,
        } => gate_with_local_postgres(features, workspace, all),
    }
}

fn up(cfg: &UpConfig) -> Result<i32> {
    if let Some(ep) = &cfg.central_endpoint {
        ensure!(
            loopback_http_origin(ep),
            "--central-endpoint must be http://127.0.0.1:<port>"
        );
    }
    if let Some(web) = &cfg.central_web_origin {
        ensure!(
            loopback_http_origin(web),
            "--central-web-origin must be http://127.0.0.1:<port>"
        );
    }
    create_private_dir(&cfg.state_dir)?;
    let state_dir = fs::canonicalize(&cfg.state_dir)?;
    let _owner = lock_stack(&state_dir)?;
    let _ = fs::remove_file(state_dir.join(STOP_FILE));
    let bin_dir = state_dir.join("bin");
    let logs_dir = state_dir.join("logs");
    fs::create_dir_all(&logs_dir)?;

    build_and_stage(&bin_dir, !cfg.no_client)?;
    if state_dir.join(STOP_FILE).exists() {
        fs::remove_file(state_dir.join(STOP_FILE))?;
        println!("Local stack startup cancelled before opening any listeners.");
        return Ok(0);
    }
    let pg = LocalPostgres::start(state_dir.join("pgdata"), None, true)?;
    pg.create_database("baylee")?;
    pg.create_database("baylee_feedback_dev")?;

    let supervise_res = supervise(cfg, &state_dir, &bin_dir, &logs_dir, &pg);
    let pg_res = pg.finish();
    if pg_res.is_ok() {
        let _ = fs::remove_file(state_dir.join(STATE_FILE));
        let _ = fs::remove_file(state_dir.join(STOP_FILE));
    }
    supervise_res?;
    pg_res?;
    println!("Local stack stopped cleanly and all owned listeners are closed.");
    Ok(0)
}

fn supervise(
    cfg: &UpConfig,
    state_dir: &Path,
    bin_dir: &Path,
    logs_dir: &Path,
    pg: &LocalPostgres,
) -> Result<()> {
    let admin_port = free_loopback_port()?;
    let feedback_port = free_loopback_port()?;
    let gateway_origin = format!("http://127.0.0.1:{}", cfg.gateway_port);
    let mut children = spawn_backend_services(
        cfg,
        bin_dir,
        logs_dir,
        pg,
        admin_port,
        feedback_port,
        &gateway_origin,
    )?;

    let mut client = if cfg.no_client {
        None
    } else {
        Some(spawn_interactive_client(
            bin_dir,
            logs_dir,
            &gateway_origin,
            cfg.central_endpoint.as_deref(),
            cfg.central_web_origin.as_deref(),
            cfg.monitor,
        )?)
    };

    let state = StackState {
        supervisor_pid: std::process::id(),
        postgres_port: pg.port(),
        gateway_port: cfg.gateway_port,
        admin_port,
        feedback_port,
        gateway_origin: gateway_origin.clone(),
        central_endpoint: cfg.central_endpoint.clone(),
        monitor: cfg.monitor,
        logs_dir: logs_dir.to_path_buf(),
        client_pid: client.as_ref().map(ManagedChild::id),
    };
    fs::write(
        state_dir.join(STATE_FILE),
        serde_json::to_vec_pretty(&state)?,
    )?;
    println!(
        "STACK_READY gateway={gateway_origin} monitor={}",
        cfg.monitor
    );
    hold_until_stopped(state_dir, &mut children, &mut client)
}

fn spawn_backend_services(
    cfg: &UpConfig,
    bin_dir: &Path,
    logs_dir: &Path,
    pg: &LocalPostgres,
    admin_port: u16,
    feedback_port: u16,
    gateway_origin: &str,
) -> Result<Vec<ManagedChild>> {
    let agent_token = crate::secret::hex(32)?;
    let admin_token = crate::secret::hex(32)?;
    let fb_gw_token = crate::secret::hex(32)?;
    let fb_read_token = crate::secret::hex(32)?;
    let fb_admin_token = crate::secret::hex(32)?;
    let fb_direct_key = crate::secret::hex(32)?;

    let mut children = Vec::with_capacity(3);
    let mut feedback = ManagedChild::spawn(
        "baylee-feedback",
        Command::new(staged_exe(bin_dir, "baylee-feedback"))
            .env("RUST_LOG", "info")
            .env("FEEDBACK_DATABASE_URL", pg.url("baylee_feedback_dev"))
            .env("FEEDBACK_BIND", format!("127.0.0.1:{feedback_port}"))
            .env("FEEDBACK_GATEWAY_TOKENS", format!("local={fb_gw_token}"))
            .env("FEEDBACK_READ_TOKEN", &fb_read_token)
            .env("FEEDBACK_ADMIN_TOKEN", &fb_admin_token)
            .env("FEEDBACK_DIRECT_KEY", &fb_direct_key)
            .env(
                "FEEDBACK_GATEWAY_ADMIN_URL",
                format!("http://127.0.0.1:{admin_port}"),
            )
            .env("FEEDBACK_GATEWAY_ADMIN_TOKEN", &admin_token),
        logs_dir,
    )?;
    wait_tcp(feedback_port, &mut feedback, Duration::from_secs(15))?;
    children.push(feedback);

    let mut gw_cmd = Command::new(staged_exe(bin_dir, "baylee-gateway"));
    gw_cmd
        .env("RUST_LOG", "info")
        .env("PORT", cfg.gateway_port.to_string())
        .env("BAYLEE_BIND", "127.0.0.1")
        .env("BAYLEE_ADMIN_BIND", format!("127.0.0.1:{admin_port}"))
        .env("BAYLEE_ADMIN_TOKEN", &admin_token)
        .env("DATABASE_URL", pg.url("baylee"))
        .env("BAYLEE_GATEWAY_NAME", "Baylee Local Dev")
        .env("BAYLEE_AGENT_TOKEN", &agent_token)
        .env(
            "BAYLEE_FEEDBACK_URL",
            format!("http://127.0.0.1:{feedback_port}"),
        )
        .env("BAYLEE_FEEDBACK_TOKEN", &fb_gw_token);
    if let (Some(ep), Some(bridge)) = (&cfg.central_endpoint, &cfg.central_bridge_file) {
        gw_cmd
            .env("BAYLEE_AUTH_MODE", "mixed")
            .env("BAYLEE_REGISTRATION", "open")
            .env("BAYLEE_CENTRAL_ENDPOINT", ep)
            .env("BAYLEE_CENTRAL_CONFIG_FILE", bridge);
    } else {
        gw_cmd.env("BAYLEE_AUTH_MODE", "guests");
    }
    let mut gateway = ManagedChild::spawn("baylee-gateway", &mut gw_cmd, logs_dir)?;
    wait_http_ready(
        cfg.gateway_port,
        "/health",
        "\"ok\":true",
        &mut gateway,
        Duration::from_secs(20),
    )?;
    children.push(gateway);

    let extra_bins = std::env::join_paths([
        staged_exe(bin_dir, "baylee-engine-standard"),
        staged_exe(bin_dir, "baylee-engine-tower"),
    ])?;
    let mut agent = ManagedChild::spawn(
        "baylee-agent",
        Command::new(staged_exe(bin_dir, "baylee-agent"))
            .env("RUST_LOG", "info")
            .env("BAYLEE_GATEWAY", gateway_origin)
            .env("BAYLEE_AGENT_TOKEN", &agent_token)
            .env("BAYLEE_AGENT_NAME", "local-stack-agent")
            .env(
                "BAYLEE_ENGINE_BIN",
                staged_exe(bin_dir, "baylee-engine-server"),
            )
            .env("BAYLEE_ENGINE_BINS", extra_bins),
        logs_dir,
    )?;
    wait_http_ready(
        cfg.gateway_port,
        "/health",
        "\"agents\":{\"connected\":1",
        &mut agent,
        Duration::from_secs(20),
    )?;
    children.push(agent);
    Ok(children)
}

fn hold_until_stopped(
    state_dir: &Path,
    children: &mut [ManagedChild],
    client: &mut Option<ManagedChild>,
) -> Result<()> {
    let stop_path = state_dir.join(STOP_FILE);
    while !stop_path.exists() {
        for child in children.iter_mut() {
            if let Some(status) = child.exited()? {
                bail!("{} exited unexpectedly with {status}", child.name());
            }
        }
        if let Some(ui) = client
            && ui.exited()?.is_some()
        {
            *client = None;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

fn spawn_interactive_client(
    bin_dir: &Path,
    logs_dir: &Path,
    gateway_origin: &str,
    central_endpoint: Option<&str>,
    central_web_origin: Option<&str>,
    monitor: i64,
) -> Result<ManagedChild> {
    let mut cmd = Command::new(staged_exe(bin_dir, "baylee-client"));
    cmd.env("BAYLEE_DEV_MONITOR", monitor.to_string())
        .env("BAYLEE_GATEWAY", gateway_origin)
        .env_remove("BAYLEE_DEV_CONTROL")
        .env_remove("BAYLEE_DEV_PASSIVE");
    if let Some(ep) = central_endpoint {
        cmd.env("BAYLEE_CENTRAL_DEV_ENDPOINT", ep);
    }
    if let Some(web) = central_web_origin {
        cmd.env("BAYLEE_CENTRAL_WEB_ORIGIN", web);
    }
    ManagedChild::spawn("baylee-client", &mut cmd, logs_dir)
}

fn down(state_dir: &Path) -> Result<i32> {
    if !state_dir.exists() {
        println!("Local stack is down.");
        return Ok(0);
    }
    let state_path = state_dir.join(STATE_FILE);
    let recorded = fs::read(&state_path)
        .ok()
        .and_then(|raw| serde_json::from_slice::<StackState>(&raw).ok());
    let owner = open_stack_lock(state_dir)?;
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        // The lock also covers compilation, before stack.json exists. Leave the
        // stop request in place on timeout so a slow startup still cancels.
        fs::write(state_dir.join(STOP_FILE), b"stop\n")?;
        match owner.try_lock() {
            Ok(()) => break,
            Err(fs::TryLockError::WouldBlock) => {
                ensure!(
                    Instant::now() < deadline,
                    "stack supervisor is still starting or stopping; stop request retained"
                );
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
    ensure!(
        !state_path.exists(),
        "stack supervisor has not completed shutdown; leaving its state intact"
    );
    let _ = fs::remove_file(state_dir.join(STOP_FILE));
    if let Some(st) = recorded {
        for port in [
            st.gateway_port,
            st.admin_port,
            st.feedback_port,
            st.postgres_port,
        ] {
            fixture::refused(port)
                .with_context(|| format!("port 127.0.0.1:{port} is still listening"))?;
        }
    }
    println!("Local stack is down.");
    Ok(0)
}

fn lock_stack(state_dir: &Path) -> Result<fs::File> {
    let file = open_stack_lock(state_dir)?;
    file.try_lock()
        .context("a stack supervisor already owns this directory")?;
    Ok(file)
}

fn open_stack_lock(state_dir: &Path) -> Result<fs::File> {
    Ok(fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(state_dir.join("supervisor.lock"))?)
}

fn status(state_dir: &Path) -> Result<i32> {
    let state_path = state_dir.join(STATE_FILE);
    let Ok(raw) = fs::read(&state_path) else {
        println!("Local stack: stopped (no {})", state_path.display());
        return Ok(1);
    };
    let st: StackState = serde_json::from_slice(&raw)?;
    let gw_ok =
        probe_http_200(st.gateway_port, "/health").is_some_and(|body| body.contains("\"ok\":true"));
    println!(
        "Local stack: {} (gateway={}, postgres=127.0.0.1:{}, monitor={}, logs={})",
        if gw_ok { "healthy" } else { "unhealthy" },
        st.gateway_origin,
        st.postgres_port,
        st.monitor,
        st.logs_dir.display()
    );
    Ok(i32::from(!gw_ok))
}

fn launch_extra_client(state_dir: &Path, monitor: i64) -> Result<i32> {
    let state_path = state_dir.join(STATE_FILE);
    let raw = fs::read(&state_path).with_context(|| {
        format!(
            "no running stack at {}; run `cargo dev stack up` first",
            state_path.display()
        )
    })?;
    let st: StackState = serde_json::from_slice(&raw)?;
    let exe = build_and_stage_client(&state_dir.join("bin"))?;
    let mut cmd = Command::new(exe);
    cmd.env("BAYLEE_DEV_MONITOR", monitor.to_string())
        .env("BAYLEE_GATEWAY", &st.gateway_origin)
        .env_remove("BAYLEE_DEV_CONTROL")
        .env_remove("BAYLEE_DEV_PASSIVE");
    if let Some(ep) = &st.central_endpoint {
        cmd.env("BAYLEE_CENTRAL_DEV_ENDPOINT", ep);
    }
    let child = cmd.spawn().context("spawning interactive baylee-client")?;
    println!(
        "Started interactive baylee-client (pid {}) on monitor {monitor}",
        child.id()
    );
    Ok(0)
}

fn gate_with_local_postgres(features: bool, workspace: bool, all: bool) -> Result<i32> {
    let dir = PathBuf::from("target").join(format!("gate-pg-{}", crate::secret::hex(6)?));
    let pg = LocalPostgres::start(dir, None, true)?;
    pg.create_database("baylee_gate")?;
    let mut cmd = Command::new(std::env::current_exe()?);
    cmd.arg("gate");
    if features {
        cmd.arg("features");
    } else {
        if workspace {
            cmd.arg("--workspace");
        }
        if all {
            cmd.arg("--all");
        }
    }
    cmd.env("DATABASE_URL", pg.url("baylee_gate"));
    let status = cmd.status();
    let cleanup = pg.finish();
    let status = status.context("running `cargo dev gate` child")?;
    cleanup?;
    Ok(status.code().unwrap_or(1))
}

pub(super) fn loopback_http_origin(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("http://127.0.0.1:") else {
        return false;
    };
    rest.parse::<u16>().is_ok_and(|port| port > 0)
}
