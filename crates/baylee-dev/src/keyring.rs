//! A real Secret Service round trip with no access to the user's keyring
//! (`baylee-keyring`'s `isolated_secret_service`, Linux only).
//!
//! The outer half makes a throwaway home and starts a private session bus
//! (`dbus-run-session`) running this task again with `--inside`; the inner
//! half starts `gnome-keyring-daemon` in that home, waits for its bus name
//! without reading a credential or opening a UI, and runs the ignored test.
//! Toolchains and caches stay where they are; only the daemon's view moves.
use crate::run::{self, cargo_with};
use anyhow::{Context, Result, bail};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

pub fn run(inside: bool, extra: &[String]) -> Result<i32> {
    if !cfg!(target_os = "linux") {
        bail!("the Secret Service fixture runs on Linux only");
    }
    if inside { inner(extra) } else { outer(extra) }
}

fn outer(extra: &[String]) -> Result<i32> {
    for tool in [
        "dbus-run-session",
        "gnome-keyring-daemon",
        "gdbus",
        "timeout",
    ] {
        if !run::succeeds(Command::new("sh").args(["-c", &format!("command -v {tool} >/dev/null")]))
        {
            bail!("{tool} is not installed");
        }
    }
    let fixture = std::env::temp_dir().join(format!("baylee-keyring.{}", crate::secret::hex(6)?));
    let result = (|| {
        for dir in ["data", "config", "cache", "run", "control"] {
            create_private_dir(&fixture.join(dir))?;
        }
        let home = |var: &str| std::env::var_os(var).map(PathBuf::from);
        let real_home = home("HOME").context("no HOME")?;
        let mut session = Command::new("timeout");
        session
            .args(["--kill-after=15s", "120", "dbus-run-session", "--"])
            .arg(std::env::current_exe()?)
            .args(["keyring-linux", "--inside"])
            .args(extra)
            .env(
                "CARGO_HOME",
                home("CARGO_HOME").unwrap_or_else(|| real_home.join(".cargo")),
            )
            .env(
                "RUSTUP_HOME",
                home("RUSTUP_HOME").unwrap_or_else(|| real_home.join(".rustup")),
            )
            .env("HOME", &fixture)
            .env("BAYLEE_KEYRING_FIXTURE", &fixture)
            .env("XDG_DATA_HOME", fixture.join("data"))
            .env("XDG_CONFIG_HOME", fixture.join("config"))
            .env("XDG_CACHE_HOME", fixture.join("cache"))
            .env("XDG_RUNTIME_DIR", fixture.join("run"));
        Ok(run::code(run::status(&mut session)))
    })();
    let _ = std::fs::remove_dir_all(&fixture);
    result
}

fn inner(extra: &[String]) -> Result<i32> {
    let fixture = std::env::var("BAYLEE_KEYRING_FIXTURE").context("isolated fixture required")?;
    if std::env::var("HOME").ok().as_deref() != Some(fixture.as_str()) {
        bail!("HOME is not the fixture");
    }
    let mut daemon = Command::new("gnome-keyring-daemon")
        .args(["--foreground", "--components=secrets", "--unlock"])
        .arg(format!("--control-directory={fixture}/control"))
        .stdin(Stdio::piped())
        .spawn()
        .context("starting gnome-keyring-daemon")?;
    // `--unlock` reads the login password from stdin: an empty one.
    if let Some(mut stdin) = daemon.stdin.take() {
        let _ = stdin.write_all(b"\n");
    }
    let result = (|| {
        let mut ready = false;
        for _ in 0..100 {
            let owner = Command::new("gdbus")
                .args([
                    "call",
                    "--session",
                    "--dest",
                    "org.freedesktop.DBus",
                    "--object-path",
                    "/org/freedesktop/DBus",
                ])
                .args([
                    "--method",
                    "org.freedesktop.DBus.NameHasOwner",
                    "org.freedesktop.secrets",
                ])
                .output()?;
            if String::from_utf8_lossy(&owner.stdout).contains("true") {
                ready = true;
                break;
            }
            if daemon.try_wait()?.is_some() {
                bail!("gnome-keyring-daemon exited");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !ready {
            bail!("the secret service never came up");
        }
        let mut test = cargo_with(["test", "--locked", "-p", "baylee-keyring", "--lib"]);
        test.args(extra).args([
            "platform::tests::isolated_secret_service",
            "--",
            "--ignored",
            "--exact",
        ]);
        Ok(run::code(run::status(&mut test)))
    })();
    let _ = daemon.kill();
    let _ = daemon.wait();
    result
}

#[cfg(unix)]
fn create_private_dir(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_private_dir(path: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}
