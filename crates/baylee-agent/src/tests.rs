use super::*;

fn line(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
}

#[test]
fn it_runs_on_nothing_and_helps_on_help() {
    assert_eq!(asked(&[]), Ok(Asked::Run));
    assert_eq!(asked(&line(&["--help"])), Ok(Asked::Help));
    assert_eq!(asked(&line(&["-h"])), Ok(Asked::Help));
}

/// Every other argument stops it, and the refusal repeats no value.
#[test]
fn any_other_argument_is_refused_without_its_value() {
    for (args, said) in [
        (&["--version"][..], "\"--version\""),
        (&["--gateway", "http://10.0.0.2:28766"][..], "\"--gateway\""),
        (&["--token=s3cret"][..], "\"--token\""),
        (&["run"][..], "takes no argument"),
        (&["s3cret"][..], "takes no argument"),
        (&["--bogus", "--help"][..], "\"--bogus\""),
    ] {
        let refused = asked(&line(args)).expect_err(said);
        assert!(refused.contains(said), "{args:?}: {refused}");
        assert!(!refused.contains("s3cret"), "{args:?}: {refused}");
    }
}

fn config() -> AgentConfig {
    AgentConfig {
        gateway: "http://127.0.0.1:28766".to_string(),
        token: "secret".to_string(),
        name: "test".to_string(),
        capacity: 2,
        engine_bin: PathBuf::from("/bin/true"),
        foresight_slots: None,
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

/// An agent on the gateway's unix socket hands its engines the socket;
/// one over TCP passes on the address the gateway ordered.
#[test]
fn a_unix_agent_sends_its_engines_down_the_same_socket() {
    let local = AgentConfig {
        gateway: "unix:/run/baylee/gateway.sock".to_string(),
        ..config()
    };
    assert_eq!(
        local.engine_address("ws://127.0.0.1:28766/engine/ws"),
        "unix:/run/baylee/gateway.sock"
    );
    assert_eq!(local.control_url(), "ws://localhost/agent/ws");
    assert_eq!(
        config().engine_address("ws://127.0.0.1:28766/engine/ws"),
        "ws://127.0.0.1:28766/engine/ws",
        "over TCP the gateway's own word stands"
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
        engine_argv("ws://gw/engine/ws", "g1"),
        vec![
            "--attach".to_string(),
            "ws://gw/engine/ws".to_string(),
            "--game".to_string(),
            "g1".to_string(),
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

/// A websocket URL is passed through untouched, which is the claim
/// `ws_base` makes about itself: guessing at one would be worse than
/// letting the socket refuse it. A base with no scheme at all is in the
/// same position — this function's job is the two it knows.
#[test]
fn a_base_that_is_already_a_socket_is_left_alone() {
    for already in ["ws://gw:28766", "wss://play.example"] {
        assert_eq!(ws_base(already), already);
        assert_eq!(
            ws_base(&format!("{already}/")),
            already,
            "and a trailing slash is never a second one"
        );
    }
    assert_eq!(
        ws_base("127.0.0.1:28766"),
        "127.0.0.1:28766",
        "a base with no scheme is handed on as it was written"
    );
    assert_eq!(
        ws_base("http://host/play"),
        "ws://host/play",
        "a gateway behind a path keeps its path"
    );
    let behind_a_path = AgentConfig {
        gateway: "https://example.test/baylee".to_string(),
        ..config()
    };
    assert_eq!(
        behind_a_path.control_url(),
        "wss://example.test/baylee/agent/ws"
    );
}

/// The welcome is the frame the agent waits for, and it carries the two
/// things it then lives by: the id the gateway knows it as and how often
/// to prove it is alive. Nothing else on that socket is acted on — an
/// envelope this build cannot read is silence, not a guess, which is the
/// same answer the wire gives a message from a newer peer.
#[test]
fn the_welcome_is_read_and_everything_unknown_is_silence() {
    let welcome = Envelope {
        msg: Some(v1::envelope::Msg::AgentWelcome(v1::AgentWelcome {
            agent_id: "a7".to_string(),
            heartbeat_secs: 20,
        })),
    };
    assert_eq!(
        order(welcome),
        Order::Welcomed {
            agent_id: "a7".to_string(),
            heartbeat_secs: 20,
        }
    );
    assert_eq!(
        order(Envelope { msg: None }),
        Order::Nothing,
        "an empty envelope is not an order"
    );
    let refusal = Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: "no".to_string(),
        })),
    };
    assert_eq!(
        order(refusal),
        Order::Refused("no".to_string()),
        "a refusal is read, so the operator is told why (#271)"
    );
}

/// The status kind travels as an integer and is read back as an enum, so
/// the cast is the contract: a gateway that reads a different variant
/// than the agent sent would log one engine's fate as another's. Each of
/// the three is sent through the wire type and asked for by name.
#[test]
fn an_engine_status_survives_the_number_it_travels_as() {
    use v1::engine_status::Kind;
    for kind in [Kind::Started, Kind::Exited, Kind::Failed] {
        let Some(v1::envelope::Msg::EngineStatus(sent)) =
            status("g1", kind, "exit status: 0").msg
        else {
            panic!("a status");
        };
        assert_eq!(sent.game_id, "g1");
        assert_eq!(sent.detail, "exit status: 0");
        assert_eq!(
            Kind::try_from(sent.kind).expect("a kind the schema knows"),
            kind,
            "the number the gateway reads back is the one the agent meant"
        );
        assert_ne!(sent.kind, Kind::Unspecified as i32);
    }
}

/// A heartbeat carries the agent's own clock, which is what lets the
/// gateway tell a live agent from a socket that is merely still open.
#[test]
fn a_heartbeat_carries_the_time_it_was_sent_at() {
    let Some(v1::envelope::Msg::Heartbeat(beat)) = heartbeat(1_700_000_000_123).msg else {
        panic!("a heartbeat");
    };
    assert_eq!(beat.client_time_ms, 1_700_000_000_123);
}

/// Everything the environment is allowed to say, in one place, so that
/// each test below is about one variable and the engine binary is never
/// a file this machine has to actually have.
fn configured(vars: &[(&str, &str)]) -> Result<AgentConfig, String> {
    AgentConfig::read(
        |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        },
        || Some(PathBuf::from("/opt/baylee/baylee-engine-server")),
    )
}

/// **An agent with no token does not start**, which is the one refusal
/// in the whole reading.
///
/// The reason is the topology rather than the protocol: this is the only
/// program here that spawns anything, so an agent that would introduce
/// itself to whoever answered is a way to run processes on this machine
/// at somebody else's asking. Refusing at start rather than at the first
/// `StartEngine` is the difference between an operator seeing it and a
/// player seeing it.
#[test]
fn an_agent_with_no_token_is_a_way_to_run_programs_and_does_not_start() {
    for vars in [vec![], vec![("BAYLEE_AGENT_TOKEN", "")]] {
        assert_eq!(
            configured(&vars),
            Err("BAYLEE_AGENT_TOKEN is not set".to_string()),
            "an empty variable is not a secret, it is one nobody set"
        );
    }
    assert_eq!(
        configured(&[("BAYLEE_AGENT_TOKEN", "s3cret")]).map(|config| config.token),
        Ok("s3cret".to_string()),
        "and with one it starts"
    );
}

/// **Empty is unset, for every one of them.** `BAYLEE_AGENT_NAME=` is a
/// line a shell writes when a variable it was expanding was itself
/// empty, and a reader asking only whether the variable *exists* would
/// name this agent the empty string, dial no gateway at all and look for
/// an engine at a path with nothing in front of it.
#[test]
fn a_variable_that_is_empty_is_a_variable_nobody_set() {
    let empty = configured(&[
        ("BAYLEE_AGENT_TOKEN", "s3cret"),
        ("BAYLEE_GATEWAY", ""),
        ("BAYLEE_AGENT_NAME", ""),
        ("BAYLEE_AGENT_CAPACITY", ""),
        ("BAYLEE_ENGINE_BIN", ""),
    ])
    .expect("a token is the only thing this needs");

    assert_eq!(
        empty,
        configured(&[("BAYLEE_AGENT_TOKEN", "s3cret")]).expect("the same reading"),
        "four empty variables read as four variables nobody set"
    );
    assert_eq!(empty.gateway, "http://127.0.0.1:28766");
    assert_eq!(empty.name, "agent");
    assert_eq!(empty.capacity, 0);
    assert_eq!(
        empty.engine_bin,
        PathBuf::from("/opt/baylee/baylee-engine-server")
    );
}

/// The engines' search slots are the operator's directory where one is
/// named, else one the agent makes, under the service's runtime directory
/// first, then the user's, then the temporary one; a relative or empty
/// runtime directory is passed over.
#[test]
fn the_search_slots_are_the_operators_or_the_agents_own() {
    let named = configured(&[
        ("BAYLEE_AGENT_TOKEN", "s3cret"),
        ("BAYLEE_FORESIGHT_SLOTS", "/run/baylee/foresight"),
    ])
    .expect("a token");
    assert_eq!(
        named.foresight_slots,
        Some(PathBuf::from("/run/baylee/foresight"))
    );
    let own = configured(&[
        ("BAYLEE_AGENT_TOKEN", "s3cret"),
        ("BAYLEE_FORESIGHT_SLOTS", ""),
    ])
    .expect("a token");
    assert_eq!(own.foresight_slots, None, "empty is unset");
    let parent = |pairs: &[(&str, &str)]| {
        let env: std::collections::BTreeMap<String, String> = pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        slots_parent(|name| env.get(name).cloned())
    };
    assert_eq!(
        parent(&[
            ("RUNTIME_DIRECTORY", "/run/baylee-agent:/run/other"),
            ("XDG_RUNTIME_DIR", "/run/user/1000"),
        ]),
        PathBuf::from("/run/baylee-agent")
    );
    assert_eq!(
        parent(&[("XDG_RUNTIME_DIR", "/run/user/1000")]),
        PathBuf::from("/run/user/1000")
    );
    assert_eq!(parent(&[]), std::env::temp_dir());
    assert_eq!(
        parent(&[("RUNTIME_DIRECTORY", "relative"), ("XDG_RUNTIME_DIR", "")]),
        std::env::temp_dir()
    );
}

/// The gateway base is kept in the **one** spelling the socket is built
/// out of. A trailing slash is what a shell completes a URL with and
/// what a browser's address bar hands over, and it would otherwise reach
/// [`AgentConfig::control_url`] as `…//agent/ws`.
///
/// [`ws_base`] trims one as well, which is not the same rule twice:
/// it takes any base from anywhere, and this one is about what the field
/// holds — what gets logged at start, and what two configurations are
/// compared on.
#[test]
fn the_gateway_base_is_kept_without_the_slash_a_shell_completes_it_with() {
    for written in ["http://gw:28766", "http://gw:28766/", "http://gw:28766///"] {
        let config = configured(&[
            ("BAYLEE_AGENT_TOKEN", "s3cret"),
            ("BAYLEE_GATEWAY", written),
        ])
        .expect("a gateway and a token");
        assert_eq!(config.gateway, "http://gw:28766", "written as {written}");
        assert_eq!(config.control_url(), "ws://gw:28766/agent/ws");
    }
}

/// **A capacity that is not a number takes the limit off**, which is the
/// wrong direction for a typo to fall in: `BAYLEE_AGENT_CAPACITY=fore`
/// does not cap this agent at four, it uncaps it, and the gateway then
/// hands it every game it has.
///
/// Pinned as it is rather than as it should be. The neighbouring
/// decision in this workspace went the other way — a `dev-table` board
/// specification that does not resolve refuses to start the gateway
/// rather than failing at the moment somebody presses Start — and the
/// day this reading does the same, this is the test that changes: the
/// first assertion becomes an `Err` naming the variable.
#[test]
fn a_capacity_that_is_not_a_number_takes_the_limit_off() {
    let config = configured(&[
        ("BAYLEE_AGENT_TOKEN", "s3cret"),
        ("BAYLEE_AGENT_CAPACITY", "fore"),
    ])
    .expect("it starts, which is the whole of what this pins");
    assert_eq!(config.capacity, 0);
    assert!(
        config.has_room(1000),
        "and zero is not `no engines`, it is `no limit`"
    );
    assert_eq!(
        configured(&[
            ("BAYLEE_AGENT_TOKEN", "s3cret"),
            ("BAYLEE_AGENT_CAPACITY", "4"),
        ])
        .map(|config| config.capacity),
        Ok(4),
        "a number that parses is still read as one"
    );
}

/// The engine binary is the one named, or the one beside this agent.
///
/// That the fallback is **not consulted** when a path was given is worth
/// an assertion rather than a reading, because the fallback asks the
/// filesystem: a reader that called it first would make an agent with an
/// explicit engine depend on a file it is never going to run.
#[test]
fn the_engine_binary_is_the_one_named_or_the_one_beside_this_agent() {
    let named = AgentConfig::read(
        |name| match name {
            "BAYLEE_AGENT_TOKEN" => Some("s3cret".to_string()),
            "BAYLEE_ENGINE_BIN" => Some("/opt/other/engine".to_string()),
            _ => None,
        },
        || -> Option<PathBuf> { panic!("not looked for when a path was given") },
    )
    .expect("a named binary");
    assert_eq!(named.engine_bin, PathBuf::from("/opt/other/engine"));

    assert_eq!(
        configured(&[("BAYLEE_AGENT_TOKEN", "s3cret")]).map(|config| config.engine_bin),
        Ok(PathBuf::from("/opt/baylee/baylee-engine-server")),
        "and the one beside this agent when nobody said"
    );
    assert_eq!(
        AgentConfig::read(
            |name| (name == "BAYLEE_AGENT_TOKEN").then(|| "s3cret".to_string()),
            || None,
        ),
        Err("cannot find baylee-engine-server; set BAYLEE_ENGINE_BIN".to_string()),
        "an agent that cannot find an engine names the variable to set"
    );
}
