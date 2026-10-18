//! `cards/creatures/mv_6/stangg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn stangg_exiles_twin_when_stangg_leaves_and_sacrifices_stangg_when_twin_leaves() {
    let stangg = baylee_core::generated::index::STANGG;
    let murder = baylee_core::generated::index::MURDER;
    let p0 = PlayerId::new(0);

    // Case 1: Stangg leaves the battlefield -> Stangg Twin is exiled.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[stangg, murder])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, stangg);
    pass_until(&mut engine, stack_is_empty);

    let stangg_id = on_battlefield(&engine, p0, stangg).expect("Stangg on battlefield");
    let twin_id = engine
        .state()
        .battlefield_seen()
        .find(|&id| engine.state().object(id).is_some_and(|o| o.card.is_none()))
        .expect("Stangg Twin token created");

    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 3);
    engine.refresh_offer();
    cast_with_floating(&mut engine, p0, murder);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![stangg_id],
            },
        )
        .expect("Murder targets Stangg");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine.state().battlefield_seen().any(|id| id == twin_id),
        "Stangg Twin is exiled when Stangg leaves the battlefield"
    );

    // Case 2: Stangg Twin leaves the battlefield -> Stangg is sacrificed.
    let mut engine2 = Duel::new(SEED + 1, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[stangg, murder])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    cast_from_hand(&mut engine2, p0, stangg);
    pass_until(&mut engine2, stack_is_empty);

    let twin_id2 = engine2
        .state()
        .battlefield_seen()
        .find(|&id| engine2.state().object(id).is_some_and(|o| o.card.is_none()))
        .expect("Stangg Twin token created");

    engine2.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 3);
    engine2.refresh_offer();
    cast_with_floating(&mut engine2, p0, murder);
    engine2
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![twin_id2],
            },
        )
        .expect("Murder targets Stangg Twin");
    pass_until(&mut engine2, stack_is_empty);

    assert!(
        on_battlefield(&engine2, p0, stangg).is_none(),
        "Stangg is sacrificed when Stangg Twin leaves the battlefield"
    );
    assert!(
        in_graveyard(&engine2, p0, stangg).is_some(),
        "sacrificed Stangg is in the graveyard"
    );
}
