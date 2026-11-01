//! Native shell acceptance tools for an already running development client.
mod check;
mod common;
mod devctl;
mod front;
mod keys;
mod lane;
mod measure;
mod pixels;
mod report;
mod role_contract;
mod screens;
mod settings;
#[cfg(test)]
mod tests;
mod tree;
mod walks;

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use devctl::Devctl;
use serde_json::Value;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Parser)]
#[command(
    name = "shell",
    about = "Drive an isolated, already running dev-control client; never launches one"
)]
struct Args {
    /// Loopback dev-control port (otherwise `BAYLEE_DEVCTL_PORT` or 28770).
    #[arg(long, global = true)]
    port: Option<u16>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Gallery acceptance or the current screen's centring.
    Check {
        #[command(subcommand)]
        command: Check,
    },
    /// Loopback harness GET/POST, with checked error bodies and bounded timeouts.
    Devctl {
        /// Route, e.g. /state, /key, /pause, /step, /hide, /window, /screenshot.
        route: String,
        /// JSON request file, or - for stdin. Omit for GET. Keep secrets off argv.
        #[arg(long)]
        body_file: Option<PathBuf>,
    },
    /// Five front-door faces, colophon, layout and contrast matrices.
    Front { out: PathBuf, gateway: String },
    /// Signed-in header acceptance.
    Header { out: PathBuf },
    /// Gallery Tab order, focus-ring pixels, overlay and text-size shortcuts.
    Keys { out: PathBuf },
    /// Sign in to a lane's saved, removable gateway.
    Lane {
        #[command(subcommand)]
        command: Lane,
    },
    /// Photograph selected lobby screens at selected sizes.
    Look {
        out: PathBuf,
        #[arg(value_delimiter = ',', num_args = 1, required = true)]
        screens: Vec<String>,
        sizes: String,
        #[arg(long)]
        touch: bool,
        #[arg(long,default_value="en",value_parser=["en","de"])]
        lang: String,
    },
    /// Hover rebuild counts, counted setup presses, or macOS close/Undo flush.
    Measure {
        #[command(subcommand)]
        command: Measure,
    },
    /// Inspect one screen's layout; fails if any contract fails.
    Probe {
        screen: String,
        size: String,
        #[arg(value_parser=["en","de"])]
        lang: String,
        #[arg(value_parser=["pointer","touch","auto"])]
        input: String,
        #[arg(value_parser=["xs","s","m","l","xl"])]
        step: String,
        shot: Option<PathBuf>,
    },
    /// Report-sheet W9b reference walks and four geometry dumps.
    Report {
        #[command(subcommand)]
        command: ReportCommand,
    },
    /// Lobby or an already seated room's size/language/input sweep.
    Screens {
        out: PathBuf,
        #[arg(default_value="lobby",value_parser=["lobby","room","all"])]
        which: String,
    },
    /// Ten settings sections plus unsaved model-profile sheet (no model calls).
    Settings { out: PathBuf },
    /// Keyboard-only W2/W3/W6/W7 (default: w6 w7 w3).
    Walks {
        #[arg(value_parser=["w2","w3","w6","w7"])]
        which: Vec<String>,
    },
}
#[derive(Subcommand)]
enum Check {
    Gallery { out: PathBuf },
    Centred,
}
#[derive(Subcommand)]
enum Lane {
    /// Refused since the cutover (#406): a gateway signs nobody in with a
    /// password any more.
    Login {
        /// The old command's arguments, refused unread.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
        _old: Vec<String>,
    },
}
#[derive(Subcommand)]
enum Measure {
    Hover,
    Presses,
    Quit {
        pid: u32,
        /// Local gateway (otherwise `BAYLEE_GATEWAY_URL` or <http://127.0.0.1:28931>).
        #[arg(long)]
        gateway: Option<String>,
        #[command(flatten)]
        session: measure::Session,
    },
}
#[derive(Subcommand)]
enum ReportCommand {
    Walk,
    Table,
    Dump { out: PathBuf },
}

/// Run the shell tools. The caller supplies repository root for source contracts.
///
/// # Errors
/// Returns an error for refused requests, missing fixtures, timeouts or assertions.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let Some(args) = parse(args)? else {
        return Ok(());
    };
    let port = match args.port {
        Some(p) => p,
        None => std::env::var("BAYLEE_DEVCTL_PORT")
            .unwrap_or_else(|_| "28770".into())
            .parse()?,
    };
    let d = Devctl::new(port)?;
    match args.command {
        Command::Check {
            command: Check::Gallery { out },
        } => common::gallery(&d, &out),
        Command::Check {
            command: Check::Centred,
        } => {
            let faults = check::centred(&d.nodes(None)?, d.size()?.0);
            for fault in &faults {
                println!("{fault}");
            }
            ensure!(faults.is_empty(), "{} centring faults", faults.len());
            println!("centred: 0 faults");
            Ok(())
        }
        Command::Devctl { route, body_file } => raw(&d, &route, body_file.as_deref()),
        Command::Front { out, gateway } => front::run(&d, &out, &gateway),
        Command::Header { out } => screens::header(&d, &out),
        Command::Keys { out } => keys::run(root, &d, &out),
        Command::Lane {
            command: Lane::Login { .. },
        } => lane::login(),
        Command::Look {
            out,
            screens,
            sizes,
            touch,
            lang,
        } => screens::look(&d, &out, &screens, &sizes, touch, &lang),
        Command::Measure { command } => match command {
            Measure::Hover => measure::hover(&d),
            Measure::Presses => measure::presses(&d),
            Measure::Quit {
                pid,
                gateway,
                session,
            } => measure::quit(
                &d,
                &gateway.unwrap_or_else(|| {
                    std::env::var("BAYLEE_GATEWAY_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:28931".into())
                }),
                &session.read()?,
                pid,
            ),
        },
        Command::Probe {
            screen,
            size,
            lang,
            input,
            step,
            shot,
        } => screens::probe(&d, &screen, &size, &lang, &input, &step, shot.as_deref()),
        Command::Report { command } => match command {
            ReportCommand::Walk => report::walk(&d),
            ReportCommand::Table => report::table(&d),
            ReportCommand::Dump { out } => report::dump(&d, &out),
        },
        Command::Screens { out, which } => screens::sweep(&d, &out, &which),
        Command::Settings { out } => settings::run(&d, &out),
        Command::Walks { which } => walks::run(&d, &which),
    }
}
fn parse(args: &[String]) -> Result<Option<Args>> {
    match Args::try_parse_from(std::iter::once("shell").chain(args.iter().map(String::as_str))) {
        Ok(args) => Ok(Some(args)),
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            error.print()?;
            Ok(None)
        }
        // A secret given as a positional argument must not be echoed by
        // clap's "unexpected argument" diagnostic. Do not attach its text.
        Err(_) => anyhow::bail!(
            "invalid shell arguments; use shell --help (session tokens belong in --session-env/file/stdin)"
        ),
    }
}
fn raw(d: &Devctl, route: &str, path: Option<&Path>) -> Result<()> {
    let body = path
        .map(|p| -> Result<Value> {
            let bytes = if p == Path::new("-") {
                let mut buf = Vec::new();
                io::stdin().take(1024 * 1024 + 1).read_to_end(&mut buf)?;
                buf
            } else {
                fs::read(p)?
            };
            ensure!(bytes.len() <= 1024 * 1024, "request exceeds 1 MiB");
            serde_json::from_slice(&bytes)
                .map_err(|_| anyhow::anyhow!("invalid JSON request; body withheld"))
        })
        .transpose()?;
    let timeout = Duration::from_secs(if route == "/screenshot" { 120 } else { 60 });
    let mut reply = d.http.call(route, body.as_ref(), None, timeout)?;
    redact(&mut reply);
    println!("{}", serde_json::to_string_pretty(&reply)?);
    Ok(())
}
fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = k.to_ascii_lowercase();
                if key.contains("password")
                    || key.contains("token")
                    || key.contains("secret")
                    || key == "authorization"
                    || key == "api_key"
                {
                    *v = Value::String("[redacted]".into());
                } else {
                    redact(v);
                }
            }
        }
        Value::Array(values) => {
            for v in values {
                redact(v);
            }
        }
        _ => {}
    }
}
