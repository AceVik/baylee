//! `cards/enchantments/auras/mv_1/blue_ward.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blue Ward: "Enchanted creature has protection from blue." Unsummon,
/// a blue spell, cannot target the warded creature.
#[test]
fn blue_ward_protects_the_creature_it_enchants_from_blue_spells() {
    let p0 = PlayerId::new(0);
    let ward = blue_ward();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), island(), quiet_creature(), festering_goblin()],
        )
        .hand(0, &[ward, unsummon()])
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
    cast_with_floating(&mut engine, p0, unsummon());
    let offered = match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("Unsummon asks for a target: {other:?}"),
    };
    assert!(offered.contains(&bystander), "{offered:?}");
    assert!(
        !offered.contains(&creature),
        "a blue spell cannot target the warded creature: {offered:?}"
    );
}

/// Blue Ward: "This effect doesn't remove this Aura." A Thoughtlace turns the Ward
/// blue while it guards against blue, and it stays on; a second Aura on the
/// same creature that a Thoughtlace turns blue falls off.
#[test]
fn blue_ward_stays_on_when_it_is_itself_blue() {
    a_ward_outlasts_its_own_color(blue_ward(), thoughtlace(), island(), Color::Blue);
}

/// Blue Ward: protection from blue includes "can't be enchanted by blue
/// Auras": Flight is not offered the warded creature, and White's Holy
/// Armor still is.
#[test]
fn blue_ward_keeps_blue_auras_off_the_creature() {
    a_ward_keeps_auras_of_its_color_off(blue_ward(), flight(), island());
}

/// Blue Ward: protection from blue includes damage from blue sources, read off
/// a block with and without the Ward.
#[test]
fn blue_ward_prevents_damage_from_a_blue_source() {
    a_ward_prevents_damage_from_its_color(blue_ward(), merfolk_of_the_pearl_trident(), false);
    a_ward_prevents_damage_from_its_color(blue_ward(), merfolk_of_the_pearl_trident(), true);
}

/// Blue Ward: protection from blue includes "can't be blocked by blue
/// creatures", read with and without the Ward.
#[test]
fn blue_ward_cannot_be_blocked_by_a_blue_creature() {
    a_warded_creature_slips_past_its_color(blue_ward(), merfolk_of_the_pearl_trident(), false);
    a_warded_creature_slips_past_its_color(blue_ward(), merfolk_of_the_pearl_trident(), true);
}
