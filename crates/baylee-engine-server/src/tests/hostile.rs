//! A guest's bytes through the engine's own door: `EngineRunner::handle`, as the
//! gateway hands them on, unread. `baylee-engine/tests/hostile_actions.rs` holds
//! the decoder itself to the same frames.

use super::*;

/// The seat socket's `MAX_SEAT_FRAME`.
const FRAME_LIMIT: usize = 64 * 1024;

/// `bytes` as seat `seat`'s action frame.
fn sent(runner: &mut EngineRunner, seat: u32, bytes: Vec<u8>) -> Vec<Envelope> {
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: "g1".to_string(),
            seat_token: String::new(),
            action: bytes,
        })),
    };
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &[],
    )
}

/// A payment inside a payment to the frame limit, as it would be written.
fn nested_payment() -> Vec<u8> {
    let empty = baylee_core::wire::encode(&PlayerAction::PayWindow { steps: Vec::new() });
    let tag = &empty[..empty.len() - 1];
    let mut frame = Vec::with_capacity(FRAME_LIMIT);
    while frame.len() + tag.len() < FRAME_LIMIT {
        frame.extend(tag);
        frame.push(1);
    }
    frame
}

fn a_table_at_its_mulligan() -> EngineRunner {
    let mut runner = EngineRunner::new();
    setup(&mut runner, &duel(600));
    sit(&mut runner, 0);
    runner
}

/// What no frame may do: move the game, or stop the seat from playing on.
fn stands(runner: &mut EngineRunner, before: u64) {
    assert_eq!(
        runner.session().expect("a game").decision_seq(),
        before,
        "a frame that did not read moved nothing"
    );
    act(runner, 0, &PlayerAction::MulliganKeep);
    assert_ne!(
        runner.session().expect("a game").decision_seq(),
        before,
        "and the seat plays on"
    );
}

#[test]
fn a_payment_nested_to_the_frame_limit_reaches_the_engine_and_moves_nothing() {
    let mut runner = a_table_at_its_mulligan();
    let before = runner.session().expect("a game").decision_seq();
    let out = sent(&mut runner, 0, nested_payment());
    assert!(out.is_empty(), "nothing to say to it: {out:?}");
    stands(&mut runner, before);
}

#[test]
fn a_list_that_claims_everything_reaches_the_engine_and_moves_nothing() {
    let mut runner = a_table_at_its_mulligan();
    let before = runner.session().expect("a game").decision_seq();
    let empty = baylee_core::wire::encode(&PlayerAction::ChooseObjects {
        objects: Vec::new(),
    });
    let mut frame = empty[..empty.len() - 1].to_vec();
    frame.extend([0xff; 9]);
    frame.push(0x01);
    frame.resize(FRAME_LIMIT, 0x01);
    let out = sent(&mut runner, 0, frame);
    assert!(out.is_empty(), "nothing to say to it: {out:?}");
    stands(&mut runner, before);
}

#[test]
fn bytes_that_are_no_action_reach_the_engine_and_move_nothing() {
    let mut runner = a_table_at_its_mulligan();
    let before = runner.session().expect("a game").decision_seq();
    for frame in [
        Vec::new(),
        vec![0xff; 64],
        b"{\"MulliganKeep\":null}".to_vec(),
        vec![0xff; FRAME_LIMIT],
    ] {
        let out = sent(&mut runner, 0, frame);
        assert!(out.is_empty(), "nothing to say to it: {out:?}");
    }
    stands(&mut runner, before);
}

/// `bytes` as seat `seat`'s setting frame.
fn set(runner: &mut EngineRunner, seat: u32, bytes: Vec<u8>) -> Vec<Envelope> {
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::SeatSetting(v1::SeatSettingMsg {
            setting: bytes,
        })),
    };
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &[],
    )
}

#[test]
fn bytes_that_are_no_setting_reach_the_engine_and_move_nothing() {
    let mut runner = a_table_at_its_mulligan();
    let before = runner.session().expect("a game").decision_seq();
    let mut claims = vec![0x00];
    claims.extend([0xff, 0xff, 0xff, 0xff, 0x0f]);
    for frame in [
        Vec::new(),
        claims,
        vec![0xff; 64],
        b"{\"RevealHand\":true}".to_vec(),
        vec![0xff; FRAME_LIMIT],
    ] {
        let out = set(&mut runner, 0, frame);
        assert!(out.is_empty(), "nothing to say to it: {out:?}");
    }
    stands(&mut runner, before);
}

/// A setting that reads, naming seats no table has: what it asks is refused.
#[test]
fn a_setting_naming_seats_past_the_table_moves_nothing() {
    use baylee_core::ids::{PlayerId, SeatSet};
    use baylee_view::SeatSetting;

    let mut runner = a_table_at_its_mulligan();
    let before = runner.session().expect("a game").decision_seq();
    let settings = [
        SeatSetting::ShareHand((0..=SeatSet::MAX_SEAT).map(PlayerId::new).collect()),
        SeatSetting::RequestHand(PlayerId::new(u8::MAX)),
        SeatSetting::DeclineHand(PlayerId::new(u8::MAX)),
        SeatSetting::ShareHandOnce((0..=SeatSet::MAX_SEAT).map(PlayerId::new).collect()),
    ];
    for setting in settings {
        set(&mut runner, 0, baylee_core::wire::encode(&setting));
    }
    // Every bit of the set's word, past what `SeatSet` itself can be built to.
    set(&mut runner, 0, vec![0, 0xff, 0xff, 0x03]);
    stands(&mut runner, before);
}
