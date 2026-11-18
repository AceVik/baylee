//! Baylee Central and its website, run from a sibling checkout
//! (`cargo dev stack up --central <checkout>`).
//!
//! The service, the server-rendered site and the site's assets are the
//! checkout's own, built here, so the website is whatever that checkout holds
//! and never a stale `target/site`. The database is provisioned by this
//! workspace's `baylee-dev-provision` at the Central revision the workspace
//! pins; the checkout must be at a revision with the same migrations and
//! runtime grants, and a different one is refused by name.
#![forbid(unsafe_code)]

use super::{
    postgres::LocalPostgres,
    provision,
    services::{ManagedChild, stage_file, wait_http_ok, wait_tcp, write_private},
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

/// Central's native endpoint. The client's development build expects the
/// website four ports above it, and the website's crypto module is built for
/// this origin, so these are fixed.
pub(super) const NATIVE_PORT: u16 = 28790;
/// The browser API the website's `/api/*` is passed to.
pub(super) const BROWSER_PORT: u16 = 28791;
/// The server-rendered site behind the website's front.
pub(super) const SITE_PORT: u16 = 28792;
/// The website as a browser opens it: what "Register" in the client links to.
pub(super) const WEBSITE_PORT: u16 = 28794;

/// Central's liveness route on its native listener.
pub(super) const LIVE: &str = "/health/live";

const DATABASE: &str = "central_dev";
/// The synthetic administrator every fresh stack starts with.
const ACCOUNT: &str = "dev_admin";
/// This workspace's copy of Central's `ops/grants.sql` at the pinned revision.
const PINNED_GRANTS: &str = "crates/baylee-gateway/tests/central/runtime-grants.sql";
const FRONT: &str = "crates/central-web/browser/e2e-real/front.ts";
const FALLBACK: &str = "crates/central-web-server/tests/data";
const MIGRATIONS: &str = "crates/central-migration/src";

pub(super) fn origin(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// A Baylee Central checkout this stack may build and run.
pub(super) struct Checkout {
    root: PathBuf,
}

impl Checkout {
    /// Refuses, before anything is built, a directory that is not a Central
    /// checkout this workspace can provision, and a machine without `node`.
    pub(super) fn open(path: &Path) -> Result<Self> {
        let root = fs::canonicalize(path)
            .with_context(|| format!("--central {} does not exist", path.display()))?;
        for needed in [
            "crates/central-service/Cargo.toml",
            "crates/central-web-server/Cargo.toml",
            "ops/grants.sql",
            FRONT,
            FALLBACK,
            MIGRATIONS,
        ] {
            ensure!(
                root.join(needed).exists(),
                "{} is not a Baylee Central checkout with a website: {needed} is missing",
                root.display()
            );
        }
        ensure!(
            fs::read(root.join("ops/grants.sql"))? == fs::read(PINNED_GRANTS)?,
            "{}/ops/grants.sql differs from {PINNED_GRANTS}: the checkout is not at a revision \
             the workspace's Central pin can provision",
            root.display()
        );
        ensure!(
            Command::new("node")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success()),
            "the website's local front needs `node` on PATH"
        );
        Ok(Self { root })
    }

    /// Builds the service, the site server and the site, the latter for this
    /// stack's loopback Central and gateway instead of the production hosts.
    pub(super) fn build(&self, gateway_origin: &str) -> Result<()> {
        ensure!(
            self.cargo(&[
                "build",
                "--locked",
                "-p",
                "central-service",
                "-p",
                "central-web-server"
            ])
            .status()
            .is_ok_and(|status| status.success()),
            "building Central in {} failed",
            self.root.display()
        );
        ensure!(
            self.cargo(&["xtask", "site"])
                .env("BAYLEE_CENTRAL_DEV_ENDPOINT", origin(NATIVE_PORT))
                .env("BAYLEE_CENTRAL_DEV_GATEWAY", gateway_origin)
                .status()
                .is_ok_and(|status| status.success()),
            "building Central's website in {} failed",
            self.root.display()
        );
        Ok(())
    }

    /// The checkout's branch and commit, so the stack says which website it
    /// serves: a sibling checkout on an old branch looks like lost work.
    pub(super) fn revision(&self) -> String {
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(args)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map_or_else(|| "?".to_owned(), |text| text.trim().to_owned())
        };
        format!(
            "{} at {} ({})",
            self.root.display(),
            git(&["rev-parse", "--short", "HEAD"]),
            git(&["rev-parse", "--abbrev-ref", "HEAD"])
        )
    }

    /// A cargo that resolves the checkout's own toolchain and target
    /// directory, not the ones `cargo dev` was started with.
    fn cargo(&self, args: &[&str]) -> Command {
        let mut cargo = Command::new("cargo");
        cargo.args(args).current_dir(&self.root);
        for (name, _) in std::env::vars_os() {
            if name_is_this_workspaces(&name) {
                cargo.env_remove(name);
            }
        }
        cargo
    }

    /// Copies the two executables and the built site into the stack's state,
    /// so a later build in the checkout never changes a running stack.
    /// Returns the staged site.
    pub(super) fn stage(&self, bin_dir: &Path, state_dir: &Path) -> Result<PathBuf> {
        let built = self.root.join("target/debug");
        for name in ["central-service", "central-web"] {
            let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            stage_file(&built.join(file), bin_dir, name)?;
        }
        let site = state_dir.join("central-site");
        if site.exists() {
            fs::remove_dir_all(&site)?;
        }
        copy_tree(&self.root.join("target/site"), &site)
            .context("staging Central's built website")?;
        Ok(site)
    }

    /// The migrations the checkout's service was built with, by file name.
    fn migrations(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(self.root.join(MIGRATIONS))? {
            let path = entry?.path();
            if let Some(stem) = path.file_stem().and_then(OsStr::to_str)
                && path.extension() == Some(OsStr::new("rs"))
                && is_migration(stem)
            {
                names.push(stem.to_owned());
            }
        }
        names.sort();
        Ok(names)
    }
}

/// `m20261019_000001_pending_accounts`, not `lib`.
fn is_migration(stem: &str) -> bool {
    stem.strip_prefix('m')
        .is_some_and(|rest| rest.as_bytes().first().is_some_and(u8::is_ascii_digit))
}

/// Variables that tie a cargo to this workspace: the package `cargo dev` runs
/// as, the toolchain rustup chose for it and a redirected target directory.
pub(super) fn name_is_this_workspaces(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        name.starts_with("CARGO_PKG_")
            || name.starts_with("CARGO_MANIFEST_")
            || matches!(
                name,
                "CARGO"
                    | "CARGO_CRATE_NAME"
                    | "CARGO_BIN_NAME"
                    | "CARGO_PRIMARY_PACKAGE"
                    | "CARGO_TARGET_DIR"
                    | "RUSTUP_TOOLCHAIN"
            )
    })
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// What the rest of the stack needs from a running Central.
pub(super) struct Linked {
    /// The gateway's `BAYLEE_CENTRAL_CONFIG_FILE`, `0600`, removed at stop.
    pub bridge_file: PathBuf,
    /// The bridge capability as the challenge publisher takes it, in its
    /// environment only.
    pub bridge_hex: String,
    /// The synthetic administrator's sign-in, `0600`, removed at stop.
    pub account_file: PathBuf,
    /// Which checkout the service and the website were built from.
    pub revision: String,
    /// The synthetic administrator's password, for [`Linked::prove_play`].
    password: String,
}

impl Linked {
    /// With the gateway and its agent up: the synthetic administrator signs
    /// in, is admitted at the gateway and asks it for its Tower capacity. The
    /// gateway answers that only once the Tower authority accepted it as a
    /// host, with the Tower engines its agents have free; a stack that says
    /// ready without it has a Tower nobody can play. Returns how many
    /// engines are free.
    pub(super) fn prove_play(
        &self,
        bin_dir: &Path,
        logs_dir: &Path,
        gateway_origin: &str,
    ) -> Result<u64> {
        let reply = provision::ask(
            bin_dir,
            logs_dir,
            "signin",
            &json!({
                "endpoint": origin(NATIVE_PORT),
                "username": ACCOUNT,
                "password": self.password,
                "gateway": gateway_origin,
            }),
        )
        .context("the synthetic administrator cannot reach the Tower through the gateway")?;
        let tower = origin(super::tower::PORT);
        ensure!(
            reply["tower"]["origin"] == json!(tower),
            "the gateway hosts for another Tower authority than {tower}"
        );
        let engines = reply["tower"]["engines"].as_u64().unwrap_or(0);
        ensure!(
            engines > 0,
            "the gateway's agents offer no Tower engine the authority knows"
        );
        Ok(engines)
    }
}

/// The files a running stack keeps its two on-disk secrets in.
pub(super) fn secret_files(state_dir: &Path) -> [PathBuf; 2] {
    [
        state_dir.join("gateway-central-bridge.json"),
        state_dir.join("central-account.json"),
    ]
}

pub(super) struct Launch<'a> {
    pub checkout: &'a Checkout,
    pub pg: &'a LocalPostgres,
    pub bin_dir: &'a Path,
    pub logs_dir: &'a Path,
    pub state_dir: &'a Path,
    pub site: &'a Path,
    pub gateway_origin: &'a str,
    pub tower_origin: &'a str,
    /// The Tower authority's award credential, which Central must know.
    pub tower_issuer: &'a str,
}

/// Provisions a fresh Central in the stack's PostgreSQL and starts the
/// service, the site server and the website's front, in that order.
pub(super) fn start(launch: &Launch<'_>) -> Result<(Vec<ManagedChild>, Linked)> {
    let fresh = provision(launch)?;
    let children = serve(launch, &fresh)?;
    prove_sign_in(launch, &fresh)?;
    Ok((children, keep_secrets(launch, &fresh)?))
}

/// Signs the synthetic administrator in against the running service, the way
/// the client does: the keys in the database and the ones the service was
/// started with meet only there, and a stack whose one account cannot sign in
/// is not ready.
fn prove_sign_in(launch: &Launch<'_>, fresh: &Fresh) -> Result<()> {
    provision::ask(
        launch.bin_dir,
        launch.logs_dir,
        "signin",
        &json!({
            "endpoint": origin(NATIVE_PORT),
            "username": ACCOUNT,
            "password": fresh.password,
        }),
    )
    .context("the synthetic administrator cannot sign in to the Central just started")?;
    Ok(())
}

/// A provisioned database and the secrets its service must be started with.
struct Fresh {
    runtime: String,
    legacy: String,
    /// The synthetic administrator's password.
    password: String,
    /// The helper's reply: the OPAQUE setup, the bridge and the token seed.
    reply: Value,
}

fn provision(launch: &Launch<'_>) -> Result<Fresh> {
    let (migrator, runtime, legacy) = (
        crate::secret::hex(32)?,
        crate::secret::hex(32)?,
        crate::secret::hex(32)?,
    );
    let mut bootstrap = crate::central::roles(&migrator, &runtime, &legacy);
    bootstrap.push_str(&crate::central::databases(&[DATABASE]));
    launch
        .pg
        .sql("postgres", &bootstrap)
        .context("creating Central's roles and database")?;

    let password = crate::secret::hex(12)?;
    let reply = provision::ask(
        launch.bin_dir,
        launch.logs_dir,
        "central",
        &json!({
            "postgres_port": launch.pg.port(),
            "database": DATABASE,
            "migrator_password": migrator,
            "runtime_password": runtime,
            "legacy_password": legacy,
            "endpoint": origin(NATIVE_PORT),
            "gateway": launch.gateway_origin,
            "tower": launch.tower_origin,
            "issuer": launch.tower_issuer,
            "account": {
                "username": ACCOUNT,
                "display_name": "DevAdmin",
                "password": password,
            },
        }),
    )?;
    let applied: Vec<String> = serde_json::from_value(reply["migrations"].clone())
        .context("the helper named no migrations")?;
    if let Some(difference) = first_difference(&applied, &launch.checkout.migrations()?) {
        bail!(
            "the Central checkout's migrations are not the pinned revision's ({difference}); \
             use a checkout at the pin, or move the pin"
        );
    }
    Ok(Fresh {
        runtime,
        legacy,
        password,
        reply,
    })
}

fn serve(launch: &Launch<'_>, fresh: &Fresh) -> Result<Vec<ManagedChild>> {
    let mut children = Vec::with_capacity(3);
    let settings = service_settings(launch, fresh);
    let mut service = ManagedChild::spawn_with_input(
        "central-service",
        Command::new(super::services::staged_exe(
            launch.bin_dir,
            "central-service",
        ))
        .arg("--dev-accounts-stdin"),
        launch.logs_dir,
        &serde_json::to_vec(&settings)?,
    )?;
    wait_http_ok(NATIVE_PORT, LIVE, &mut service, Duration::from_secs(20))?;
    wait_tcp(BROWSER_PORT, &mut service, Duration::from_secs(20))?;
    children.push(service);

    let mut site = ManagedChild::spawn_with_input(
        "central-web",
        Command::new(super::services::staged_exe(launch.bin_dir, "central-web"))
            .arg("--config-stdin"),
        launch.logs_dir,
        &serde_json::to_vec(&site_settings(
            launch.site,
            &launch.checkout.root.join(FALLBACK),
        ))?,
    )?;
    wait_tcp(SITE_PORT, &mut site, Duration::from_secs(20))?;
    children.push(site);

    let mut front = ManagedChild::spawn(
        "central-front",
        Command::new("node")
            .arg(launch.checkout.root.join(FRONT))
            .args(["--site", &SITE_PORT.to_string()])
            .args(["--port", &WEBSITE_PORT.to_string()])
            .args(["--listener", &BROWSER_PORT.to_string()]),
        launch.logs_dir,
    )?;
    // Through the front to the site server: the page as a browser gets it.
    wait_http_ok(WEBSITE_PORT, "/en", &mut front, Duration::from_secs(20))?;
    children.push(front);
    Ok(children)
}

/// Writes the two secrets a running stack keeps on disk.
fn keep_secrets(launch: &Launch<'_>, fresh: &Fresh) -> Result<Linked> {
    let bridge: Vec<u8> = serde_json::from_value(fresh.reply["bridge"].clone())
        .context("the helper made no bridge")?;
    let [bridge_file, account_file] = secret_files(launch.state_dir);
    write_private(
        &bridge_file,
        &serde_json::to_vec(&json!({
            "endpoint": origin(NATIVE_PORT),
            "bridge": fresh.reply["bridge"],
            "origin": launch.gateway_origin,
        }))?,
    )?;
    write_private(
        &account_file,
        &serde_json::to_vec_pretty(&json!({
            "username": ACCOUNT,
            "password": fresh.password,
            "website": origin(WEBSITE_PORT),
            "note": "A synthetic administrator on this local stack only; it is gone when the stack stops.",
        }))?,
    )?;
    Ok(Linked {
        bridge_file,
        bridge_hex: hex(&bridge),
        account_file,
        revision: launch.checkout.revision(),
        password: fresh.password.clone(),
    })
}

/// `central-service --dev-accounts-stdin`'s settings.
fn service_settings(launch: &Launch<'_>, fresh: &Fresh) -> Value {
    json!({
        "endpoint": origin(NATIVE_PORT),
        "gateway": launch.gateway_origin,
        "tower_origin": launch.tower_origin,
        "database_port": launch.pg.port(),
        "database_user": "central_runtime",
        "database_name": DATABASE,
        "database_password": fresh.runtime,
        "opaque_setup": fresh.reply["opaque_setup"],
        "bridge": fresh.reply["bridge"],
        "token_keys": { "active": fresh.reply["token_seed"], "published": [] },
        "browser": {
            "listen": format!("127.0.0.1:{BROWSER_PORT}"),
            "origin": origin(WEBSITE_PORT),
        },
        "legacy": {
            "database_user": "central_legacy",
            "database_password": fresh.legacy,
        },
    })
}

/// `central-web --config-stdin`'s settings: no secret in them.
pub(super) fn site_settings(site: &Path, fallback: &Path) -> Value {
    json!({
        "listen": format!("127.0.0.1:{SITE_PORT}"),
        "origin": origin(WEBSITE_PORT),
        "site": site,
        "central": format!("127.0.0.1:{BROWSER_PORT}"),
        "github": false,
        "fallback": fallback,
    })
}

/// What tells two ordered migration lists apart, or `None` when they agree.
pub(super) fn first_difference(applied: &[String], checkout: &[String]) -> Option<String> {
    let only =
        |from: &[String], other: &[String]| from.iter().find(|name| !other.contains(name)).cloned();
    if let Some(name) = only(checkout, applied) {
        return Some(format!("the checkout adds {name}"));
    }
    only(applied, checkout).map(|name| format!("the checkout lacks {name}"))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}
