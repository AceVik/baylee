//! Owned throwaway processes and an owned PostgreSQL container.
//!
//! Ownership is checked, not assumed: the container gets a random name and a
//! label, and cleanup removes only a container carrying both, then proves it
//! gone and its published port refusing connections. A probe that times out
//! is not proof of closure. The admin password reaches `docker create` in its
//! environment, never in argv, and no secret is printed.
use anyhow::{Context, Result, bail};
use std::{
    io::{ErrorKind, Read},
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// The official multi-architecture PostgreSQL 18 image, pinned by digest
/// rather than a moving tag.
pub const POSTGRES_IMAGE: &str =
    "postgres@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873";
const LABEL: &str = "baylee.central.test";

/// A PostgreSQL container this run created and will remove.
pub struct Postgres {
    name: String,
    id: Option<String>,
    port: Option<u16>,
    admin: String,
}

impl Postgres {
    /// Creates and starts a read-only container with its data on a bounded
    /// tmpfs, published on IPv4 loopback only. `memory` is docker's limit,
    /// `data_bytes` the data filesystem's size.
    pub fn start(prefix: &str, memory: &str, data_bytes: u64) -> Result<Self> {
        let mut fixture = Self {
            name: format!("{prefix}-{}", crate::secret::hex(6)?),
            id: None,
            port: None,
            admin: crate::secret::hex(32)?,
        };
        println!("Owned PostgreSQL fixture: {}", fixture.name);
        if let Err(error) = fixture.boot(memory, data_bytes) {
            let cleanup = fixture.finish();
            return Err(match cleanup {
                Ok(()) => error,
                Err(cleanup) => error.context(cleanup),
            });
        }
        Ok(fixture)
    }

    fn boot(&mut self, memory: &str, data_bytes: u64) -> Result<()> {
        if !quiet(Command::new("docker").args(["image", "inspect", POSTGRES_IMAGE])) {
            let pulled = Command::new("docker")
                .args(["pull", POSTGRES_IMAGE])
                .status()?;
            if !pulled.success() {
                bail!("pinned PostgreSQL image unavailable");
            }
        }
        let created = Command::new("docker")
            .args([
                "create",
                "--name",
                &self.name,
                "--label",
                &format!("{LABEL}=owned"),
            ])
            .args([
                "--memory",
                memory,
                "--cpus",
                "2",
                "--pids-limit",
                "128",
                "--read-only",
            ])
            .args([
                "--tmpfs",
                &format!("/var/lib/postgresql:rw,nosuid,nodev,size={data_bytes}"),
            ])
            .args([
                "--tmpfs",
                "/var/run/postgresql:rw,nosuid,nodev",
                "--tmpfs",
                "/tmp:rw,nosuid,nodev",
            ])
            .args(["--publish", "127.0.0.1::5432", "--env", "POSTGRES_PASSWORD"])
            .args(["--env", "PGDATA=/var/lib/postgresql/test", POSTGRES_IMAGE])
            .env("POSTGRES_PASSWORD", &self.admin)
            .output()?;
        if !created.status.success() {
            bail!("creating the isolated PostgreSQL fixture failed");
        }
        let id = String::from_utf8(created.stdout)?.trim().to_owned();
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("invalid owned container identity");
        }
        self.id = Some(id.clone());
        if !quiet(Command::new("docker").args(["start", &id])) {
            bail!("starting the PostgreSQL fixture failed");
        }
        let ready = (0..50).any(|_| {
            let ok = quiet(Command::new("docker").args([
                "exec",
                &id,
                "pg_isready",
                "-q",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
            ]));
            if !ok {
                std::thread::sleep(Duration::from_millis(500));
            }
            ok
        });
        if !ready {
            bail!("the PostgreSQL fixture did not become ready");
        }
        let port = Command::new("docker")
            .args(["port", &id, "5432/tcp"])
            .output()?;
        let port = String::from_utf8(port.stdout)?;
        let port = port
            .trim()
            .strip_prefix("127.0.0.1:")
            .and_then(|p| p.parse().ok())
            .context("the fixture must publish exclusively on IPv4 loopback")?;
        self.port = Some(port);
        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port.unwrap_or_default()
    }

    /// The superuser's URL, for the suites that make their own schemas.
    pub fn admin_url(&self) -> String {
        format!(
            "postgres://postgres:{}@127.0.0.1:{}/postgres",
            self.admin,
            self.port()
        )
    }

    /// Runs SQL as the superuser; on failure its diagnostics are withheld,
    /// since they can echo the statement and its generated secrets.
    pub fn sql(&self, sql: &str) -> Result<()> {
        let mut psql = Command::new("docker")
            .args([
                "exec",
                "-i",
                self.id(),
                "psql",
                "-X",
                "-q",
                "-v",
                "ON_ERROR_STOP=1",
                "-U",
                "postgres",
                "-d",
                "postgres",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        std::io::Write::write_all(psql.stdin.as_mut().context("psql stdin")?, sql.as_bytes())?;
        drop(psql.stdin.take());
        if !psql.wait()?.success() {
            bail!("fixture SQL failed (diagnostics withheld)");
        }
        Ok(())
    }

    pub fn version(&self) -> Result<String> {
        let out = Command::new("docker")
            .args([
                "exec",
                self.id(),
                "psql",
                "-X",
                "-At",
                "-U",
                "postgres",
                "-c",
                "SHOW server_version",
            ])
            .output()?;
        Ok(String::from_utf8(out.stdout)?.trim().to_owned())
    }

    /// Reclaims generated WAL between suites, never while a child is active.
    pub fn checkpoint(&self) -> Result<()> {
        if !quiet(Command::new("docker").args([
            "exec",
            self.id(),
            "psql",
            "-X",
            "-q",
            "-U",
            "postgres",
            "-c",
            "CHECKPOINT",
        ])) {
            bail!("fixture checkpoint failed");
        }
        Ok(())
    }

    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or_default()
    }

    /// Removes the container if it is ours and proves it and its listener gone.
    pub fn finish(mut self) -> Result<()> {
        let result = self.remove();
        self.id = None;
        self.port = None;
        result
    }

    fn remove(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        // The preallocated name also finds a container an interruption left
        // half-created, before its id was known.
        match Command::new("docker")
            .args([
                "ps",
                "-a",
                "--no-trunc",
                "--filter",
                &format!("name=^/{}$", self.name),
            ])
            .args([
                "--format",
                &format!("{{{{.ID}}}} {{{{.Label \"{LABEL}\"}}}}"),
            ])
            .output()
        {
            Ok(out) if out.status.success() => {
                let found = String::from_utf8_lossy(&out.stdout).trim().to_owned();
                if !found.is_empty()
                    && let Err(e) = self.remove_found(&found)
                {
                    errors.push(format!(
                        "owned PostgreSQL container cleanup could not be verified: {e}"
                    ));
                }
            }
            _ => errors.push("container inventory unavailable".to_owned()),
        }
        if let Some(port) = self.port
            && let Err(e) = refused(port)
        {
            errors.push(format!(
                "owned PostgreSQL listener closure could not be verified: {e}"
            ));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors.join("; "))
        }
    }

    fn remove_found(&self, found: &str) -> Result<()> {
        let id = found
            .strip_suffix(" owned")
            .context("ownership marker mismatch")?;
        if self.id.as_deref().is_some_and(|known| known != id) {
            bail!("container identity changed");
        }
        if !quiet(Command::new("docker").args(["rm", "--force", "--volumes", id])) {
            bail!("container removal failed");
        }
        let left = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--no-trunc",
                "--filter",
                &format!("id={id}"),
                "--format",
                "{{.ID}}",
            ])
            .output()?;
        if !left.status.success() || !String::from_utf8_lossy(&left.stdout).trim().is_empty() {
            bail!("container removal unverified");
        }
        Ok(())
    }
}

impl Drop for Postgres {
    /// A panic between start and [`Postgres::finish`] still removes the container.
    fn drop(&mut self) {
        if self.id.is_some() || self.port.is_some() {
            let _ = self.remove();
        }
    }
}

/// Proves a loopback port refuses connections; a timeout proves nothing.
pub fn refused(port: u16) -> Result<()> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    match TcpStream::connect_timeout(&address, Duration::from_secs(5)) {
        Ok(_) => bail!("the listener remains"),
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => Ok(()),
        Err(e) => bail!("probe failed: {e}"),
    }
}

fn quiet(command: &mut Command) -> bool {
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A child with its output read on background threads, so neither pipe can
/// fill and stall it.
pub struct Owned {
    child: Child,
    readers: Vec<JoinHandle<String>>,
}

/// What an [`Owned`] child left behind.
pub struct Finished {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl Owned {
    pub fn spawn(command: &mut Command) -> Result<Self> {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("starting an owned process")?;
        let mut readers = Vec::new();
        if let Some(mut out) = child.stdout.take() {
            readers.push(std::thread::spawn(move || drain(&mut out)));
        }
        if let Some(mut err) = child.stderr.take() {
            readers.push(std::thread::spawn(move || drain(&mut err)));
        }
        Ok(Self { child, readers })
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn exited(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_some()
    }

    /// Waits up to `limit` for the child to exit on its own, killing nothing.
    pub fn wait_for(&mut self, limit: Duration) -> Result<bool> {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if self.child.try_wait()?.is_some() {
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Ok(false)
    }

    /// Waits up to `limit`; an existing `cancel` file ends the wait early.
    /// Either way past the deadline the child's whole tree is killed and that
    /// is an error.
    pub fn wait(mut self, limit: Duration, cancel: Option<&Path>) -> Result<Finished> {
        let deadline = Instant::now() + limit;
        let status = loop {
            if let Some(status) = self.child.try_wait()? {
                break status;
            }
            if cancel.is_some_and(Path::exists) {
                self.kill()?;
                bail!("fixture cancelled by its owner");
            }
            if Instant::now() >= deadline {
                self.kill()?;
                bail!("an owned process timed out");
            }
            std::thread::sleep(Duration::from_millis(250));
        };
        let mut texts = self.readers.drain(..).map(|r| r.join().unwrap_or_default());
        Ok(Finished {
            status,
            stdout: texts.next().unwrap_or_default(),
            stderr: texts.next().unwrap_or_default(),
        })
    }

    /// Kills the child's tree and returns what it had written.
    pub fn kill_and_collect(mut self) -> Result<Finished> {
        self.kill()?;
        let status = self.child.wait()?;
        let mut texts = self.readers.drain(..).map(|r| r.join().unwrap_or_default());
        Ok(Finished {
            status,
            stdout: texts.next().unwrap_or_default(),
            stderr: texts.next().unwrap_or_default(),
        })
    }

    /// Kills the child and everything it started, then waits until it is
    /// gone. A tree, not a process: killing `cargo test` alone would leave
    /// its test binary, and the servers that binary started, running.
    pub fn kill(&mut self) -> Result<()> {
        kill_tree(self.child.id());
        let _ = self.child.kill();
        self.child
            .wait()
            .context("owned process cleanup could not be verified")?;
        Ok(())
    }
}

#[cfg(unix)]
fn kill_tree(pid: u32) {
    // Children first are collected before anything dies, so a reparented
    // grandchild is still found under its original parent.
    let mut pending = vec![pid];
    let mut tree = Vec::new();
    while let Some(parent) = pending.pop() {
        if let Ok(out) = Command::new("pgrep")
            .args(["-P", &parent.to_string()])
            .output()
        {
            for child in String::from_utf8_lossy(&out.stdout)
                .split_whitespace()
                .filter_map(|p| p.parse().ok())
            {
                pending.push(child);
                tree.push(child);
            }
        }
    }
    for child in tree.iter().rev() {
        let _ = Command::new("kill")
            .args(["-KILL", &child.to_string()])
            .status();
    }
}

#[cfg(windows)]
fn kill_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .output();
}

fn drain(stream: &mut impl Read) -> String {
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}
