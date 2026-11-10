//! Well of Life — {4} — Artifact
//! Oracle: At the beginning of your end step, if you control no untapped lands, you gain 2 life.

use super::*;

#[test]
fn well_of_life_gains_two_life_at_end_step_when_all_lands_are_tapped() {
    let well_of_life = card_index("4e05d1a2-ad53-4f2e-aaea-7ec464b07c04");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[well_of_life])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, well_of_life);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, well_of_life).is_some());
    assert!(
        all_on_battlefield(&engine, p0, plains())
            .into_iter()
            .all(|id| is_tapped(&engine, id))
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 22);
}

#[test]
fn well_of_life_does_not_trigger_when_an_untapped_land_remains() {
    let well_of_life = card_index("4e05d1a2-ad53-4f2e-aaea-7ec464b07c04");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[well_of_life])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let lands = all_on_battlefield(&engine, p0, plains());
    for &land in &lands[..4] {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .expect("tap Plains");
    }
    let card = in_hand(&engine, p0, well_of_life).expect("Well of Life in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("cast Well of Life");
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, lands[4]));

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 20);
}
