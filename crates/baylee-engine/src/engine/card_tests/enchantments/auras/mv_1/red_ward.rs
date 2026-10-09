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

/// Red Ward: "This effect doesn't remove this Aura." A Chaoslace turns the Ward
/// red while it guards against red, and it stays on; a second Aura on the
/// same creature that a Chaoslace turns red falls off.
#[test]
fn red_ward_stays_on_when_it_is_itself_red() {
    a_ward_outlasts_its_own_color(red_ward(), chaoslace(), mountain(), Color::Red);
}

/// Red Ward: protection from red includes "can't be enchanted by red
/// Auras": Firebreathing is not offered the warded creature, and White's Holy
/// Armor still is.
#[test]
fn red_ward_keeps_red_auras_off_the_creature() {
    a_ward_keeps_auras_of_its_color_off(red_ward(), firebreathing(), mountain());
}

/// Red Ward: protection from red includes damage from red sources, read off
/// a block with and without the Ward.
#[test]
fn red_ward_prevents_damage_from_a_red_source() {
    a_ward_prevents_damage_from_its_color(red_ward(), gray_ogre(), false);
    a_ward_prevents_damage_from_its_color(red_ward(), gray_ogre(), true);
}

/// Red Ward: protection from red includes "can't be blocked by red
/// creatures", read with and without the Ward.
#[test]
fn red_ward_cannot_be_blocked_by_a_red_creature() {
    a_warded_creature_slips_past_its_color(red_ward(), gray_ogre(), false);
    a_warded_creature_slips_past_its_color(red_ward(), gray_ogre(), true);
}
