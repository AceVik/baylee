//! `cards/instants/mv_2/negate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Negate` is an instant costing `{1}{U}` under `Coverage::Implemented` that counters target noncreature spell.
/// When the opponent casts a noncreature spell (such as `Rejuvenate`) and passes priority, `Negate`
/// can be cast in response off two Islands. Targeting the spell on the stack counters it, placing both
/// cards into their respective owners' graveyards without the countered spell taking effect.
#[test]
fn negate_counters_target_noncreature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let rejuvenate_card = card_index("35069cdb-e5bd-4224-a1a8-b329054e003b");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[negate()])
        .battlefield(1, &[forest(), forest(), forest(), forest()])
        .hand(1, &[rejuvenate_card])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, rejuvenate_card);
    let spell_on_stack = on_stack(&engine, rejuvenate_card).expect("Rejuvenate is on the stack");

    // p1 passes priority; p0 responds with Negate.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority for p0, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "p0 receives priority while spell is on stack");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, negate());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Negate, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&spell_on_stack),
        "Rejuvenate on stack is offered as target noncreature spell: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell_on_stack],
            },
        )
        .expect("targeting Rejuvenate with Negate is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        20,
        "countered Rejuvenate never resolved; opponent gained no life"
    );
    assert!(
        in_graveyard(&engine, p1, rejuvenate_card).is_some(),
        "countered spell was put into owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, negate()).is_some(),
        "resolved Negate is in caster's graveyard"
    );
}
