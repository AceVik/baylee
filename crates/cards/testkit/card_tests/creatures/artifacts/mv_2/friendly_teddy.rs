//! `cards/creatures/artifacts/mv_2/friendly_teddy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn friendly_teddy() -> CardIndex {
    card_index("02a8b849-e708-427c-92fe-9a41bb0ec7fe")
}

/// Friendly Teddy: a 2/2; "when this creature dies, each player draws a
/// card". Both players, controller and opponent alike, draw exactly one.
#[test]
fn friendly_teddy_makes_each_player_draw_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = table(&[], &[friendly_teddy()], &[]);
    let teddy = on_battlefield(&engine, p0, friendly_teddy()).expect("seated");
    assert_eq!(pt(&engine, teddy), (2, 2));
    let (mine, theirs) = (hand_count(&engine, 0), hand_count(&engine, 1));
    let (lib0, lib1) = (library_size(&engine, p0), library_size(&engine, p1));
    kill(&mut engine, teddy);
    assert!(in_graveyard(&engine, p0, friendly_teddy()).is_some());
    assert_eq!(hand_count(&engine, 0), mine + 1);
    assert_eq!(hand_count(&engine, 1), theirs + 1);
    assert_eq!(library_size(&engine, p0), lib0 - 1);
    assert_eq!(library_size(&engine, p1), lib1 - 1);
}
