//! `cards/enchantments/auras/mv_2/phantasmal_terrain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phantasmal Terrain: "Enchant land" restricts the target menu to lands —
/// a Llanowar Elves beside the target is never among the options. "As this
/// Aura enters, choose a basic land type." That choice is asked of the
/// Aura's own caster, not of the enchanted land's controller — targeting
/// the *other* player's Forest is what tells the two apart — and the menu
/// offers exactly the five basic land types and refuses a nonbasic one.
#[test]
fn phantasmal_terrain_offers_only_the_five_basic_land_types() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[phantasmal_terrain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, forest()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, phantasmal_terrain());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Enchant land asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target),
        "the opponent's land is a legal \"Enchant land\" target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "\"Enchant land\": a creature is never offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });

    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(
        player, p0,
        "the Aura's own controller chooses, not the enchanted land's controller (p1)"
    );
    let mut offered = options.clone();
    offered.sort();
    let mut basics = vec![
        baylee_core::generated::subtypes::land::PLAINS,
        baylee_core::generated::subtypes::land::ISLAND,
        baylee_core::generated::subtypes::land::SWAMP,
        baylee_core::generated::subtypes::land::MOUNTAIN,
        baylee_core::generated::subtypes::land::FOREST,
    ];
    basics.sort();
    assert_eq!(
        offered, basics,
        "the five basic land types and nothing else, as a set"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::land::DESERT)
            )
            .is_err(),
        "a nonbasic land type is refused"
    );
}

/// Phantasmal Terrain: "Enchant land" restricts the target menu to lands —
/// a Festering Goblin on the board is never among the options. "Enchanted
/// land is the chosen type." Choosing Swamp turns the enchanted Forest
/// into one, tapping for {B}; a second Forest beside it, never chosen,
/// still taps for {G}.
#[test]
fn phantasmal_terrain_makes_the_enchanted_land_the_chosen_type() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), forest(), forest(), festering_goblin()],
        )
        .hand(0, &[phantasmal_terrain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    let (target, bystander) = (forests[0], forests[1]);
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("seated");

    tap_all_mana_but(&mut engine, p0, Some(forest()));
    cast_with_floating(&mut engine, p0, phantasmal_terrain());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Enchant land asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&target));
    assert!(
        !options.contains(&goblin),
        "\"Enchant land\": a creature is never offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::land::SWAMP),
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, phantasmal_terrain()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(target)
    );

    let mut swamp_only = baylee_core::types::SubtypeSet::EMPTY;
    swamp_only.insert(baylee_core::generated::subtypes::land::SWAMP);
    assert_eq!(
        engine
            .state()
            .object(target)
            .unwrap()
            .characteristics()
            .subtypes,
        swamp_only,
        "a Swamp, and no longer a Forest"
    );
    assert!(
        engine
            .state()
            .object(bystander)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::FOREST),
        "the untouched Forest is still one"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: target })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: bystander })
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the enchanted land taps for {{B}}"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the untouched Forest still taps for {{G}}"
    );
}
