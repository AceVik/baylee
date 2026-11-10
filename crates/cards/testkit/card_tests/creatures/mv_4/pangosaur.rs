//! `cards/creatures/mv_4/pangosaur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "0b350f23-9150-40c7-8ac7-86adfdb8ba52"
fn pangosaur() -> CardIndex {
    card_index("0b350f23-9150-40c7-8ac7-86adfdb8ba52")
}

/// `Pangosaur` (`Coverage::Generated`):
/// "Whenever a player plays a land, return this creature to its owner's hand."
///
/// Verifies that `Pangosaur` returns to its owner's hand whenever either its
/// controller or an opponent plays a land.
#[test]
fn pangosaur_returns_to_owners_hand_when_any_player_plays_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9410, forest())
        .battlefield(0, &[pangosaur(), forest(), forest(), forest(), forest()])
        .hand(0, &[forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    // P0 plays a land -> Pangosaur returns to P0's hand.
    let land0 = in_hand(&engine, p0, forest()).expect("P0 Forest in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land0 })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, pangosaur()).is_none());
    assert!(in_hand(&engine, p0, pangosaur()).is_some());

    // Recast Pangosaur, then on P1's turn P1 plays a land -> Pangosaur returns again.
    cast_from_hand(&mut engine, p0, pangosaur());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, pangosaur()).is_some());

    assert!(walk_to_own_main(&mut engine, p1));
    let land1 = in_hand(&engine, p1, forest()).expect("P1 Forest in hand");
    engine
        .apply(p1, PlayerAction::PlayLand { card: land1 })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, pangosaur()).is_none());
    assert!(in_hand(&engine, p0, pangosaur()).is_some());
}
