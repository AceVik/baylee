//! `cards/artifacts/mv_4/tower_of_calamities.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tower_of_calamities() -> CardIndex {
    card_index("27618701-e8c9-4117-94f3-3836cf34990d")
}

/// "{8}, {T}: This artifact deals 12 damage to target creature." Only
/// creatures are on the menu, and 12 kills a creature with toughness 5.
#[test]
fn twelve_damage_to_a_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[tower_of_calamities(), forest(), forest(), forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[earth_elemental(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tower = on_battlefield(&engine, p0, tower_of_calamities()).unwrap();
    let giant = on_battlefield(&engine, p1, earth_elemental()).unwrap();
    let rock = on_battlefield(&engine, p1, quiet_artifact()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tower_of_calamities(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a creature target, got {:?}", engine.pending())
    };
    assert!(options.contains(&giant));
    assert!(!options.contains(&rock), "not a creature: {options:?}");
    aim_at(&mut engine, p0, giant);
    assert!(is_tapped(&engine, tower));
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, earth_elemental()).is_some(),
        "12 damage is more than any toughness here"
    );
}

/// Seven mana are not eight.
#[test]
fn seven_mana_do_not_pay_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[tower_of_calamities(), forest(), forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tower = on_battlefield(&engine, p0, tower_of_calamities()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(!priority_offer(&engine).abilities.contains(&(tower, 0)));
}
