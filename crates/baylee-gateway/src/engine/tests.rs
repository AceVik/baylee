use super::*;
use axum::extract::ws::Message;
use prost::Message as _;

/// **"Nothing to act on" and "the socket is done" are different answers,
/// and only one of them ends the loop.**
///
/// That is the sentence this enum exists for, and every frame kind a peer
/// can send has to land on the right side of it. A text frame reading as
/// `Closed` would hang up on a client sending a keepalive the wrong way;
/// a transport error reading as `Ignored` would spin the loop on a socket
/// that is already gone.
///
/// Undecodable bytes are the case worth naming. This plane is reached
/// with a per-game token, so a peer that has one may still send rubbish —
/// and it is dropped rather than hung up on, because the next frame from
/// the same engine is the game continuing.
///
/// A close frame ends the conversation immediately; no second frame or
/// completed TCP close handshake is needed to release its ownership.
#[test]
fn a_frame_that_carries_nothing_is_not_a_socket_that_is_gone() {
    let hello = Envelope {
        msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello::default())),
    };
    assert!(
        matches!(
            next_envelope(Some(Ok(Message::Binary(hello.encode_to_vec().into())))),
            Incoming::Msg(v1::envelope::Msg::EngineHello(_))
        ),
        "the one shape this plane acts on"
    );

    for (what, frame) in [
        (
            "an empty envelope",
            Message::Binary(Envelope { msg: None }.encode_to_vec().into()),
        ),
        (
            "bytes that are no envelope",
            Message::Binary(vec![0xff, 0xff, 0xff, 0xff].into()),
        ),
        ("a text frame", Message::Text("hello".into())),
        ("a ping", Message::Ping(Vec::new().into())),
        ("a pong", Message::Pong(Vec::new().into())),
    ] {
        assert!(
            matches!(next_envelope(Some(Ok(frame))), Incoming::Ignored),
            "{what} is nothing to act on and is not a reason to hang up"
        );
    }

    assert!(matches!(
        next_envelope(Some(Ok(Message::Close(None)))),
        Incoming::Closed
    ));

    assert!(
        matches!(next_envelope(None), Incoming::Closed),
        "the stream ending is the socket being gone"
    );
    assert!(
        matches!(
            next_envelope(Some(Err(axum::Error::new("the socket went away")))),
            Incoming::Closed
        ),
        "and so is a transport error"
    );
}

fn agent(games: usize, capacity: u32) -> Agent {
    Agent {
        name: "test".to_string(),
        local: false,
        capacity,
        engines: vec![execution::native(
            baylee_protocol::execution::Variant::StandardLoops,
        )],
        tx: mpsc::unbounded_channel().0,
        games: (0..games).map(|i| i.to_string()).collect(),
    }
}

#[test]
fn the_least_busy_agent_gets_the_next_game() {
    let mut agents = Agents::default();
    agents.connected.insert("b".to_string(), agent(0, 0));
    agents.connected.insert("a".to_string(), agent(3, 0));
    assert_eq!(agents.pick().as_deref(), Some("b"));
}

#[test]
fn a_full_agent_is_not_picked_and_no_agent_is_not_a_panic() {
    let mut agents = Agents::default();
    assert_eq!(agents.pick(), None, "nothing to pick from");
    agents.connected.insert("a".to_string(), agent(2, 2));
    assert_eq!(agents.pick(), None, "at capacity");
    agents.connected.insert("b".to_string(), agent(2, 0));
    assert_eq!(
        agents.pick().as_deref(),
        Some("b"),
        "capacity 0 means no limit, however many it is already running"
    );
}

/// Two agents with the same load must not be picked by hash order, or
/// "which agent runs this game" would differ between runs of the same
/// gateway with the same agents connected.
#[test]
fn ties_break_on_the_id() {
    let mut agents = Agents::default();
    for id in ["m", "a", "z"] {
        agents.connected.insert(id.to_string(), agent(1, 0));
    }
    assert_eq!(agents.pick().as_deref(), Some("a"));
}

/// A full agent still counts as available: its games end and it has room
/// again, and the lobby should not flicker with every game that starts.
#[test]
fn a_full_agent_is_still_an_available_one() {
    let mut agents = Agents::default();
    assert!(!agents.any_connected(), "nobody connected");
    agents.connected.insert("a".to_string(), agent(2, 2));
    assert!(agents.any_connected());
    assert_eq!(agents.pick(), None, "though it takes no game right now");
}

#[test]
fn retained_executors_are_exact_and_never_take_ordinary_games() {
    let mut agents = Agents::default();
    let mut retained = agent(0, 1);
    retained.engines[0].build = "retained-build".into();
    agents.connected.insert("old".into(), retained);
    assert_eq!(agents.pick(), None);
    assert_eq!(
        agents.pick_executor(Some("retained-build")).as_deref(),
        Some("old")
    );
    assert_eq!(agents.pick_executor(Some(baylee_build::short())), None);
    let mut legacy = agent(0, 0);
    legacy.engines.clear();
    agents.connected.insert("legacy".into(), legacy);
    assert_eq!(agents.pick(), None);
    assert_eq!(agents.pick_executor(Some(baylee_build::short())), None);
    let current = agent(1, 0);
    agents.connected.insert("current".into(), current);
    assert_eq!(
        agents.pick_executor(Some(baylee_build::short())).as_deref(),
        Some("current")
    );
    agents
        .connected
        .get_mut("old")
        .unwrap()
        .games
        .push("active".into());
    assert_eq!(agents.pick_executor(Some("retained-build")), None);
}

#[test]
fn variants_share_capacity_and_every_contract_field_is_pinned() {
    use baylee_protocol::execution::Variant;
    let mut agents = Agents::default();
    let mut all = agent(0, 1);
    all.engines = [Variant::Standard, Variant::StandardLoops, Variant::Tower]
        .into_iter()
        .map(execution::native)
        .collect();
    agents.connected.insert("all".into(), all);
    for variant in [Variant::Standard, Variant::StandardLoops, Variant::Tower] {
        assert_eq!(
            agents.pick_variant(variant, None),
            Some(("all".into(), execution::native(variant)))
        );
    }
    agents
        .connected
        .get_mut("all")
        .unwrap()
        .games
        .push("one".into());
    for variant in [Variant::Standard, Variant::StandardLoops, Variant::Tower] {
        assert!(agents.pick_variant(variant, None).is_none());
    }
    let agent = agents.connected.get_mut("all").unwrap();
    agent.games.clear();
    agent.engines = vec![execution::native(Variant::Standard)];
    for field in ["features", "rules", "cards", "ai", "build"] {
        let mut bad = execution::native(Variant::Standard);
        match field {
            "features" => bad.features = "b".repeat(64),
            "rules" => bad.rules.push('x'),
            "cards" => bad.cards.push('x'),
            "ai" => bad.ai.push('x'),
            _ => bad.build.push('x'),
        }
        agents.connected.get_mut("all").unwrap().engines[0] = bad;
        assert!(
            agents.pick_variant(Variant::Standard, None).is_none(),
            "{field}"
        );
    }
}

#[test]
fn attached_engines_must_match_the_complete_reservation() {
    let expected = execution::native(baylee_protocol::execution::Variant::Standard);
    let mut hello = v1::EngineHello {
        build: expected.build.clone(),
        execution: expected.to_metadata(),
        ..Default::default()
    };
    assert!(execution::matches(Some(&expected), &hello));
    assert!(!execution::matches(None, &hello));
    for field in ["variant", "build", "features", "rules", "cards", "ai"] {
        let mut other = expected.clone();
        match field {
            "variant" => other.variant = baylee_protocol::execution::Variant::Tower,
            "build" => other.build.push('x'),
            "features" => other.features = "b".repeat(64),
            "rules" => other.rules.push('x'),
            "cards" => other.cards.push('x'),
            _ => other.ai.push('x'),
        }
        hello.execution = other.to_metadata();
        assert!(!execution::matches(Some(&expected), &hello), "{field}");
    }
    hello.execution = expected.to_metadata();
    hello.build.push('x');
    assert!(!execution::matches(Some(&expected), &hello));
}
