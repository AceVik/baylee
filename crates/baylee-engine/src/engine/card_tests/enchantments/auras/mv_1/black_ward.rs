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

/// Black Ward: "This effect doesn't remove this Aura." A Deathlace turns the Ward
/// black while it guards against black, and it stays on; a second Aura on the
/// same creature that a Deathlace turns black falls off.
#[test]
fn black_ward_stays_on_when_it_is_itself_black() {
    a_ward_outlasts_its_own_color(black_ward(), deathlace(), swamp(), Color::Black);
}

/// Black Ward: protection from black includes "can't be enchanted by black
/// Auras": Unholy Strength is not offered the warded creature, and White's Holy
/// Armor still is.
#[test]
fn black_ward_keeps_black_auras_off_the_creature() {
    a_ward_keeps_auras_of_its_color_off(black_ward(), unholy_strength(), swamp());
}

/// Black Ward: protection from black includes damage from black sources, read off
/// a block with and without the Ward.
#[test]
fn black_ward_prevents_damage_from_a_black_source() {
    a_ward_prevents_damage_from_its_color(black_ward(), bile_urchin(), false);
    a_ward_prevents_damage_from_its_color(black_ward(), bile_urchin(), true);
}

/// Black Ward: protection from black includes "can't be blocked by black
/// creatures", read with and without the Ward.
#[test]
fn black_ward_cannot_be_blocked_by_a_black_creature() {
    a_warded_creature_slips_past_its_color(black_ward(), bile_urchin(), false);
    a_warded_creature_slips_past_its_color(black_ward(), bile_urchin(), true);
}
