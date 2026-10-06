//! `cards/sorceries/mv_1/hurricane.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hurricane: "deals X damage to each creature with flying and each
/// player." A flying 1/3 dies to X = 3; a nonflying 1/1 beside it is
/// untouched.
#[test]
fn hurricane_burns_each_player_and_each_flying_creature_but_spares_grounded_ones() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), quiet_creature()],
        )
        .hand(0, &[hurricane()])
        .battlefield(1, &[oboro_envoy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before0 = life_of(&engine, p0);
    let before1 = life_of(&engine, p1);
    cast_from_hand(&mut engine, p0, hurricane());
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, p0), before0 - 3, "\"each player\"");
    assert_eq!(life_of(&engine, p1), before1 - 3);
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "no flying: untouched"
    );
    assert!(
        on_battlefield(&engine, p1, oboro_envoy()).is_none(),
        "flying, 3 toughness: dies to 3"
    );
}
