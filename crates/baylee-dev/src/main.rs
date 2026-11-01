//! The workspace's development tasks, one cross-platform binary instead of
//! shell scripts: `cargo dev <task>` (the alias is in `.cargo/config.toml`).
//! Each task keeps what its script learned; the comments say why a step has
//! the shape it has. Every task runs from the checkout's root.
mod api;
mod central;
mod changes;
mod deploy;
mod devctl;
mod features;
mod fixture;
mod gate;
mod keyring;
mod land;
mod mobile;
mod run;
mod secret;
mod shipping;

use clap::{Parser, Subcommand};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(name = "cargo dev", about = "Baylee's development tasks", version)]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// The gates; without a name, the full gate CI runs (needs `DATABASE_URL`).
    Gate {
        #[command(subcommand)]
        which: Option<Gate>,
    },
    /// The full and desktop feature gates, then a fast-forward push to main.
    Land {
        /// Gate HEAD as it is and push nothing.
        #[arg(long)]
        check: bool,
        /// Internal: the part that runs under the machine's cargo lock.
        #[arg(long, hide = true)]
        locked: bool,
    },
    /// Classifies changed paths read from stdin for `ci.yml` (`key=true|false`).
    CiChanges,
    /// Deploys the committed HEAD to `BAYLEE_DEPLOY_HOST`.
    Deploy {
        /// End open games instead of waiting for them.
        #[arg(long)]
        force: bool,
    },
    /// Builds the client as an APK (and with `--run` installs and starts it).
    Android {
        #[command(subcommand)]
        env: Option<AndroidEnv>,
        /// The gateway the app dials (default: this machine's LAN address).
        #[arg(long)]
        gateway: Option<String>,
        /// dev-control's port inside the app.
        #[arg(long, default_value_t = 28770)]
        devctl: u16,
        /// Build without dev-control.
        #[arg(long)]
        no_dev_control: bool,
        /// Install and start it (`cargo apk run`).
        #[arg(long)]
        run: bool,
        /// An optimised build.
        #[arg(long)]
        release: bool,
    },
    /// Builds the client for the iOS simulator and starts it there (macOS).
    IosSim {
        #[arg(long, default_value = "iPhone 15 Pro")]
        device: String,
        #[arg(long, default_value = "http://127.0.0.1:28766")]
        gateway: String,
        #[arg(long, default_value_t = 28770)]
        devctl: u16,
    },
    /// Starts a gateway and an agent a phone on this network can reach.
    ServeLan {
        /// Only print the address to use from each kind of client.
        #[arg(long)]
        print: bool,
    },
    /// The isolated Secret Service round trip of `baylee-keyring` (Linux).
    KeyringLinux {
        /// Internal: the half that runs inside the private session bus.
        #[arg(long, hide = true)]
        inside: bool,
        /// Further arguments for `cargo test`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Writes the API documents in `docs/api/` from the servers' sources.
    Api {
        #[command(subcommand)]
        which: Option<Api>,
    },
    /// Central's integration checks against an owned PostgreSQL container.
    Central {
        #[command(subcommand)]
        check: Central,
    },
}

#[derive(Subcommand)]
enum Api {
    /// Writes nothing; fails on a stale document or an undocumented route.
    Check,
    /// Shows the documents in Swagger UI on 127.0.0.1 only.
    Serve {
        #[arg(long, default_value_t = 28768)]
        port: u16,
    },
}

#[derive(Subcommand)]
enum AndroidEnv {
    /// Prints the Android build environment for a shell to take over.
    Env,
}

#[derive(Subcommand)]
enum Central {
    /// `baylee-db`'s schema suite, then any named gateway e2e targets.
    Db {
        /// Gateway e2e targets to add (e.g. `e2e_account`).
        #[arg(long = "gateway-test")]
        gateway_tests: Vec<String>,
        /// A local cargo patch configuration for sibling Central crates.
        #[arg(long)]
        cargo_config: Option<PathBuf>,
        /// Fail right after start-up to prove the cleanup path.
        #[arg(long)]
        exercise_failure: bool,
    },
    /// A real gateway and Central against one isolated database.
    Integration {
        #[arg(long)]
        cargo_config: Option<PathBuf>,
        /// End early once this file exists.
        #[arg(long)]
        cancel_file: Option<PathBuf>,
        #[arg(long)]
        exercise_failure: bool,
    },
    /// The dev-control client signing in against the integration fixture.
    Client {
        /// The additional left monitor's index.
        #[arg(long, default_value_t = 0)]
        monitor: i64,
    },
}

#[derive(Subcommand)]
enum Gate {
    /// Everything but the client and wasm; stops at the first red.
    Rules,
    /// fmt and clippy over changed `.rs` files (not a gate).
    Touched {
        /// Compare with this revision instead of the working tree.
        since: Option<String>,
    },
    /// Local-only development features `--workspace` never builds.
    Features,
    /// Ensures verification hooks are absent from shipped feature graphs.
    ShippedFeatures,
    /// Experimental browser crates for wasm32 (coming soon; not a landing gate).
    Wasm,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (name, result) = match run::enter_root() {
        Err(error) => ("dev", Err(error)),
        Ok(_) => match cli.task {
            Task::Gate { which: None } => ("gate", gate::full()),
            Task::Gate {
                which: Some(Gate::Rules),
            } => ("gate rules", Ok(gate::rules())),
            Task::Gate {
                which: Some(Gate::Touched { since }),
            } => ("gate touched", gate::touched(since.as_deref())),
            Task::Gate {
                which: Some(Gate::Features),
            } => ("gate features", Ok(features::run())),
            Task::Gate {
                which: Some(Gate::ShippedFeatures),
            } => ("gate shipped-features", Ok(shipping::run())),
            Task::Gate {
                which: Some(Gate::Wasm),
            } => ("gate wasm", Ok(gate::wasm())),
            Task::Land { check, locked } => ("land", land::run(check, locked)),
            Task::CiChanges => ("ci-changes", changes::run()),
            Task::Api { which } => (
                "api",
                Ok(match which {
                    None => api::run(&["write"]),
                    Some(Api::Check) => api::run(&["check"]),
                    Some(Api::Serve { port }) => api::run(&["serve", "--port", &port.to_string()]),
                }),
            ),
            Task::Deploy { force } => ("deploy", deploy::run(force)),
            Task::Android {
                env: Some(AndroidEnv::Env),
                ..
            } => ("android env", mobile::print_env()),
            Task::Android {
                env: None,
                gateway,
                devctl,
                no_dev_control,
                run,
                release,
            } => (
                "android",
                mobile::android(&mobile::Android {
                    gateway,
                    devctl,
                    dev_control: !no_dev_control,
                    run,
                    release,
                }),
            ),
            Task::IosSim {
                device,
                gateway,
                devctl,
            } => ("ios-sim", mobile::ios_sim(&device, &gateway, devctl)),
            Task::ServeLan { print } => ("serve-lan", mobile::serve_lan(print)),
            Task::KeyringLinux { inside, args } => ("keyring-linux", keyring::run(inside, &args)),
            Task::Central {
                check:
                    Central::Db {
                        gateway_tests,
                        cargo_config,
                        exercise_failure,
                    },
            } => (
                "central db",
                central::db(&gateway_tests, cargo_config.as_deref(), exercise_failure),
            ),
            Task::Central {
                check:
                    Central::Integration {
                        cargo_config,
                        cancel_file,
                        exercise_failure,
                    },
            } => (
                "central integration",
                central::integration(
                    cargo_config.as_deref(),
                    cancel_file.as_deref(),
                    exercise_failure,
                ),
            ),
            Task::Central {
                check: Central::Client { monitor },
            } => ("central client", central::client(monitor)),
        },
    };
    match result {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(error) => {
            eprintln!("{name}: {error:#}");
            ExitCode::FAILURE
        }
    }
}
