//! `cards/enchantments/auras/mv_1/evil_presence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Evil Presence: "Enchant land" restricts the target menu to lands — a
/// Festering Goblin on the board is never among the options. "Enchanted
/// land is a Swamp." The enchanted Forest loses its own land type, taps
/// for {B} instead of {G}, and keeps being a land and a basic land by
/// supertype. A second Forest beside it, never targeted, still taps for
/// {G} — the pool that tells the two apart.
#[test]
fn evil_presence_turns_a_forest_into_a_swamp_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest(), forest(), festering_goblin()])
        .hand(0, &[evil_presence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 2, "two Forests are seated");
    let (target, bystander) = (forests[0], forests[1]);
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("seated");

    tap_all_mana_but(&mut engine, p0, Some(forest()));
    cast_with_floating(&mut engine, p0, evil_presence());
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
    pass_until(&mut engine, stack_is_empty);

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
            .object(target)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::BASIC),
        "still a basic land by supertype"
    );
    assert!(
        engine
            .state()
            .object(target)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::LAND),
        "still a land"
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
        "only the untouched Forest still taps for {{G}}"
    );
}

/// Evil Presence: "Enchanted land is a Swamp." Cast on Darksteel Citadel —
/// an Artifact Land with a printed keyword (indestructible) and its own
/// printed mana ability — it keeps neither afterward: nothing is left to
/// activate but the new type's own mana, and it is destructible again. It
/// stays a land, and an artifact.
#[test]
fn evil_presence_strips_a_nonbasic_lands_printed_keyword_and_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), darksteel_citadel()])
        .hand(0, &[evil_presence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let citadel = on_battlefield(&engine, p0, darksteel_citadel()).expect("the Citadel is seated");
    assert!(
        keywords(&engine, citadel).contains(KeywordSet::INDESTRUCTIBLE),
        "indestructible, as printed, before anything enchants it"
    );
    let before = priority_offer(&engine);
    assert!(
        before.abilities.contains(&(citadel, 0)),
        "its printed {{T}}: Add {{C}} is offered"
    );
    assert!(
        !before.mana_abilities.contains(&citadel),
        "no basic land type yet, so the CR 305.6 shortcut is not"
    );

    tap_all_mana_but(&mut engine, p0, Some(darksteel_citadel()));
    cast_with_floating(&mut engine, p0, evil_presence());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Enchant land asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&citadel));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![citadel],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !keywords(&engine, citadel).contains(KeywordSet::INDESTRUCTIBLE),
        "the printed keyword is gone"
    );
    assert!(
        engine
            .state()
            .object(citadel)
            .unwrap()
            .abilities(&engine.lookup)
            .is_empty(),
        "no ability its text prints is left"
    );
    let after_types = engine
        .state()
        .object(citadel)
        .unwrap()
        .characteristics()
        .types;
    assert!(
        after_types.contains(TypeSet::LAND) && after_types.contains(TypeSet::ARTIFACT),
        "still an Artifact Land: {after_types:?}"
    );
    let after = priority_offer(&engine);
    assert!(
        !after.abilities.contains(&(citadel, 0)),
        "the printed {{T}}: Add {{C}} is no longer offered"
    );
    assert!(
        after.mana_abilities.contains(&citadel),
        "the Swamp's own mana is offered instead"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: citadel })
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "it taps for {{B}}");
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "not its printed {{C}} any more"
    );
}
