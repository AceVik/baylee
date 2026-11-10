//! `cards/sorceries/mv_1/stir_the_grave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("296dcc57-5651-4cb1-b89c-fcdd3f5311bd")
}

#[test]
fn stir_the_grave_returns_only_own_graveyard_creature_within_mana_value_x_to_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
                fangren_hunter(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let bear = on_battlefield(&game, p0, quiet_creature()).unwrap();
    let giant = on_battlefield(&game, p0, fangren_hunter()).unwrap();
    let artifact = on_battlefield(&game, p0, quiet_artifact()).unwrap();
    let their_bear = on_battlefield(&game, p1, quiet_creature()).unwrap();
    bury(&mut game, &[bear, giant, artifact, their_bear]);
    game.refresh_offer();

    cast_from_hand(&mut game, p0, subject());
    let Pending::ChooseNumber {
        player, min, max, ..
    } = game.pending().clone()
    else {
        panic!("expected X announcement, got {:?}", game.pending());
    };
    assert_eq!(player, p0);
    assert!(
        min <= 2 && 2 <= max,
        "X = 2 must be offered in {min}..={max}"
    );
    game.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();

    let options = aim_at(&mut game, p0, bear);
    assert_eq!(
        options,
        vec![bear],
        "with X = 2, only own mana-value-2 creature is targetable (not mana-value-4 Fangren Hunter, noncreature artifact, or opponent graveyard creature)"
    );
    pass_until(&mut game, stack_is_empty);

    assert_eq!(on_battlefield(&game, p0, quiet_creature()), Some(bear));
    assert!(in_graveyard(&game, p0, quiet_creature()).is_none());
    assert_eq!(in_graveyard(&game, p0, fangren_hunter()), Some(giant));
    assert_eq!(in_graveyard(&game, p0, quiet_artifact()), Some(artifact));
    assert_eq!(in_graveyard(&game, p1, quiet_creature()), Some(their_bear));
}
