//! `baylee-feedback`: keeps the reports gateways pass on (`docs/feedback.md`).
//!
//! `FEEDBACK_DATABASE_URL` (required), `FEEDBACK_GATEWAY_TOKENS`
//! (`name=token,…`), `FEEDBACK_READ_TOKEN`, `FEEDBACK_ADMIN_TOKEN`,
//! `FEEDBACK_BIND` (`127.0.0.1:28780`), `FEEDBACK_POOL` (4),
//! `FEEDBACK_WEB_DIR` (the built web UI; unset = none),
//! `FEEDBACK_TRUSTED_PROXIES`.
//!
//! ```text
//! baylee-feedback                      serve
//! baylee-feedback admin add <name>     the password is one line on stdin
//! baylee-feedback admin remove <name>  and every session they hold
//! baylee-feedback admin list
//! ```

use std::io::{BufRead as _, IsTerminal as _, Write as _};
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context as _, bail};
use axum::serve::ListenerExt as _;
use tracing_subscriber::EnvFilter;

const USAGE: &str = "usage: baylee-feedback [admin add <name> | admin remove <name> | admin list]";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    // The admin commands answer a person at a terminal: only warnings, so
    // the migrator's and Postgres's notices never bury the one line that
    // says what happened (a refused password went unseen that way).
    let filter = match args.first() {
        Some(&"admin") => EnvFilter::new("warn"),
        _ => EnvFilter::from_default_env(),
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    match args.as_slice() {
        [] => serve().await,
        ["admin", rest @ ..] => admin(rest).await,
        ["-h" | "--help" | "help"] => {
            println!("{USAGE}");
            Ok(())
        }
        _ => bail!("{USAGE}"),
    }
}

async fn database() -> anyhow::Result<sea_orm::DatabaseConnection> {
    let url = std::env::var("FEEDBACK_DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .ok_or_else(|| anyhow::anyhow!("FEEDBACK_DATABASE_URL is not set"))?;
    let pool = std::env::var("FEEDBACK_POOL")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4);
    baylee_feedback::connect(&url, pool).await
}

async fn serve() -> anyhow::Result<()> {
    let config = baylee_feedback::Config::from_env()?;
    let ui = baylee_feedback::ui::Ui::from_env()?;
    let web = ui.web_dir().map(|d| d.display().to_string());
    let db = database().await?;
    let bind = std::env::var("FEEDBACK_BIND").unwrap_or_else(|_| "127.0.0.1:28780".to_owned());
    // Nagle's algorithm off on every accepted connection (`TCP_NODELAY`), as
    // on the gateway: a reply is not held back for an acknowledgement that
    // Linux delays by up to 40 ms.
    let listener = tokio::net::TcpListener::bind(&bind).await?.tap_io(|tcp| {
        if let Err(e) = tcp.set_nodelay(true) {
            tracing::debug!(error = %e, "TCP_NODELAY was not set on a connection");
        }
    });
    tracing::info!(
        bind,
        web = web.as_deref().unwrap_or("off"),
        version = baylee_build::short(),
        "baylee-feedback serving"
    );
    let app = baylee_feedback::app_with(Arc::new(baylee_feedback::AppState { db, config }), ui);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

/// The password for `admin add`: one line of stdin. On a terminal it is
/// asked for twice, without echo.
fn read_password() -> anyhow::Result<String> {
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        return Ok(baylee_feedback::admin::password_line(&line).to_owned());
    }
    let echo = |on: bool| {
        let _ = std::process::Command::new("stty")
            .arg(if on { "echo" } else { "-echo" })
            .stdin(std::process::Stdio::inherit())
            .status();
    };
    let ask = |prompt: &str| -> anyhow::Result<String> {
        eprint!("{prompt}");
        std::io::stderr().flush()?;
        echo(false);
        let mut line = String::new();
        let read = std::io::stdin().lock().read_line(&mut line);
        echo(true);
        eprintln!();
        read?;
        Ok(baylee_feedback::admin::password_line(&line).to_owned())
    };
    let first = ask("password: ")?;
    if ask("again: ")? != first {
        bail!("the two passwords differ");
    }
    Ok(first)
}

async fn admin(args: &[&str]) -> anyhow::Result<()> {
    match args {
        ["add", name] => {
            let password = read_password().context("reading the password")?;
            let db = database().await?;
            baylee_feedback::admin::add(&db, name, &password).await?;
            println!("admin {name} added");
        }
        ["remove", name] => {
            let db = database().await?;
            if baylee_feedback::admin::remove(&db, name).await? {
                println!("admin {name} removed, with every session");
            } else {
                bail!("no admin named {name}");
            }
        }
        ["list"] => {
            let db = database().await?;
            let admins = baylee_feedback::admin::list(&db).await?;
            if admins.is_empty() {
                println!("(no admins)");
            }
            for admin in admins {
                println!(
                    "{}\tsince {}\t{} session(s)",
                    admin.name, admin.created, admin.sessions
                );
            }
        }
        _ => bail!("{USAGE}"),
    }
    Ok(())
}
