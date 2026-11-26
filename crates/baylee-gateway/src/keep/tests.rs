//! A kept game's row: its shape, what is kept, and that reading it back
//! gives the same game.

use super::restore::header_of;
use super::row::{Row, VERSION, keepable};
use crate::lobby::{Delegate, Kept, LobbyGame, LobbySeat, LobbyState, SeatKind};
use baylee_core::card_metadata::RulesMetadata;
use baylee_core::preset::{AIProfile, FormatId, GamePreset, HouseRules};
use baylee_protocol::execution::{Identity, Variant};

fn preset() -> GamePreset {
    GamePreset {
        format: FormatId::Custom,
        seed: 7,
        house_rules: HouseRules::default(),
        seats: Vec::new(),
        prints: Vec::new(),
        modifiers: Vec::new(),
        rules: RulesMetadata::default(),
    }
}

fn identity(variant: Variant) -> Identity {
    Identity::native(
        variant,
        baylee_build::short(),
        baylee_build::COMMIT,
        u32::from(AIProfile::GENERATION),
    )
}

/// A running duel of a person and a seat bridge its host seated, with what
/// a row holds set to something other than its default.
fn running() -> LobbyGame {
    let mut me = LobbySeat::open(0);
    me.account_id = Some("0190a000-0000-7000-8000-000000000001".into());
    me.seat_token_hash = Some("aa".repeat(32));
    me.central_session = Some(vec![1, 2, 3]);
    me.deck_name = "Mine".into();
    me.team = Some(1);
    me.joined_seq = Some(0);
    let mut bridge = LobbySeat::open(1);
    bridge.delegate = Some(Delegate {
        by: "0190a000-0000-7000-8000-000000000001".into(),
        name: "LLM-Test".into(),
        ready: true,
        reported: false,
    });
    bridge.seat_token_hash = Some("bb".repeat(32));
    bridge.participation_ended = true;
    bridge.kind = SeatKind::Human;
    let mut game = LobbyGame::playing(
        "0190a000-0000-7000-8000-0000000000aa".into(),
        vec![me, bridge],
        preset(),
        100,
    );
    game.host = Some("0190a000-0000-7000-8000-000000000001".into());
    game.name = "Evening".into();
    game.format = Some("commander".into());
    game.allow_spectators = false;
    game.password_hash = Some("cc".repeat(32));
    game.next_seq = 2;
    game.execution = Some(identity(Variant::StandardLoops));
    game
}

/// The shape a row is written in. Whoever changes this text changes what
/// the next build reads back, so it moves only with [`VERSION`].
#[test]
fn the_row_keeps_its_shape() {
    assert_eq!(VERSION, 1);
    let row = Row::of(&running()).expect("a running casual game is kept");
    let text = serde_json::to_string(&row).unwrap();
    assert_eq!(
        text,
        concat!(
            r#"{"host":"0190a000-0000-7000-8000-000000000001","name":"Evening","#,
            r#""format":"commander","list":null,"setup":{"starting_life":40,"free_mulligans":1,"seats":[]},"#,
            r#""allow_spectators":false,"#,
            r#""password_hash":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","#,
            r#""created_at":100,"started_at":100,"next_seq":2,"parent":null,"seats":["#,
            r#"{"kind":"human","ai":null,"account":"0190a000-0000-7000-8000-000000000001","#,
            r#""token_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","#,
            r#""session":"AQID","participation_ended":false,"deck_name":"Mine","deck":null,"#,
            r#""team":1,"joined_seq":0,"delegate":null,"hosted":null},"#,
            r#"{"kind":"human","ai":null,"account":null,"#,
            r#""token_hash":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","#,
            r#""session":null,"participation_ended":true,"deck_name":"","deck":null,"#,
            r#""team":null,"joined_seq":null,"#,
            r#""delegate":{"by":"0190a000-0000-7000-8000-000000000001","name":"LLM-Test","ready":true,"reported":false},"#,
            r#""hosted":null}]}"#,
        )
    );
}

/// Read back, a row is the game it was written from: the same row again,
/// running, waiting for its engine on the artifact its record names.
#[test]
fn a_row_read_back_is_the_game_it_was() {
    let game = running();
    let row = Row::of(&game).unwrap();
    let text = serde_json::to_string(&row).unwrap();
    let read: Row = serde_json::from_str(&text).unwrap();
    let again = read.game(game.id.clone(), preset(), identity(Variant::StandardLoops));
    assert_eq!(Row::of(&again), Some(row));
    assert_eq!(again.state, LobbyState::Playing);
    assert!(again.restored(), "it waits to be resumed");
    assert!(again.engine.is_none() && again.agent_id.is_none());
    assert_eq!(
        again.seats[0].central_session.as_deref(),
        Some(&[1, 2, 3][..])
    );
    assert_eq!(again.preset.as_ref().map(|p| p.seed), Some(7));
}

/// Only a running casual game is kept: not a waiting room, not a game that
/// is over, not a ranked duel, a Tower run or a challenge, and not a game
/// whose record stopped being stored.
#[test]
fn only_a_running_casual_game_is_kept() {
    assert!(keepable(&running()));
    let mut waiting = running();
    waiting.state = LobbyState::Waiting;
    let mut over = running();
    over.finish(200);
    let mut tower = running();
    tower.execution = Some(identity(Variant::Tower));
    let mut challenge = running();
    challenge.challenge = Some([1; 16]);
    let mut cut = running();
    cut.kept = Kept::Cut;
    let mut unordered = running();
    unordered.execution = None;
    for (what, game) in [
        ("a waiting room", waiting),
        ("a game that is over", over),
        ("a Tower run", tower),
        ("a challenge", challenge),
        ("a cut record", cut),
        ("a game with no engine ordered", unordered),
    ] {
        assert!(!keepable(&game), "{what}");
        assert_eq!(Row::of(&game), None, "{what}");
    }
}

/// A row of a later shape is not read as this one.
#[test]
fn a_row_with_a_field_this_build_does_not_know_is_not_read() {
    let mut text = serde_json::to_string(&Row::of(&running()).unwrap()).unwrap();
    text.insert_str(1, r#""later":1,"#);
    assert!(serde_json::from_str::<Row>(&text).is_err());
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(bytes).unwrap();
    gz.finish().unwrap()
}

/// The header a resumed game takes its preset and its engine from is the
/// record's first line, read from its first piece alone.
#[test]
fn the_header_is_read_from_the_first_piece() {
    let execution = identity(Variant::StandardLoops);
    let line = serde_json::json!({
        "kind": "header",
        "record": 1,
        "build": baylee_build::short(),
        "preset": preset(),
        "hash": "00",
        "execution": execution,
    });
    let mut record = serde_json::to_vec(&line).unwrap();
    record.extend_from_slice(b"\n{\"kind\":\"input\"}\n");
    let header = header_of(&gzip(&record)).unwrap();
    assert_eq!(header.preset.seed, 7);
    assert_eq!(header.execution, Some(execution));

    let cut = serde_json::to_vec(&line).unwrap();
    assert!(
        header_of(&gzip(&cut)).is_err(),
        "a header with no line end is cut off"
    );
    assert!(header_of(b"not gzip").is_err());
    assert!(
        header_of(&gzip(b"{\"kind\":\"input\"}\n")).is_err(),
        "a record must begin with its header"
    );
}
