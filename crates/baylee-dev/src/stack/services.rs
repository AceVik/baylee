//! Build staging, child supervision and loopback HTTP readiness for `cargo dev stack`.
#![forbid(unsafe_code)]

use crate::run::{self, cargo_with};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read as _, Write as _},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// Non-secret metadata written to `<state_dir>/stack.json` while a local stack runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct StackState {
    pub supervisor_pid: u32,
    pub postgres_port: u16,
    pub gateway_port: u16,
    pub admin_port: u16,
    pub feedback_port: u16,
    pub gateway_origin: String,
    pub central_endpoint: Option<String>,
    /// The website the client's browser links open, when the stack knows one.
    #[serde(default)]
    pub central_web_origin: Option<String>,
    /// Further loopback listeners this stack owns (`--central`: Central, its
    /// website and the Tower authority); `down` proves each closed.
    #[serde(default)]
    pub owned_ports: Vec<u16>,
    /// Where the synthetic administrator's sign-in is written, `0600`; the
    /// path only, never its contents.
    #[serde(default)]
    pub account_file: Option<PathBuf>,
    /// The Central checkout, commit and branch the website was built from.
    #[serde(default)]
    pub central_revision: Option<String>,
    pub monitor: i64,
    pub logs_dir: PathBuf,
    pub client_pid: Option<u32>,
}

/// An owned child process killed and reaped when dropped.
pub(super) struct ManagedChild {
    name: &'static str,
    child: Child,
}

impl ManagedChild {
    pub(super) fn spawn(name: &'static str, cmd: &mut Command, logs_dir: &Path) -> Result<Self> {
        Self::start(name, cmd, logs_dir, None)
    }

    /// A service that reads its whole configuration, secrets included, from
    /// stdin: written once and closed, so it reaches no file or argument.
    pub(super) fn spawn_with_input(
        name: &'static str,
        cmd: &mut Command,
        logs_dir: &Path,
        input: &[u8],
    ) -> Result<Self> {
        Self::start(name, cmd, logs_dir, Some(input))
    }

    fn start(
        name: &'static str,
        cmd: &mut Command,
        logs_dir: &Path,
        input: Option<&[u8]>,
    ) -> Result<Self> {
        let out = File::create(logs_dir.join(format!("{name}.out.log")))?;
        let err = File::create(logs_dir.join(format!("{name}.err.log")))?;
        let child = cmd
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err))
            .spawn()
            .with_context(|| format!("spawning {name}"))?;
        // Owned from here on: a failed write still kills and reaps the child.
        let mut managed = Self { name, child };
        if let Some(input) = input {
            let mut stdin = managed.child.stdin.take().context("child stdin")?;
            stdin
                .write_all(input)
                .with_context(|| format!("handing {name} its configuration"))?;
        }
        Ok(managed)
    }

    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    pub(super) fn exited(&mut self) -> Result<Option<std::process::ExitStatus>> {
        Ok(self.child.try_wait()?)
    }

    pub(super) fn name(&self) -> &'static str {
        self.name
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// Builds and copies the stack binaries into `bin_dir` so recompiles while
/// playing never overwrite a running executable.
pub(super) fn build_and_stage(bin_dir: &Path, with_client: bool) -> Result<()> {
    fs::create_dir_all(bin_dir)?;
    let mut packages = vec![
        "build",
        "-p",
        "baylee-gateway",
        "-p",
        "baylee-agent",
        "-p",
        "baylee-feedback",
        "-p",
        "baylee-engine-server",
    ];
    if with_client {
        // Interactive local play deliberately builds WITHOUT `dev-control` so
        // the window never runs harnessed or passive.
        packages.extend(["-p", "baylee-client"]);
    }
    ensure!(
        run::succeeds(&mut cargo_with(packages)),
        "building local stack binaries failed"
    );
    let mut names = vec![
        "baylee-gateway",
        "baylee-agent",
        "baylee-feedback",
        "baylee-engine-server",
    ];
    if with_client {
        names.push("baylee-client");
    }
    for name in names {
        stage_debug_bin(name, name, bin_dir)?;
    }

    ensure!(
        run::succeeds(&mut cargo_with([
            "build",
            "-p",
            "baylee-engine-server",
            "--no-default-features",
        ])),
        "building standard engine variant failed"
    );
    stage_debug_bin("baylee-engine-server", "baylee-engine-standard", bin_dir)?;

    ensure!(
        run::succeeds(&mut cargo_with([
            "build",
            "-p",
            "baylee-engine-server",
            "--no-default-features",
            "--features",
            "tower",
        ])),
        "building Tower engine variant failed"
    );
    stage_debug_bin("baylee-engine-server", "baylee-engine-tower", bin_dir)
}

/// Builds and stages what `--central` adds from this workspace: the Tower
/// authority, the provisioning helper and the challenge publisher, from the
/// same commit as the engines (a challenge is certified for one engine build).
pub(super) fn build_and_stage_full(bin_dir: &Path) -> Result<()> {
    ensure!(
        run::succeeds(&mut cargo_with([
            "build",
            "-p",
            "baylee-tower-service",
            "-p",
            "baylee-dev-provision",
        ])),
        "building the Tower authority and the provisioning helper failed"
    );
    ensure!(
        run::succeeds(&mut cargo_with([
            "build",
            "-p",
            "baylee-train",
            "--bin",
            "challenge",
        ])),
        "building the challenge publisher failed"
    );
    for name in ["baylee-tower-service", "baylee-dev-provision", "challenge"] {
        stage_debug_bin(name, name, bin_dir)?;
    }
    Ok(())
}

/// Copies a built executable into `bin_dir`, replacing an older one.
pub(super) fn stage_file(source: &Path, bin_dir: &Path, name: &str) -> Result<()> {
    let destination = staged_exe(bin_dir, name);
    // Unlink first on Unix so replacing an executable never writes through an active inode.
    let _ = fs::remove_file(&destination);
    fs::copy(source, &destination).with_context(|| {
        format!(
            "staging {} from {}",
            destination.display(),
            source.display()
        )
    })?;
    Ok(())
}

/// Writes a file only this user can read, replacing an older one. For the two
/// secrets a running stack must keep on disk: the gateway reads its Central
/// bridge from a file, and the owner reads the synthetic account's sign-in.
pub(super) fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    let _ = fs::remove_file(path);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(contents)?;
    file.flush()?;
    Ok(())
}

/// Builds and stages only the interactive desktop client.
pub(super) fn build_and_stage_client(bin_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(bin_dir)?;
    ensure!(
        run::succeeds(&mut cargo_with(["build", "-p", "baylee-client"])),
        "building interactive baylee-client failed"
    );
    stage_debug_bin("baylee-client", "baylee-client", bin_dir)?;
    Ok(staged_exe(bin_dir, "baylee-client"))
}

pub(super) fn staged_exe(bin_dir: &Path, name: &str) -> PathBuf {
    bin_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

fn stage_debug_bin(src_name: &str, dst_name: &str, bin_dir: &Path) -> Result<()> {
    let suffix = std::env::consts::EXE_SUFFIX;
    let src = PathBuf::from("target/debug").join(format!("{src_name}{suffix}"));
    stage_file(&src, bin_dir, dst_name)
}

/// Waits until `127.0.0.1:port` accepts a TCP connection or `child` exits.
pub(super) fn wait_tcp(port: u16, child: &mut ManagedChild, timeout: Duration) -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(status) = child.exited()? {
            bail!("{} exited early with {status}", child.name());
        }
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    bail!("timed out waiting for {} on 127.0.0.1:{port}", child.name())
}

/// Waits until an HTTP `GET path` on `127.0.0.1:port` returns 200 containing `needle`.
pub(super) fn wait_http_ready(
    port: u16,
    path: &str,
    needle: &str,
    child: &mut ManagedChild,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(status) = child.exited()? {
            bail!("{} exited early with {status}", child.name());
        }
        if probe_http_200(port, path).is_some_and(|body| body.contains(needle)) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    bail!(
        "timed out waiting for {} HTTP readiness on http://127.0.0.1:{port}{path}",
        child.name()
    )
}

/// Performs a bounded loopback HTTP/1.1 `GET` and returns the body on `200 OK`.
pub(super) fn probe_http_200(port: u16, path: &str) -> Option<String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut buf = Vec::new();
    stream.take(65_536).read_to_end(&mut buf).ok()?;
    parse_http_200_body(&buf)
}

pub(super) fn parse_http_200_body(raw: &[u8]) -> Option<String> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    let status = head.lines().next()?.split_whitespace().nth(1)?;
    if status != "200" {
        return None;
    }
    String::from_utf8(raw[split + 4..].to_vec()).ok()
}

/// Waits until `GET path` on `127.0.0.1:port` answers `200`, whatever its
/// body: a page is larger than the body probe reads, and need not be told
/// apart from another page.
pub(super) fn wait_http_ok(
    port: u16,
    path: &str,
    child: &mut ManagedChild,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(status) = child.exited()? {
            bail!("{} exited early with {status}", child.name());
        }
        if http_status(port, path) == Some(200) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    bail!(
        "timed out waiting for {} to serve http://127.0.0.1:{port}{path}",
        child.name()
    )
}

/// The status of a bounded loopback `GET`, read from the response's first
/// bytes only.
pub(super) fn http_status(port: u16, path: &str) -> Option<u16> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut head = Vec::new();
    stream.take(64).read_to_end(&mut head).ok()?;
    parse_http_status(&head)
}

/// `HTTP/1.1 200 OK` → 200, from as much of the status line as was read.
pub(super) fn parse_http_status(head: &[u8]) -> Option<u16> {
    let line = head.split(|byte| *byte == b'\r').next()?;
    let mut words = std::str::from_utf8(line).ok()?.split_whitespace();
    words
        .next()
        .filter(|version| version.starts_with("HTTP/"))?;
    words.next()?.parse().ok()
}
