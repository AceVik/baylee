//! `cards/enchantments/auras/mv_2/mistform_mask.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn mistform_mask() -> CardIndex {
    card_index("bbb2ac2e-de5c-441a-9707-5ced64df43a6")
}

/// Mistform Mask enchants a creature; its controller may pay `{1}` to turn the
/// enchanted creature into any chosen creature type (including Wall) until end
/// of turn, even if an opponent controls the creature.
#[test]
fn mistform_mask_changes_enchanted_creatures_type_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[mistform_mask()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent Elf seated");
    cast_from_hand(&mut engine, p0, mistform_mask());
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, stack_is_empty);

    let mask = on_battlefield(&engine, p0, mistform_mask()).expect("Mistform Mask attached");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mistform_mask(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("matched ChooseSubtype");
    };
    assert_eq!(player, p0);
    assert!(
        options.contains(&subtypes::creature::WALL),
        "Mistform Mask does not exclude Wall"
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::WALL))
        .expect("choose Wall for Mistform Mask");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::WALL));
    assert!(!subs.contains(subtypes::creature::ELF));

    // Only the Aura's controller (P0) owns the activated ability, not P1.
    reach_their_main_phase(&mut engine, p1);
    let subs_next = &engine
        .state()
        .object(elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        subs_next.contains(subtypes::creature::ELF)
            && !subs_next.contains(subtypes::creature::WALL),
        "type change ended at cleanup"
    );
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected P1 priority");
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == mask),
        "creature controller P1 cannot activate P0's Mistform Mask"
    );
}
