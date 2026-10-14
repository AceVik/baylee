//! Login into a lane's already running local client. Secrets never enter argv.
use super::devctl::{Devctl, sleep};
use anyhow::{Context, Result, ensure};
use clap::Args;
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};
#[derive(Args)]
pub(super) struct Credentials {
    /// Environment variable containing the lane password (its value is never printed).
    #[arg(long="password-env",conflicts_with_all=["file","stdin"])]
    pub env: Option<String>,
    /// File containing the lane password. A final newline is removed.
    #[arg(long = "password-file", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// Read the lane password from standard input (not an interactive prompt).
    #[arg(long = "password-stdin")]
    pub stdin: bool,
}
impl Credentials {
    pub fn read(&self) -> Result<String> {
        let mut password = if let Some(name) = &self.env {
            std::env::var(name)
                .map_err(|_| anyhow::anyhow!("password environment variable is unavailable"))?
        } else if let Some(path) = &self.file {
            fs::read_to_string(path).map_err(|_| anyhow::anyhow!("cannot read password file"))?
        } else if self.stdin {
            let mut value = String::new();
            io::stdin()
                .take(65537)
                .read_to_string(&mut value)
                .map_err(|_| anyhow::anyhow!("cannot read password stdin"))?;
            value
        } else {
            std::env::var("BAYLEE_SHELL_PASSWORD").map_err(|_|anyhow::anyhow!("supply --password-env, --password-file or --password-stdin (or BAYLEE_SHELL_PASSWORD)"))?
        };
        if password.ends_with('\n') {
            password.pop();
            if password.ends_with('\r') {
                password.pop();
            }
        }
        ensure!(
            !password.is_empty() && password.len() <= 65536,
            "password is empty or too long"
        );
        Ok(password)
    }
}
pub(super) fn login(
    d: &Devctl,
    user: &str,
    password: &str,
    gateway_row: Option<&str>,
) -> Result<()> {
    if !d.has("Front(LeaveGateway)")? {
        let names = d.names()?;
        let pick = gateway_row
            .map(str::to_owned)
            .or_else(|| {
                names
                    .iter()
                    .find(|n| n.starts_with("Front(ForgetGateway("))
                    .map(|n| n.replace("ForgetGateway", "SelectGateway"))
            })
            .context("no local gateway saved")?;
        // Explicit override may only select a removable lane gateway, never
        // the pinned production gateway. The lane still needs local config.
        ensure!(
            names.contains(&pick.replace("SelectGateway", "ForgetGateway")),
            "gateway row must be a removable lane gateway"
        );
        d.press_until(&pick, || d.has("Shared(Focus(Username))"), 8)?;
        sleep(1.0);
    }
    d.press("Shared(Focus(Username))")?;
    for _ in 0..24 {
        d.key("Backspace", None, 0.0)?;
    }
    d.typed(user, 0.0)?;
    d.press("Shared(Focus(Password))")?;
    for _ in 0..32 {
        d.key("Backspace", None, 0.0)?;
    }
    d.typed(password, 0.0)?;
    d.press("Front(Submit)")?;
    Devctl::until(20.0, || d.has("Header(Nav(0))"))?;
    sleep(3.0);
    println!("signed in");
    Ok(())
}
