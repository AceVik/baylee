//! `cards/creatures/artifacts/mv_4/hedron_rover.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hedron_rover() -> CardIndex {
    card_index("1b2826ea-a3d5-4b8a-b362-0b0880d1cc48")
}

/// Hedron Rover — {4} 2/2: "Landfall — Whenever a land you control enters,
/// this creature gets +2/+2 until end of turn." Playing a Forest from hand
/// makes it 4/4, and it is a 2/2 again when the next turn starts. The
/// opponent's land does not count.
#[test]
fn hedron_rover_gets_plus_two_on_landfall() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hedron_rover()])
        .hand(0, &[forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rover = on_battlefield(&engine, p0, hedron_rover()).expect("seated");
    assert_eq!(pt(&engine, rover), (2, 2));

    let land = in_hand(&engine, p0, forest()).expect("a Forest in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .expect("the land drop");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, rover), (4, 4), "landfall");

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(pt(&engine, rover), (2, 2), "until end of turn");

    // The other seat's land drop, on its own turn, is not "a land you control".
    reach_their_main_phase(&mut engine, p1);
    let theirs = in_hand(&engine, p1, forest()).expect("their Forest");
    engine
        .apply(p1, PlayerAction::PlayLand { card: theirs })
        .expect("their land drop");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, rover), (2, 2), "not a land you control");
}
