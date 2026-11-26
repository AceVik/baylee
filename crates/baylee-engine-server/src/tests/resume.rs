//! A game picked up again after its server restarted (`GameSetup.resume_record`,
//! `docs/proposals/games-survive-restarts.md`).

use super::record::{flush, pieces, play_a_little, unpacked};
use super::*;

/// The pieces as the gateway stores them: one gzip stream.
fn packed(pieces: &[v1::GameRecordChunk]) -> Vec<u8> {
    pieces.iter().flat_map(|p| p.data.clone()).collect()
}

/// The setup a gateway sends an engine of `build` for a game it resumes.
fn resume_on(
    runner: &mut EngineRunner,
    preset: &GamePreset,
    build: &str,
    record: Vec<u8>,
    pieces: u32,
) -> Vec<Envelope> {
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                execution: baylee_protocol::execution::Identity::native(
                    baylee_protocol::execution::Variant::StandardLoops,
                    build,
                    baylee_build::COMMIT,
                    u32::from(baylee_core::preset::AIProfile::GENERATION),
                )
                .to_metadata(),
                game_id: "g1".to_string(),
                preset_json: serde_json::to_vec(preset).expect("preset serializes"),
                seat_names: vec!["You".to_string(), "House".to_string()],
                resume_record: record,
                resume_pieces: pieces,
                ..Default::default()
            })),
        },
        &[],
    )
}

/// The game a few answers in, its record stored to the moment: the runner
/// and the pieces the gateway would hold.
fn a_game_with_its_record() -> (EngineRunner, GamePreset, Vec<v1::GameRecordChunk>) {
    let preset = duel(0);
    assert_ne!(
        preset.house_rules.loop_policy,
        baylee_core::preset::LoopPolicy::Off,
        "the duel plays under the loop detector, as `resume_on` names it"
    );
    let mut live = EngineRunner::new();
    let mut out = setup(&mut live, &preset);
    out.extend(sit(&mut live, 0));
    out.extend(play_a_little(&mut live, 12));
    out.extend(flush(&mut live, 1));
    let stored = pieces(&out);
    assert!(!stored.is_empty(), "the record was sent");
    (live, preset, stored)
}

fn ended_with(out: &[Envelope]) -> Option<String> {
    out.iter().find_map(|e| match &e.msg {
        Some(v1::envelope::Msg::GameEnded(ended)) => Some(ended.reason.clone()),
        _ => None,
    })
}

/// An engine started after its server restarted plays the stored record
/// again and is the game it was: the same state, the table open at once
/// for the seat that dials back (no curtain), the same answers taken from
/// there, and the record going on at the next piece, so the old pieces and
/// the new are one record that replays.
#[test]
fn a_restarted_engine_picks_the_game_up_where_its_record_ends() {
    let me = PlayerId::new(0);
    let (mut live, preset, stored) = a_game_with_its_record();
    let next = u32::try_from(stored.len()).unwrap();
    let mut resumed = EngineRunner::new();
    let said = resume_on(
        &mut resumed,
        &preset,
        &baylee_build::short(),
        packed(&stored),
        next,
    );
    assert!(said.is_empty(), "nothing is said before a seat is back");
    assert_eq!(
        resumed.session().unwrap().snapshot_hash(),
        live.session().unwrap().snapshot_hash(),
        "the same game"
    );
    assert!(
        resumed
            .clocks()
            .iter()
            .all(|clock| clock.what == crate::Deadline::Hold),
        "nobody is at the table: only its hold runs, no question's clock"
    );

    let back = attach(&mut resumed, 0);
    assert!(
        back.iter().any(|e| matches!(
            &e.msg,
            Some(v1::envelope::Msg::SeatFrame(frame))
                if matches!(
                    <Envelope as prost::Message>::decode(frame.envelope.clone())
                        .ok()
                        .and_then(|inner| inner.msg),
                    Some(v1::envelope::Msg::Curtain(_))
                )
        )),
        "the table is open at once"
    );

    let mut later = Vec::new();
    for _ in 0..8 {
        if live.finished() {
            break;
        }
        let action = live
            .session()
            .and_then(|s| s.house_action(me))
            .expect("the player is asked");
        act(&mut live, 0, &action);
        later.extend(act(&mut resumed, 0, &action));
        assert_eq!(
            resumed.session().unwrap().snapshot_hash(),
            live.session().unwrap().snapshot_hash(),
            "both go on alike"
        );
    }
    later.extend(flush(&mut resumed, 2));
    let later = pieces(&later);
    assert_eq!(
        later.first().map(|p| p.seq),
        Some(next),
        "the record goes on at the next piece"
    );
    let mut whole = unpacked(&stored);
    whole.extend(unpacked(&later));
    let replayed = baylee_gamehost::record::replay(&whole).expect("old and new are one record");
    assert_eq!(
        replayed.engine.snapshot_hash(),
        resumed.session().unwrap().snapshot_hash()
    );
}

/// A record is resumed only by the artifact that played it: another build
/// could make the same inputs a different game. The engine says so and ends
/// the game, as a game whose engine is gone ends today.
#[test]
fn a_record_another_build_played_is_not_resumed() {
    let (_, preset, stored) = a_game_with_its_record();
    let next = u32::try_from(stored.len()).unwrap();
    let mut other = EngineRunner::new();
    let said = resume_on(&mut other, &preset, "0.0.0+other", packed(&stored), next);
    assert_eq!(
        ended_with(&said).as_deref(),
        Some("the stored record does not resume on this engine")
    );
    assert!(other.session().is_none());
}

/// What is not a stored record of a game that goes on is refused, never
/// half-built: bytes that do not unpack, and the record of a game that is
/// over.
#[test]
fn a_record_that_cannot_go_on_is_refused() {
    let (_, preset, _) = a_game_with_its_record();
    let mut runner = EngineRunner::new();
    let said = resume_on(
        &mut runner,
        &preset,
        &baylee_build::short(),
        b"not gzip".to_vec(),
        1,
    );
    assert_eq!(
        ended_with(&said).as_deref(),
        Some("the stored record does not unpack")
    );

    let mut live = EngineRunner::new();
    let mut out = setup(&mut live, &preset);
    out.extend(sit(&mut live, 0));
    out.extend(act(&mut live, 0, &PlayerAction::Concede));
    assert!(live.finished(), "the game is over");
    let over = pieces(&out);
    let mut runner = EngineRunner::new();
    let said = resume_on(
        &mut runner,
        &preset,
        &baylee_build::short(),
        packed(&over),
        u32::try_from(over.len()).unwrap(),
    );
    assert_eq!(
        ended_with(&said).as_deref(),
        Some("the stored record is of a game that is over")
    );
}
