//! Local interactive stack and native PostgreSQL gate wrapper (`cargo dev stack`).
//!
//! Keeps `baylee-dev` free of heavy database and ORM dependencies while giving
//! developers and agents a single command for starting/stopping an interactive
//! local Baylee stack or running the gates against an owned native PostgreSQL 18
//! instance without Docker.
#![forbid(unsafe_code)]

mod central;
pub(crate) mod postgres;
mod provision;
mod services;
mod stock;
mod tower;

#[cfg(test)]
mod tests;

use crate::fixture;
use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Subcommand};
use postgres::{LocalPostgres, create_private_dir, free_loopback_port};
use services::{
    ManagedChild, StackState, build_and_stage, build_and_stage_client, build_and_stage_full,
    probe_http_200, staged_exe, wait_http_ready, wait_tcp,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use stock::Stock;

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
        /// A Baylee Central checkout to build and run beside the stack: its
        /// service and website (Register in the client opens the local
        /// website), a fresh Central database with one synthetic
        /// administrator, the Tower authority with a playable cohort, and
        /// Daily and Weekly tasks. Instead of the three options above.
        #[arg(
            long,
            value_name = "CHECKOUT",
            conflicts_with_all = ["central_endpoint", "central_web_origin", "central_bridge_file"]
        )]
        central: Option<PathBuf>,
        /// With `--central`: the days of Daily tasks to keep ahead, today
        /// included; 0 publishes no tasks.
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u8).range(0..=24))]
        stock_days: u8,
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
    central: Option<PathBuf>,
    stock_days: u8,
    state_dir: PathBuf,
}

/// How the gateway signs players in, and what it is told for it.
pub(super) enum Auth<'a> {
    /// No Central: guests only.
    Guests,
    /// A Central someone else runs, bridged through the caller's file.
    Attached { endpoint: &'a str, bridge: &'a Path },
    /// The stack's own Central and Tower authority.
    Own {
        bridge: &'a Path,
        tower_host: &'a str,
    },
}

impl Auth<'_> {
    /// The gateway's environment for this mode.
    pub(super) fn environment(&self) -> Vec<(&'static str, String)> {
        let mixed = |endpoint: &str, bridge: &Path| {
            vec![
                ("BAYLEE_AUTH_MODE", "mixed".to_owned()),
                ("BAYLEE_REGISTRATION", "open".to_owned()),
                ("BAYLEE_CENTRAL_ENDPOINT", endpoint.to_owned()),
                (
                    "BAYLEE_CENTRAL_CONFIG_FILE",
                    bridge.to_string_lossy().into_owned(),
                ),
            ]
        };
        match self {
            Self::Guests => vec![("BAYLEE_AUTH_MODE", "guests".to_owned())],
            Self::Attached { endpoint, bridge } => mixed(endpoint, bridge),
            Self::Own { bridge, tower_host } => {
                let mut environment = mixed(&central::origin(central::NATIVE_PORT), bridge);
                environment.extend([
                    ("BAYLEE_TOWER_ORIGIN", central::origin(tower::PORT)),
                    ("BAYLEE_TOWER_HOST_TOKEN", (*tower_host).to_owned()),
                    ("BAYLEE_TOWER_DEVELOPMENT", "1".to_owned()),
                ]);
                environment
            }
        }
    }
}

/// The loopback listeners `--central` adds to the stack.
pub(super) const OWN_CENTRAL_PORTS: [u16; 5] = [
    central::NATIVE_PORT,
    central::BROWSER_PORT,
    central::SITE_PORT,
    central::WEBSITE_PORT,
    tower::PORT,
];

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
            central,
            stock_days,
            state_dir,
        } => up(&UpConfig {
            monitor,
            no_client,
            gateway_port,
            central_endpoint,
            central_web_origin,
            central_bridge_file,
            central,
            stock_days,
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
    // Refused before the long build: a wrong checkout, or a Central or Tower
    // someone else is already running on the fixed ports.
    let checkout = cfg
        .central
        .as_deref()
        .map(central::Checkout::open)
        .transpose()?;
    if checkout.is_some() {
        for port in OWN_CENTRAL_PORTS {
            fixture::refused(port).with_context(|| {
                format!("127.0.0.1:{port} is taken: stop the other Central or Tower first")
            })?;
        }
    }
    create_private_dir(&cfg.state_dir)?;
    let state_dir = fs::canonicalize(&cfg.state_dir)?;
    let _owner = lock_stack(&state_dir)?;
    let _ = fs::remove_file(state_dir.join(STOP_FILE));
    let bin_dir = state_dir.join("bin");
    let logs_dir = state_dir.join("logs");
    fs::create_dir_all(&logs_dir)?;

    build_and_stage(&bin_dir, !cfg.no_client)?;
    let site = match &checkout {
        Some(checkout) => {
            build_and_stage_full(&bin_dir)?;
            checkout.build(&format!("http://127.0.0.1:{}", cfg.gateway_port))?;
            Some(checkout.stage(&bin_dir, &state_dir)?)
        }
        None => None,
    };
    if state_dir.join(STOP_FILE).exists() {
        fs::remove_file(state_dir.join(STOP_FILE))?;
        println!("Local stack startup cancelled before opening any listeners.");
        return Ok(0);
    }
    let pg = LocalPostgres::start(state_dir.join("pgdata"), None, true)?;
    pg.create_database("baylee")?;
    pg.create_database("baylee_feedback_dev")?;
    if checkout.is_some() {
        pg.create_database(tower::DATABASE)?;
    }

    let own = checkout.as_ref().zip(site.as_deref());
    let supervise_res = supervise(cfg, &state_dir, &bin_dir, &logs_dir, &pg, own);
    // The two secrets a running stack keeps on disk go with it.
    for file in central::secret_files(&state_dir) {
        let _ = fs::remove_file(file);
    }
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
    own: Option<(&central::Checkout, &Path)>,
) -> Result<()> {
    let admin_port = free_loopback_port()?;
    let feedback_port = free_loopback_port()?;
    let gateway_origin = format!("http://127.0.0.1:{}", cfg.gateway_port);

    // Central first: the Tower authority delivers awards to it, and the
    // gateway asks it who a player is.
    let mut children = Vec::new();
    let own = match own {
        Some((checkout, site)) => {
            let credentials = tower::Credentials::generate()?;
            let tower_origin = central::origin(tower::PORT);
            let (started, link) = central::start(&central::Launch {
                checkout,
                pg,
                bin_dir,
                logs_dir,
                state_dir,
                site,
                gateway_origin: &gateway_origin,
                tower_origin: &tower_origin,
                tower_issuer: &credentials.issuer,
            })?;
            children.extend(started);
            children.push(tower::start(
                pg,
                bin_dir,
                logs_dir,
                &gateway_origin,
                &credentials,
            )?);
            Some((link, credentials.host))
        }
        None => None,
    };
    children.extend(spawn_backend_services(
        cfg,
        bin_dir,
        logs_dir,
        pg,
        (admin_port, feedback_port),
        &gateway_origin,
        &match (&own, &cfg.central_endpoint, &cfg.central_bridge_file) {
            (Some((link, tower_host)), _, _) => Auth::Own {
                bridge: &link.bridge_file,
                tower_host,
            },
            (None, Some(endpoint), Some(bridge)) => Auth::Attached { endpoint, bridge },
            _ => Auth::Guests,
        },
    )?);
    let linked = own.as_ref().map(|(link, _)| link);
    serve_until_stopped(
        cfg,
        (state_dir, bin_dir, logs_dir),
        Ports {
            postgres: pg.port(),
            admin: admin_port,
            feedback: feedback_port,
        },
        &mut children,
        linked,
    )
}

/// The three ports a stack picks for itself, besides the gateway's.
#[derive(Clone, Copy)]
struct Ports {
    postgres: u16,
    admin: u16,
    feedback: u16,
}

/// With every service up: the client, the state file, the ready line, the
/// task stock, and then the wait for `down`.
fn serve_until_stopped(
    cfg: &UpConfig,
    (state_dir, bin_dir, logs_dir): (&Path, &Path, &Path),
    ports: Ports,
    children: &mut [ManagedChild],
    linked: Option<&central::Linked>,
) -> Result<()> {
    let gateway_origin = format!("http://127.0.0.1:{}", cfg.gateway_port);
    let (central_endpoint, central_web_origin) = if linked.is_some() {
        (
            Some(central::origin(central::NATIVE_PORT)),
            Some(central::origin(central::WEBSITE_PORT)),
        )
    } else {
        (cfg.central_endpoint.clone(), cfg.central_web_origin.clone())
    };
    // Before the client opens: what a player is about to do, done once.
    let tower_engines = linked
        .map(|link| link.prove_play(bin_dir, logs_dir, &gateway_origin))
        .transpose()?;
    let mut client = if cfg.no_client {
        None
    } else {
        Some(spawn_interactive_client(
            bin_dir,
            logs_dir,
            &gateway_origin,
            central_endpoint.as_deref(),
            central_web_origin.as_deref(),
            cfg.monitor,
        )?)
    };

    let state = StackState {
        supervisor_pid: std::process::id(),
        postgres_port: ports.postgres,
        gateway_port: cfg.gateway_port,
        admin_port: ports.admin,
        feedback_port: ports.feedback,
        gateway_origin: gateway_origin.clone(),
        central_endpoint,
        central_web_origin: central_web_origin.clone(),
        owned_ports: if linked.is_some() {
            OWN_CENTRAL_PORTS.to_vec()
        } else {
            Vec::new()
        },
        account_file: linked.map(|link| link.account_file.clone()),
        central_revision: linked.map(|link| link.revision.clone()),
        monitor: cfg.monitor,
        logs_dir: logs_dir.to_path_buf(),
        client_pid: client.as_ref().map(ManagedChild::id),
    };
    fs::write(
        state_dir.join(STATE_FILE),
        serde_json::to_vec_pretty(&state)?,
    )?;
    if let (Some(link), Some(website)) = (linked, &central_web_origin) {
        println!(
            "Central: website {website}, Tower {}; the synthetic administrator's sign-in is in {} (0600)",
            central::origin(tower::PORT),
            link.account_file.display()
        );
        println!("Central: built from {}", link.revision);
        if let Some(engines) = tower_engines {
            println!(
                "Central: dev_admin signed in, was admitted at the gateway, and the gateway hosts the Tower with {engines} engine(s) free"
            );
        }
    }
    println!(
        "STACK_READY gateway={gateway_origin} monitor={}",
        cfg.monitor
    );
    // After the ready line: certifying tasks takes minutes to hours and the
    // game is playable without them.
    let mut stock = match linked {
        Some(link) => Some(Stock::begin(
            bin_dir,
            logs_dir,
            state_dir,
            &link.bridge_hex,
            cfg.stock_days,
            &services::probe_http_200(cfg.gateway_port, "/info")
                .as_deref()
                .and_then(version_in)
                .context("the gateway's /info names no version")?,
        )?),
        None => None,
    };
    hold_until_stopped(state_dir, children, &mut client, &mut stock)
}

/// The build's own name in the gateway's `/info` body, the same string a
/// certified task carries as its executor. Found by its key, not parsed: the
/// probe returns the body as sent, which may be chunked.
fn version_in(body: &str) -> Option<String> {
    let rest = body.split_once("\"version\":\"")?.1;
    let version = rest.split_once('"')?.0;
    (!version.is_empty() && !version.contains('\\')).then(|| version.to_owned())
}

fn spawn_backend_services(
    cfg: &UpConfig,
    bin_dir: &Path,
    logs_dir: &Path,
    pg: &LocalPostgres,
    (admin_port, feedback_port): (u16, u16),
    gateway_origin: &str,
    auth: &Auth<'_>,
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
    gw_cmd.envs(auth.environment());
    let mut gateway = ManagedChild::spawn("baylee-gateway", &mut gw_cmd, logs_dir)?;
    wait_http_ready(
        cfg.gateway_port,
        "/health",
        "\"ok\":true",
        &mut gateway,
        Duration::from_secs(20),
    )?;
    if matches!(auth, Auth::Own { .. }) {
        // A gateway that could not read its bridge still answers `/health`.
        wait_http_ready(
            cfg.gateway_port,
            "/auth/config",
            "\"central\":true",
            &mut gateway,
            Duration::from_secs(10),
        )?;
    }
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
    stock: &mut Option<Stock>,
) -> Result<()> {
    let stop_path = state_dir.join(STOP_FILE);
    while !stop_path.exists() {
        for child in children.iter_mut() {
            if let Some(status) = child.exited()? {
                bail!("{} exited unexpectedly with {status}", child.name());
            }
        }
        if let Some(stock) = stock {
            stock.poll()?;
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
        let core = [
            st.gateway_port,
            st.admin_port,
            st.feedback_port,
            st.postgres_port,
        ];
        for port in core.into_iter().chain(st.owned_ports) {
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
    if st.owned_ports.is_empty() {
        return Ok(i32::from(!gw_ok));
    }
    // The stack's own Central: each part by what it serves.
    let answers = |port: u16, path: &str| services::http_status(port, path) == Some(200);
    let parts = [
        (
            "central",
            answers(central::NATIVE_PORT, central::LIVE),
            central::NATIVE_PORT,
        ),
        (
            "website",
            answers(central::WEBSITE_PORT, "/en"),
            central::WEBSITE_PORT,
        ),
        (
            "tower",
            probe_http_200(tower::PORT, "/health").is_some_and(|body| body.contains("ready")),
            tower::PORT,
        ),
    ];
    for (name, up, port) in parts {
        println!(
            "  {name}: {} ({})",
            if up { "up" } else { "DOWN" },
            central::origin(port)
        );
    }
    for pass in [stock::Pass::Today, stock::Pass::Ahead] {
        if let Some(line) = stock::last_line(&st.logs_dir, pass) {
            println!("  {}: {line}", pass.name());
        }
    }
    if let Some(account) = &st.account_file {
        println!(
            "  synthetic administrator's sign-in: {} (0600)",
            account.display()
        );
    }
    if let Some(revision) = &st.central_revision {
        println!("  Central built from {revision}");
    }
    Ok(i32::from(!(gw_ok && parts.iter().all(|part| part.1))))
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
    if let Some(web) = &st.central_web_origin {
        cmd.env("BAYLEE_CENTRAL_WEB_ORIGIN", web);
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
