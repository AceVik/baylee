//! `cards/enchantments/auras/mv_2/illuminated_wings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Illuminated Wings — {1}{U} Aura: "Enchant creature / Enchanted creature
/// has flying. / {2}, Sacrifice this Aura: Draw a card."
///
/// Both printed sentences are read off one board, because each is the other's
/// control. The Aura is cast onto this seat's own Elf with the opponent's Elf
/// standing beside it, so the target question shows that "creature" reaches
/// across the table while the granted flying lands only on the creature the
/// Aura is attached to. Sacrificing the Aura for the card is what makes the
/// static readable at all: the same Elf is a printed 1/1 again afterwards, so
/// the keyword came from the Aura and not from anything on the Elf. The {2}
/// is paid out of the mana the cast left floating, so the activation is a
/// real payment and not a label.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn illuminated_wings_grants_flying_to_its_host_and_sacrifices_itself_for_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[illuminated_wings()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "a Llanowar Elves prints no flying of its own"
    );

    // {1}{U} off the four Islands, with the Elf kept back so the only mana
    // sources this board offers are the lands.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, illuminated_wings());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("an Aura picks what it enchants, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"enchant creature\" is any creature on either side: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let wings = on_battlefield(&engine, p0, illuminated_wings()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(wings).and_then(|o| o.attached_to),
        Some(mine),
        "it entered attached to the creature it was cast on"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature has flying"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and the Elf across the table has nothing: the static is about the \
         creature the Aura holds and not about creatures"
    );

    // The Aura's own second sentence. The cast left two Islands' worth of
    // blue floating, which is exactly the {2} the ability charges, and the
    // only activated ability the card prints is the one the offer names.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == wings)
        .expect("`{2}, Sacrifice this Aura: Draw a card` is offered");
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(id, _)| *id == wings)
            .count(),
        1,
        "the Aura prints one activated ability, so it is offered once: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the mana the cast left floating pays the {2}");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "nothing the sacrifice asks is unanswerable"
    );

    assert!(
        on_battlefield(&engine, p0, illuminated_wings()).is_none(),
        "the Aura sacrificed itself, so it is gone from the table"
    );
    assert!(
        in_graveyard(&engine, p0, illuminated_wings()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "the Elf is a printed 1/1 again: the flying was the Aura's static, \
         and it left when the Aura did"
    );
}
