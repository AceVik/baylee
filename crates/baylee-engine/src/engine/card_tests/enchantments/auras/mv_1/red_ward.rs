//! `cards/enchantments/auras/mv_1/red_ward.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Red Ward: "Enchanted creature has protection from red." Lightning Bolt,
/// a red spell, cannot target the warded creature.
#[test]
fn red_ward_protects_the_creature_it_enchants_from_red_spells() {
    let p0 = PlayerId::new(0);
    let ward = red_ward();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), mountain(), quiet_creature(), festering_goblin()],
        )
        .hand(0, &[ward, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("the warded creature");
    let bystander = on_battlefield(&engine, p0, festering_goblin()).expect("a second creature");

    let plains_id = on_battlefield(&engine, p0, plains()).expect("the Plains");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: plains_id })
        .unwrap();
    cast_with_floating(&mut engine, p0, ward);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    let offered = match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("Lightning Bolt asks for a target: {other:?}"),
    };
    assert!(offered.contains(&bystander), "{offered:?}");
    assert!(
        !offered.contains(&creature),
        "a red spell cannot target the warded creature: {offered:?}"
    );
}
