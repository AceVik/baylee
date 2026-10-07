//! `cards/creatures/mv_3/dwarven_warriors.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dwarven Warriors — `{T}`: Target creature with power 2 or less can't be
/// blocked this turn. A power-5 creature is not on the menu, and the
/// keyword wears off at the next cleanup.
#[test]
fn dwarven_warriors_makes_a_weak_creature_unblockable_this_turn_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[dwarven_warriors(), pearled_unicorn(), water_elemental()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let warriors = on_battlefield(&engine, p0, dwarven_warriors()).expect("seated");
    let weak = on_battlefield(&engine, p0, pearled_unicorn()).expect("power 2");
    let strong = on_battlefield(&engine, p0, water_elemental()).expect("power 5");

    activate(&mut engine, p0, dwarven_warriors(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&weak), "power 2 or less is offered");
    assert!(
        !options.contains(&strong),
        "power 5 is not \"power 2 or less\": {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![weak],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, weak).contains(KeywordSet::UNBLOCKABLE));
    assert!(is_tapped(&engine, warriors));

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert!(
        !keywords(&engine, weak).contains(KeywordSet::UNBLOCKABLE),
        "\"this turn\" ended at cleanup"
    );
}
