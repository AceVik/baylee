//! Owned loopback-only native PostgreSQL 18 lifecycle for local stacks and gates.
//!
//! Secrets stay in memory: `initdb` reads a `0600` password file that is
//! deleted immediately after bootstrap, and child processes receive credentials
//! through their environment rather than argv, files or logs.
#![forbid(unsafe_code)]

use crate::fixture;
use anyhow::{Context, Result, bail, ensure};
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write as _,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

const CANDIDATE_DIRS: &[&str] = &[
    "/opt/homebrew/opt/postgresql@18/bin",
    "/usr/local/opt/postgresql@18/bin",
    "/usr/lib/postgresql/18/bin",
];

/// Resolved paths to the four PostgreSQL 18 utilities we invoke.
struct PgTools {
    initdb: PathBuf,
    pg_ctl: PathBuf,
    psql: PathBuf,
    pg_isready: PathBuf,
}

impl PgTools {
    fn discover() -> Result<Self> {
        if let Some(dir) = std::env::var_os("BAYLEE_PG_BIN") {
            return Self::from_dir(Path::new(&dir));
        }
        for candidate in CANDIDATE_DIRS {
            let dir = Path::new(candidate);
            if dir.join("pg_ctl").is_file() {
                return Self::from_dir(dir);
            }
        }
        Self::from_path()
    }

    fn from_dir(dir: &Path) -> Result<Self> {
        let tool = |name: &str| -> Result<PathBuf> {
            let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
            ensure!(path.is_file(), "missing PostgreSQL tool {}", path.display());
            Ok(path)
        };
        Ok(Self {
            initdb: tool("initdb")?,
            pg_ctl: tool("pg_ctl")?,
            psql: tool("psql")?,
            pg_isready: tool("pg_isready")?,
        })
    }

    fn from_path() -> Result<Self> {
        let path_var = std::env::var_os("PATH").context("PATH is unset; set BAYLEE_PG_BIN")?;
        let find = |name: &str| -> Result<PathBuf> {
            let bin = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            std::env::split_paths(&path_var)
                .map(|dir| dir.join(&bin))
                .find(|candidate| candidate.is_file())
                .with_context(|| {
                    format!("PostgreSQL 18 `{name}` not found on PATH; install postgresql@18 or set BAYLEE_PG_BIN")
                })
        };
        Ok(Self {
            initdb: find("initdb")?,
            pg_ctl: find("pg_ctl")?,
            psql: find("psql")?,
            pg_isready: find("pg_isready")?,
        })
    }
}

/// An owned loopback PostgreSQL instance backed by a private local directory.
pub(super) struct LocalPostgres {
    tools: PgTools,
    data_dir: PathBuf,
    port: u16,
    admin_password: String,
    remove_on_finish: bool,
    active: bool,
}

impl LocalPostgres {
    /// Bootstraps and starts a fresh PostgreSQL cluster in `data_dir`.
    pub(super) fn start(
        data_dir: PathBuf,
        port: Option<u16>,
        remove_on_finish: bool,
    ) -> Result<Self> {
        let tools = PgTools::discover()?;
        claim_data_dir(&data_dir)?;
        let port = match port {
            Some(explicit) => explicit,
            None => free_loopback_port()?,
        };
        let admin_password = crate::secret::hex(32)?;
        init_cluster(&tools.initdb, &data_dir, &admin_password)?;

        let mut instance = Self {
            tools,
            data_dir,
            port,
            admin_password,
            remove_on_finish,
            active: true,
        };
        if let Err(err) = instance.boot() {
            let _ = instance.stop_internal();
            return Err(err);
        }
        Ok(instance)
    }

    pub(super) const fn port(&self) -> u16 {
        self.port
    }

    /// Connection URL for `database`, kept in memory for child environments.
    pub(super) fn url(&self, database: &str) -> String {
        format!(
            "postgres://postgres:{}@127.0.0.1:{}/{database}",
            self.admin_password, self.port
        )
    }

    /// Creates `database` if it does not exist yet.
    pub(super) fn create_database(&self, database: &str) -> Result<()> {
        ensure!(
            !database.is_empty()
                && database
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "invalid database identifier: {database}"
        );
        self.sql("postgres", &format!("CREATE DATABASE {database};"))
            .with_context(|| format!("creating database {database} failed"))
    }

    /// Runs `sql` in `database` as the cluster's superuser. The script goes
    /// over stdin and psql's output is discarded, so a password in it reaches
    /// no argument list and no log.
    pub(super) fn sql(&self, database: &str, sql: &str) -> Result<()> {
        let mut child = Command::new(&self.tools.psql)
            .env("PGPASSWORD", &self.admin_password)
            .args([
                "-h",
                "127.0.0.1",
                "-p",
                &self.port.to_string(),
                "-X",
                "-q",
                "-v",
                "ON_ERROR_STOP=1",
                "-U",
                "postgres",
                "-d",
                database,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("spawning psql")?;
        child
            .stdin
            .as_mut()
            .context("psql stdin")?
            .write_all(sql.as_bytes())?;
        drop(child.stdin.take());
        ensure!(child.wait()?.success(), "psql refused the script");
        Ok(())
    }

    /// Stops the cluster cleanly and proves its loopback port closed.
    pub(super) fn finish(mut self) -> Result<()> {
        self.stop_internal()
    }

    fn boot(&self) -> Result<()> {
        // Keep the Unix socket in `/tmp` so deep workspace paths on macOS never
        // overflow `sockaddr_un.sun_path` (104 bytes).
        let socket_dir = if cfg!(unix) {
            PathBuf::from("/tmp")
        } else {
            self.data_dir.clone()
        };
        let options = format!(
            "-h 127.0.0.1 -p {} -k {} -F -c shared_buffers=256MB -c max_connections=200",
            self.port,
            socket_dir.display()
        );
        let mut start = Command::new(&self.tools.pg_ctl);
        start
            .arg("-D")
            .arg(&self.data_dir)
            .arg("-l")
            .arg(self.data_dir.join("postgres.log"))
            .args(["-o", &options, "start"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if !names_a_locale(std::env::var_os("LC_ALL"), std::env::var_os("LANG")) {
            // The cluster is `--no-locale` already; this only keeps the
            // postmaster from asking macOS for one.
            start.env("LC_ALL", "C");
        }
        let status = start.status().context("starting pg_ctl")?;
        ensure!(status.success(), "pg_ctl failed to start local PostgreSQL");
        let ready = (0..40).any(|_| {
            let ok = Command::new(&self.tools.pg_isready)
                .args(["-q", "-h", "127.0.0.1", "-p", &self.port.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            if !ok {
                std::thread::sleep(Duration::from_millis(200));
            }
            ok
        });
        ensure!(
            ready,
            "local PostgreSQL did not become ready on 127.0.0.1:{}",
            self.port
        );
        Ok(())
    }

    fn stop_internal(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        stop_existing_dir(&self.tools.pg_ctl, &self.data_dir);
        // Never remove a live cluster after a failed stop. Drop retries it.
        fixture::refused(self.port).with_context(|| {
            format!(
                "PostgreSQL listener on 127.0.0.1:{} remained open",
                self.port
            )
        })?;
        self.active = false;
        if self.remove_on_finish && self.data_dir.exists() {
            let _ = fs::remove_dir_all(&self.data_dir);
        }
        Ok(())
    }
}

impl Drop for LocalPostgres {
    fn drop(&mut self) {
        if self.active {
            let _ = self.stop_internal();
        }
    }
}

fn init_cluster(initdb: &Path, data_dir: &Path, password: &str) -> Result<()> {
    let pw_path = data_dir
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".pgpass-{}", crate::secret::hex(8)?));
    let write_res = write_ephemeral_secret(&pw_path, password);
    if let Err(err) = write_res {
        let _ = fs::remove_file(&pw_path);
        return Err(err);
    }
    let out = Command::new(initdb)
        .arg("-D")
        .arg(data_dir)
        .args([
            "-U",
            "postgres",
            "--auth-local=scram-sha-256",
            "--auth-host=scram-sha-256",
            "--encoding=UTF8",
            "--no-locale",
        ])
        .arg(format!("--pwfile={}", pw_path.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
    let _ = fs::remove_file(&pw_path);
    let out = out.context("running initdb")?;
    if !out.status.success() {
        bail!(
            "initdb failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Whether the environment names a locale for every category. Without one,
/// macOS answers `setlocale(…, "")` from the user's preferences on a second
/// thread, and the postmaster refuses to start ("became multithreaded during
/// startup"): a shell started by a desktop app has neither variable.
pub(super) fn names_a_locale(lc_all: Option<OsString>, lang: Option<OsString>) -> bool {
    [lc_all, lang]
        .into_iter()
        .flatten()
        .any(|value| !value.is_empty())
}

fn stop_existing_dir(pg_ctl: &Path, data_dir: &Path) {
    let _ = Command::new(pg_ctl)
        .arg("-D")
        .arg(data_dir)
        .args(["-m", "immediate", "stop"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Atomically claim a fresh directory. Existing data, including symlinks, belongs
/// to someone else until proven otherwise; starting a fixture never deletes it.
pub(super) fn claim_data_dir(path: &Path) -> Result<()> {
    fs::create_dir(path).with_context(|| {
        format!(
            "refusing to replace PostgreSQL data at {}; use a fresh state directory",
            path.display()
        )
    })?;
    create_private_dir(path)
}

pub(super) fn create_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("creating {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_ephemeral_secret(path: &Path, secret: &str) -> Result<()> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        opts.mode(0o600);
    }
    let mut file = opts.open(path)?;
    writeln!(file, "{secret}")?;
    file.flush()
        .context("flushing ephemeral initdb password file")
}

pub(super) fn free_loopback_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}
