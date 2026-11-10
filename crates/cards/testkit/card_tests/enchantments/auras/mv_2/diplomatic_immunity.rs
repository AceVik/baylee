//! `cards/enchantments/auras/mv_2/diplomatic_immunity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Diplomatic Immunity — {1}{U} Aura: "Enchant creature. Shroud. Enchanted
/// creature has shroud." Both printed sentences are about *not* being
/// targeted, so the scenario has to hold a target question open: a bare Elf
/// beside the enchanted one is what makes a refusal readable, because a
/// removal spell offered nothing at all would be refused for having no target
/// rather than for shroud. Vindicate's one menu — every permanent in the game
/// — is therefore read three ways: the unenchanted Elf is on it (the spell is
/// real), the enchanted Elf is not (the static grants), and the Aura itself is
/// not (the shroud it prints for its own body, without which pointing at the
/// Aura would strip the protection off the creature).
#[test]
#[allow(clippy::too_many_lines)] // two casts, and one menu read three ways
fn diplomatic_immunity_shrouds_the_creature_it_enchants_and_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[diplomatic_immunity(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "a printed 1/1 has none of it before the Aura lands on it"
    );

    // {1}{U}: the lands pay and the two Elves are tapped along with them,
    // which costs nothing here — they are the creatures this test is about,
    // not attackers, and the pool they leave is read below rather than
    // asserted.
    cast_from_hand(&mut engine, p0, diplomatic_immunity());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("an Aura targets as it is cast, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" is any creature on your side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let aura = on_battlefield(&engine, p0, diplomatic_immunity()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "it entered attached to the creature it was cast at"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "\"enchanted creature has shroud\" reaches the creature it holds"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "and the Elf nobody enchanted is still a bare 1/1"
    );
    assert!(
        keywords(&engine, aura).contains(KeywordSet::SHROUD),
        "the Aura prints shroud for itself, which is what keeps the grant \
         from being undone by pointing at what grants it"
    );

    // The removal spell, off what the first cast left in the pool — the
    // question it opens is the whole of what this card is.
    cast_with_floating(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bystander),
        "the unenchanted Elf is a legal target, so the spell was cast rather \
         than refused for want of one: {options:?}"
    );
    assert!(
        !options.contains(&host),
        "the enchanted creature has shroud and cannot be the target of a \
         spell (CR 702.18a): {options:?}"
    );
    assert!(
        !options.contains(&aura),
        "nor can the Aura that grants it: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .expect("the Elf the offer named");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(host)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the creature the spell could not name is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "while the one it could name was destroyed, so the spell did resolve"
    );
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and the Aura is still holding on to it"
    );
}
