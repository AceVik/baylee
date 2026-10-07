//! `cards/enchantments/auras/mv_1/black_ward.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Black Ward: "Enchanted creature has protection from black. This effect
/// doesn't remove this Aura." A black removal spell cannot target the
/// enchanted creature (CR 702.16b); an untouched second creature proves the
/// spell had a legal target at all, so the exclusion is the Ward and not an
/// empty menu.
#[test]
fn black_ward_protects_the_creature_it_enchants_from_black_spells() {
    let p0 = PlayerId::new(0);
    let ward = black_ward();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
                festering_goblin(),
            ],
        )
        .hand(0, &[ward, hero_s_downfall()])
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
    let attached = on_battlefield(&engine, p0, ward).expect("the Ward stays on the battlefield");
    assert_eq!(
        engine.state().object(attached).and_then(|o| o.attached_to),
        Some(creature)
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, hero_s_downfall());
    let offered = match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("Hero's Downfall asks for a target: {other:?}"),
    };
    assert!(
        offered.contains(&bystander),
        "the spell has a legal target: {offered:?}"
    );
    assert!(
        !offered.contains(&creature),
        "a black spell cannot target the warded creature: {offered:?}"
    );
}
