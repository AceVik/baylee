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

#![warn(missing_docs)]

use baylee_protocol::v1::{self, Envelope};
use std::path::PathBuf;

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
}

impl AgentConfig {
    /// Reads the configuration from the environment.
    ///
    /// # Errors
    /// When no token is configured, or when the engine binary cannot be found.
    pub fn from_env() -> Result<Self, String> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let token =
            var("BAYLEE_AGENT_TOKEN").ok_or_else(|| "BAYLEE_AGENT_TOKEN is not set".to_string())?;
        let engine_bin = match var("BAYLEE_ENGINE_BIN") {
            Some(path) => PathBuf::from(path),
            None => default_engine_bin().ok_or_else(|| {
                "cannot find baylee-engine-server; set BAYLEE_ENGINE_BIN".to_string()
            })?,
        };
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
        })
    }

    /// The control socket this agent connects to.
    #[must_use]
    pub fn control_url(&self) -> String {
        format!("{}/agent/ws", ws_base(&self.gateway))
    }

    /// The agent's opening frame.
    #[must_use]
    pub fn hello(&self) -> Envelope {
        Envelope {
            msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
                token: self.token.clone(),
                name: self.name.clone(),
                capacity: self.capacity,
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
    /// The gateway refused, or said something this agent does not act on.
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
        _ => Order::Nothing,
    }
}

/// The arguments one engine process is started with.
///
/// The token is an argument rather than an environment variable so that a
/// misconfigured agent cannot leak it into every child it ever starts; it is
/// still visible in the process list on this machine, which is the machine
/// that issued it.
#[must_use]
pub fn engine_argv(gateway_url: &str, game_id: &str, engine_token: &str) -> Vec<String> {
    vec![
        "--attach".to_string(),
        gateway_url.to_string(),
        "--game".to_string(),
        game_id.to_string(),
        "--token".to_string(),
        engine_token.to_string(),
    ]
}

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
mod tests {
    use super::*;

    fn config() -> AgentConfig {
        AgentConfig {
            gateway: "http://127.0.0.1:28766".to_string(),
            token: "secret".to_string(),
            name: "test".to_string(),
            capacity: 2,
            engine_bin: PathBuf::from("/bin/true"),
        }
    }

    #[test]
    fn the_control_socket_is_the_gateways_own_scheme() {
        assert_eq!(
            config().control_url(),
            "ws://127.0.0.1:28766/agent/ws",
            "http becomes ws"
        );
        let secure = AgentConfig {
            gateway: "https://play.example/".to_string(),
            ..config()
        };
        assert_eq!(
            secure.control_url(),
            "wss://play.example/agent/ws",
            "https becomes wss, and a trailing slash is not a second one"
        );
    }

    #[test]
    fn capacity_is_a_limit_and_zero_is_no_limit() {
        let two = config();
        assert!(two.has_room(0));
        assert!(two.has_room(1));
        assert!(!two.has_room(2), "two means two");
        let unlimited = AgentConfig {
            capacity: 0,
            ..config()
        };
        assert!(unlimited.has_room(1000));
    }

    #[test]
    fn the_gateways_orders_are_read_back_whole() {
        let start = Envelope {
            msg: Some(v1::envelope::Msg::StartEngine(v1::StartEngine {
                game_id: "g1".to_string(),
                engine_token: "tok".to_string(),
                gateway_url: "ws://gw/engine/ws".to_string(),
            })),
        };
        assert_eq!(
            order(start),
            Order::Start {
                game_id: "g1".to_string(),
                engine_token: "tok".to_string(),
                gateway_url: "ws://gw/engine/ws".to_string(),
            }
        );
        let stop = Envelope {
            msg: Some(v1::envelope::Msg::StopEngine(v1::StopEngine {
                game_id: "g1".to_string(),
            })),
        };
        assert_eq!(
            order(stop),
            Order::Stop {
                game_id: "g1".to_string()
            }
        );
        // A player-facing frame has no business here and is not acted on.
        let stray = Envelope {
            msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta::default())),
        };
        assert_eq!(order(stray), Order::Nothing);
    }

    #[test]
    fn the_engine_is_told_where_to_dial_and_what_to_play() {
        assert_eq!(
            engine_argv("ws://gw/engine/ws", "g1", "tok"),
            vec![
                "--attach".to_string(),
                "ws://gw/engine/ws".to_string(),
                "--game".to_string(),
                "g1".to_string(),
                "--token".to_string(),
                "tok".to_string(),
            ]
        );
    }

    #[test]
    fn the_hello_carries_what_the_gateway_checks() {
        let Some(v1::envelope::Msg::AgentHello(hello)) = config().hello().msg else {
            panic!("a hello");
        };
        assert_eq!(hello.token, "secret");
        assert_eq!(hello.capacity, 2);
    }
}
