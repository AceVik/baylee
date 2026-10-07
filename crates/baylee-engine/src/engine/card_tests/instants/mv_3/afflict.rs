//! `cards/instants/mv_3/afflict.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Afflict` is an instant costing `{2}{B}` under `Coverage::Implemented`.
/// It prints "Target creature gets -1/-1 until end of turn. Draw a card."
/// When cast from hand off three Swamps targeting an opponent's 2/2 creature (such as `Desert Drake`),
/// the target creature's power and toughness become 1/1 until end of turn and the caster draws a card.
#[test]
fn afflict_shrinks_target_creature_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let drake_card = card_index("ce8f4eb4-08b8-404b-9147-1e28c1b14a65");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[afflict()])
        .battlefield(1, &[drake_card])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let initial_hand_len = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let target_drake =
        on_battlefield(&engine, p1, drake_card).expect("opponent controls Desert Drake");
    assert_eq!(pt(&engine, target_drake), (2, 2), "initial body is 2/2");

    cast_from_hand(&mut engine, p0, afflict());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Afflict, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&target_drake),
        "target creature is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_drake],
            },
        )
        .expect("targeting Desert Drake is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target_drake),
        (1, 1),
        "target creature received -1/-1 until end of turn"
    );
    let final_hand_len = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        final_hand_len, initial_hand_len,
        "caster spent Afflict and drew 1 card, leaving hand size unchanged"
    );
    assert!(
        in_graveyard(&engine, p0, afflict()).is_some(),
        "resolved Afflict is in caster's graveyard"
    );
}
