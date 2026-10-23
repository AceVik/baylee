//! `cards/creatures/mv_2/viashino_sandscout.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn viashino_sandscout() -> CardIndex {
    card_index("f8d5a65c-382b-4277-9c08-5dec9e395b02")
}

#[test]
fn viashino_sandscout_has_haste_and_returns_to_hand_at_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[viashino_sandscout()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, viashino_sandscout());
    pass_until(&mut engine, stack_is_empty);

    let scout = on_battlefield(&engine, p0, viashino_sandscout()).unwrap();
    crate::engine::card_tests::legends_kit::attack(&mut engine, &[scout]);

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, viashino_sandscout()).is_none());
    assert!(in_hand(&engine, p0, viashino_sandscout()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        18,
        "haste allowed the cast-turn attack"
    );
}
