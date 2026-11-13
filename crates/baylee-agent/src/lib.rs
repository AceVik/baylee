//! baylee-agent — the thing that starts engines.
//!
//! The gateway routes and the engine plays; neither of them spawns a process.
//! An agent registers with a gateway, is told to start an engine for a game,
//! and starts one — passing it the gateway to dial and the game to play.
//!
//! ```text
//!   gateway ── StartEngine{game, token, url} ──> agent
//!                                                 │ spawn
//!                                                 v
//!   gateway <────────── EngineHello ────────── engine
//! ```
//!
//! It depends on the control protocol and nothing else in the workspace: an
//! agent has never heard of a card, a rule or a deck, which is exactly why it
//! can run on a machine the gateway does not.
//!
//! This module is the decisions; `main.rs` is the socket and the processes.
#![deny(unsafe_code)]
#![warn(missing_docs)]

use baylee_protocol::v1::{self, Envelope};
use std::path::PathBuf;

/// Where the control socket is on the gateway.
pub const CONTROL_PATH: &str = "/agent/ws";

/// What `baylee-agent --help` prints, and a refused command line after its
/// reason.
pub const USAGE: &str = "\
usage: baylee-agent                 dial the gateway and start one engine per game it orders
       baylee-agent --help | -h     this text

It takes no argument and opens no port; its settings are environment
variables: BAYLEE_GATEWAY (default http://127.0.0.1:28766; unix:<path> for
the gateway's unix socket), BAYLEE_AGENT_TOKEN (required), BAYLEE_AGENT_NAME,
BAYLEE_AGENT_CAPACITY (0 = no limit), BAYLEE_ENGINE_BIN (default: the
engine server beside this program) and BAYLEE_FORESIGHT_SLOTS (a directory
the engines share their search slots in; default: one this agent makes for
its own engines). Its engines read the rest of the BAYLEE_FORESIGHT_*
settings from the environment they inherit.";

/// What a command line asks the agent for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Nothing: run.
    Run,
    /// `--help` or `-h`: print [`USAGE`], start nothing.
    Help,
}

/// Reads a command line, without the program's name. The agent takes no
/// argument but a request for help, so anything else stops it before it
/// reads a setting or dials a gateway.
///
/// # Errors
/// Any other argument. The refusal names a flag up to its `=` and repeats no
/// value: a value may be a secret.
pub fn asked(args: &[String]) -> Result<Asked, String> {
    match args.first().map(String::as_str) {
        None => Ok(Asked::Run),
        Some("--help" | "-h") => Ok(Asked::Help),
        Some(flag) if flag.starts_with('-') => {
            let name = flag.split_once('=').map_or(flag, |(name, _)| name);
            Err(format!(
                "{name:?} is not a flag this program takes; its settings are environment \
                 variables"
            ))
        }
        Some(_) => Err(
            "this program takes no argument; its settings are environment variables".to_string(),
        ),
    }
}

/// How this agent was configured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentConfig {
    /// The gateway's HTTP base, e.g. `http://127.0.0.1:28766`.
    pub gateway: String,
    /// The shared secret the gateway checks. Without one the agent refuses to
    /// start: an unauthenticated agent is a way to run processes on this
    /// machine at somebody else's request.
    pub token: String,
    /// Human label, for logs and for the operator.
    pub name: String,
    /// How many engines to run at once. 0 means no limit.
    pub capacity: u32,
    /// The engine binary to spawn.
    pub engine_bin: PathBuf,
    /// The exact build of the fixed engine binary; verified again at engine attach.
    pub executor: String,
    /// Where the engines take their search slots
    /// (`BAYLEE_FORESIGHT_SLOTS`, `baylee-engine-server`'s `foresight`): the
    /// operator's directory, shared with whatever else they start there, or
    /// `None` for one this agent makes for its own engines
    /// ([`slots_parent`]).
    pub foresight_slots: Option<PathBuf>,
}

/// The variable an engine reads its search slots' directory from.
pub const SLOTS_VAR: &str = "BAYLEE_FORESIGHT_SLOTS";

/// Where an agent makes its own slots' directory: the service's runtime
/// directory where systemd gave it one (`RUNTIME_DIRECTORY`, the first of
/// several), else the user's (`XDG_RUNTIME_DIR`), else the system's temporary
/// directory. The first two are not swept by age, as `/tmp` may be.
#[must_use]
pub fn slots_parent(env: impl Fn(&str) -> Option<String>) -> PathBuf {
    let dir = |name: &str| {
        env(name)
            .and_then(|dirs| dirs.split(':').next().map(PathBuf::from))
            .filter(|dir| dir.is_absolute())
    };
    dir("RUNTIME_DIRECTORY")
        .or_else(|| dir("XDG_RUNTIME_DIR"))
        .unwrap_or_else(std::env::temp_dir)
}

impl AgentConfig {
    /// Reads the configuration from the environment.
    ///
    /// # Errors
    /// When no token is configured, or when the engine binary cannot be found.
    pub fn from_env() -> Result<Self, String> {
        Self::read(|name| std::env::var(name).ok(), default_engine_bin)
    }

    /// The reading itself, with the two things it asks the machine handed in.
    ///
    /// Separated so that it can be read at all. `std::env` is one table per
    /// process and a test runner is threads, so a test that set these
    /// variables would be a test that changed what some other test was
    /// reading at that moment. The rules below are all of the quiet kind —
    /// an empty variable, a value that will not parse, a default nobody
    /// states — which is exactly the kind that goes wrong without anybody
    /// noticing.
    ///
    /// # Errors
    /// As [`Self::from_env`].
    fn read(
        env: impl Fn(&str) -> Option<String>,
        default_bin: impl Fn() -> Option<PathBuf>,
    ) -> Result<Self, String> {
        let var = |name: &str| env(name).filter(|v| !v.is_empty());
        let token =
            var("BAYLEE_AGENT_TOKEN").ok_or_else(|| "BAYLEE_AGENT_TOKEN is not set".to_string())?;
        let engine_bin = match var("BAYLEE_ENGINE_BIN") {
            Some(path) => PathBuf::from(path),
            None => default_bin().ok_or_else(|| {
                "cannot find baylee-engine-server; set BAYLEE_ENGINE_BIN".to_string()
            })?,
        };
        let executor = var("BAYLEE_ENGINE_BUILD").unwrap_or_else(|| baylee_build::short().into());
        if executor.len() > 160 || executor.bytes().any(|byte| !(32..=126).contains(&byte)) {
            return Err("BAYLEE_ENGINE_BUILD must be a bounded printable build identity".into());
        }
        Ok(Self {
            gateway: var("BAYLEE_GATEWAY")
                .unwrap_or_else(|| "http://127.0.0.1:28766".to_string())
                .trim_end_matches('/')
                .to_string(),
            token,
            name: var("BAYLEE_AGENT_NAME").unwrap_or_else(|| "agent".to_string()),
            capacity: var("BAYLEE_AGENT_CAPACITY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            engine_bin,
            executor,
            foresight_slots: var(SLOTS_VAR).map(PathBuf::from),
        })
    }

    /// The control socket this agent connects to.
    ///
    /// For a gateway on a unix socket (`unix:<path>`) this is the URL the
    /// handshake names; the socket file is what is dialled.
    #[must_use]
    pub fn control_url(&self) -> String {
        if baylee_protocol::unix_socket(&self.gateway).is_some() {
            return format!("ws://localhost{CONTROL_PATH}");
        }
        format!("{}{CONTROL_PATH}", ws_base(&self.gateway))
    }

    /// What an engine this agent starts is told to dial.
    ///
    /// The gateway names an address in its order (`BAYLEE_ENGINE_URL`, TCP
    /// loopback by default). An agent that itself reached the gateway on its
    /// unix socket hands its engines that socket instead: they run on the
    /// same machine, and a game is "local" to the gateway because its engine
    /// came in on the socket — the same way the agent did.
    #[must_use]
    pub fn engine_address<'a>(&'a self, ordered: &'a str) -> &'a str {
        if baylee_protocol::unix_socket(&self.gateway).is_some() {
            &self.gateway
        } else {
            ordered
        }
    }

    /// The agent's opening frame.
    #[must_use]
    pub fn hello(&self) -> Envelope {
        Envelope {
            msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
                token: self.token.clone(),
                name: self.name.clone(),
                capacity: self.capacity,
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                executor: self.executor.clone(),
            })),
        }
    }

    /// Whether one more engine fits.
    #[must_use]
    pub fn has_room(&self, running: usize) -> bool {
        self.capacity == 0 || running < self.capacity as usize
    }
}

/// The websocket form of an HTTP base URL.
///
/// Anything that is already a websocket URL is passed through: guessing at it
/// would be worse than letting the socket refuse it.
#[must_use]
pub fn ws_base(http: &str) -> String {
    let base = http.trim_end_matches('/');
    match base.split_once("://") {
        Some(("https", rest)) => format!("wss://{rest}"),
        Some(("http", rest)) => format!("ws://{rest}"),
        _ => base.to_string(),
    }
}

/// What the agent should do about one control frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Order {
    /// The gateway accepted this agent.
    Welcomed {
        /// The id the gateway knows this agent by.
        agent_id: String,
        /// How often to send a heartbeat, in seconds.
        heartbeat_secs: u32,
    },
    /// Start an engine for a game.
    Start {
        /// The game to play.
        game_id: String,
        /// What the engine proves itself with.
        engine_token: String,
        /// The websocket the engine dials.
        gateway_url: String,
    },
    /// Stop the engine for a game.
    Stop {
        /// The game that is over.
        game_id: String,
    },
    /// The gateway refused this agent, and said why (#271): a protocol this
    /// build does not speak, for one. The socket closes after it.
    Refused(String),
    /// Something this agent does not act on.
    Nothing,
}

/// Reads one control frame.
#[must_use]
pub fn order(envelope: Envelope) -> Order {
    match envelope.msg {
        Some(v1::envelope::Msg::AgentWelcome(welcome)) => Order::Welcomed {
            agent_id: welcome.agent_id,
            heartbeat_secs: welcome.heartbeat_secs,
        },
        Some(v1::envelope::Msg::StartEngine(start)) => Order::Start {
            game_id: start.game_id,
            engine_token: start.engine_token,
            gateway_url: start.gateway_url,
        },
        Some(v1::envelope::Msg::StopEngine(stop)) => Order::Stop {
            game_id: stop.game_id,
        },
        Some(v1::envelope::Msg::Error(error)) => Order::Refused(error.message),
        _ => Order::Nothing,
    }
}

/// The arguments one engine process is started with.
///
/// Its token is not among them: an argument is in the process list for every
/// user of the machine, so it goes in [`ENGINE_TOKEN_VAR`], set on that one
/// child alone.
#[must_use]
pub fn engine_argv(gateway_url: &str, game_id: &str) -> Vec<String> {
    vec![
        "--attach".to_string(),
        gateway_url.to_string(),
        "--game".to_string(),
        game_id.to_string(),
    ]
}

/// Where an engine reads the token it proves itself with.
pub const ENGINE_TOKEN_VAR: &str = "BAYLEE_ENGINE_TOKEN";

/// What an engine must not inherit: this agent's own secret. An engine reads
/// players' frames, and needs nothing but its own game's token.
pub const AGENT_TOKEN_VAR: &str = "BAYLEE_AGENT_TOKEN";

/// Where the engine binary is when nobody said.
///
/// Beside this one: an agent and the engine it starts are built together and
/// shipped together, and a version skew between them is a protocol mismatch
/// nobody would enjoy debugging.
#[must_use]
pub fn default_engine_bin() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let candidate = exe.parent()?.join(engine_file_name());
    candidate.exists().then_some(candidate)
}

/// The engine binary's file name on this platform.
fn engine_file_name() -> &'static str {
    if cfg!(windows) {
        "baylee-engine-server.exe"
    } else {
        "baylee-engine-server"
    }
}

/// One engine's fate, as reported to the gateway.
#[must_use]
pub fn status(game_id: &str, kind: v1::engine_status::Kind, detail: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::EngineStatus(v1::EngineStatus {
            game_id: game_id.to_string(),
            kind: kind as i32,
            detail: detail.to_string(),
        })),
    }
}

/// A heartbeat, so the gateway can tell a live agent from a dead socket.
#[must_use]
pub fn heartbeat(millis: u64) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Heartbeat(v1::Heartbeat {
            client_time_ms: millis,
        })),
    }
}

#[cfg(test)]
mod tests;
