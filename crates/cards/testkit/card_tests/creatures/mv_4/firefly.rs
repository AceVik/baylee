//! `cards/creatures/mv_4/firefly.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Firefly` is a 1/1 Insect costing `{3}{R}` under `Coverage::Implemented` with flying.
/// It prints "{R}: This creature gets +1/+0 until end of turn."
/// When activated off floating red mana, its ability pump resolves and raises its power from 1 to 2
/// while keeping its toughness at 1.
#[test]
fn firefly_has_flying_and_pumps_its_power_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[firefly(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let insect = on_battlefield(&engine, p0, firefly()).expect("Firefly is on the battlefield");
    assert_eq!(pt(&engine, insect), (1, 1), "printed body is 1/1");
    assert!(
        keywords(&engine, insect).contains(KeywordSet::FLYING),
        "Firefly has flying"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Mountain produced one red mana"
    );

    activate(&mut engine, p0, firefly(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, insect),
        (2, 1),
        "Firefly power increased by 1 until end of turn"
    );
}
