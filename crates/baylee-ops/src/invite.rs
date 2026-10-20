//! Closed-beta keys for this server's gateway (#317), installed as
//! `/usr/local/sbin/baylee-invite` by `baylee-deploy stage`:
//!
//! ```text
//! sudo baylee-invite create [--uses N] [--expires 30d|12h] [--note "Max"] [--count K]
//! sudo baylee-invite list
//! sudo baylee-invite revoke <id>
//! ```
//!
//! Runs `baylee-gateway invite` with the gateway's own settings, so it
//! reaches the gateway's database; the gateway need not be running. A key is
//! printed once, by `create`, and only its hash is kept.
use crate::settings;
use std::{path::PathBuf, process::Command};

/// Runs the gateway's `invite` with `args`; the gateway's exit status.
pub fn main(args: &[String]) -> i32 {
    // Moved only by its test.
    let etc = PathBuf::from(
        std::env::var_os("BAYLEE_INVITE_ETC").unwrap_or_else(|| "/etc/baylee".into()),
    );
    let bin = std::env::var_os("BAYLEE_INVITE_BIN")
        .unwrap_or_else(|| "/opt/baylee/bin/baylee-gateway".into());
    let files = [etc.join("secrets.env"), etc.join("gateway.env")];
    let mut env: Vec<(String, String)> = files.iter().flat_map(|f| settings::read(f)).collect();
    if !env
        .iter()
        .any(|(k, v)| k == "DATABASE_URL" && !v.is_empty())
    {
        eprintln!(
            "baylee-invite: DATABASE_URL is in neither {} nor {}",
            files[0].display(),
            files[1].display()
        );
        return 1;
    }
    // Later files win, as sourcing them in order did.
    env.reverse();
    env.dedup_by(|a, b| a.0 == b.0);
    let status = Command::new(bin)
        .arg("invite")
        .args(args)
        .envs(env)
        .status();
    status.ok().and_then(|s| s.code()).unwrap_or(1)
}
