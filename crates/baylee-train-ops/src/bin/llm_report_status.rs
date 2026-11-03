//! The language-model seats' reports in the feedback service: `pull` them
//! with every claim, then `apply` a plan of statuses and labels to them
//! (`docs/llm-seat.md` §"Giving the reports a status"). Signs in as an
//! admin of the service's UI; the password comes from
//! `BAYLEE_FEEDBACK_ADMIN_PASSWORD`, never an argument.
#[cfg(target_os = "linux")]
mod cli {
    use anyhow::{Context, Result};
    use baylee_train_ops::collector::report_status::{self, Session};
    use clap::{Parser, Subcommand};
    use std::path::PathBuf;

    #[derive(Parser)]
    struct Args {
        /// The feedback service.
        #[arg(long, default_value = "https://feedback.baylee.app")]
        base: String,
        /// The admin to sign in as.
        #[arg(long, default_value = "claude-bigace")]
        admin: String,
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Subcommand)]
    enum Command {
        /// Every report the filter matches outside the trash, to a new JSON
        /// Lines file.
        Pull {
            /// The list's filters as a query string (`docs/feedback.md`).
            #[arg(long, default_value = "source=llm-seat")]
            filter: String,
            #[arg(long)]
            out: PathBuf,
        },
        /// The counts per status, kind, source and the rest under the filter.
        Facets {
            #[arg(long, default_value = "")]
            filter: String,
        },
        /// A plan's changes (`{"id","status","labels","revision"}` a line):
        /// counted, and made only with `--write`.
        Apply {
            #[arg(long)]
            plan: PathBuf,
            #[arg(long)]
            write: bool,
        },
    }

    fn session(args: &Args) -> Result<Session> {
        let password = std::env::var("BAYLEE_FEEDBACK_ADMIN_PASSWORD")
            .context("BAYLEE_FEEDBACK_ADMIN_PASSWORD is not set")?;
        Session::sign_in(
            &args.base,
            &args.admin,
            password.trim_end_matches(['\r', '\n']),
        )
    }

    pub fn run() -> Result<()> {
        let args = Args::parse();
        match &args.command {
            Command::Pull { filter, out } => {
                let session = session(&args)?;
                let pulled = report_status::pull(&session, filter, out)?;
                session.sign_out()?;
                println!("{{\"pulled\":{pulled}}}");
            }
            Command::Facets { filter } => {
                let session = session(&args)?;
                let facets = report_status::facets(&session, filter)?;
                session.sign_out()?;
                println!("{facets}");
            }
            Command::Apply { plan, write } => {
                let session = if *write { Some(session(&args)?) } else { None };
                let applied = report_status::apply(session.as_ref(), plan, *write)?;
                if let Some(session) = session {
                    session.sign_out()?;
                }
                println!("{}", serde_json::to_string(&applied)?);
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    cli::run()
}

#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("report administration requires Linux")
}
