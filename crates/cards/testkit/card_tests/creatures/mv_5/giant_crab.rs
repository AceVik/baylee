//! `cards/creatures/mv_5/giant_crab.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Giant Crab is a 3/3 Crab under `Coverage::Implemented` with an activated shroud ability.
/// Paying {U} gives Giant Crab shroud until end of turn.
/// Activating the ability spends the blue mana and grants `KeywordSet::SHROUD` through the layer system.
/// The body power and toughness remain unchanged at 3/3.
#[test]
fn giant_crab_gains_shroud_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[giant_crab(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let crab = on_battlefield(&engine, p0, giant_crab()).expect("Giant Crab is on battlefield");
    assert_eq!(pt(&engine, crab), (3, 3), "printed body is 3/3");
    assert!(
        !keywords(&engine, crab).contains(KeywordSet::SHROUD),
        "Giant Crab starts without shroud"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Island produces one blue mana"
    );

    activate(&mut engine, p0, giant_crab(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, crab).contains(KeywordSet::SHROUD),
        "Giant Crab gained shroud"
    );
    assert_eq!(pt(&engine, crab), (3, 3), "body remains 3/3");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the blue mana was spent"
    );
}
