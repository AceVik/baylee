//! `cards/sorceries/mv_1/earthquake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Earthquake: "deals X damage to each creature without flying and each
/// player." A nonflying 1/1 dies; a flying 1/3 beside it is untouched.
#[test]
fn earthquake_burns_each_player_and_each_nonflying_creature_but_spares_flyers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), quiet_creature()])
        .hand(0, &[earthquake()])
        .battlefield(1, &[oboro_envoy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before0 = life_of(&engine, p0);
    let before1 = life_of(&engine, p1);
    cast_from_hand(&mut engine, p0, earthquake());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, p0), before0 - 2, "\"each player\"");
    assert_eq!(life_of(&engine, p1), before1 - 2);
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "1 toughness, no flying: dies to 2"
    );
    assert!(
        on_battlefield(&engine, p1, oboro_envoy()).is_some(),
        "flying: untouched"
    );
}
