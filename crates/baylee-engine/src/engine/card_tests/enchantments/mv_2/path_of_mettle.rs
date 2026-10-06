//! `cards/enchantments/mv_2/path_of_mettle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Path of Mettle prints "When this enchantment enters, it deals 1 damage to
/// each creature that doesn't have first strike, double strike, vigilance, or
/// haste", and the board is three identical printed 1/1 Llanowar Elves — two
/// of mine and one across the table — of which exactly one has been given
/// haste by Lightning Greaves before the enchantment arrives. The Greaves
/// equip for `{0}`, so nothing about the mana spent on the enchantment is
/// confounded by them, and both halves of the sentence get a witness of the
/// same body: one Elf dies beside the survivor of my own, and one dies on the
/// other side of the table, because "each creature" names no controller.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn path_of_mettle_spares_the_creature_that_has_one_of_its_four_keywords() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                mountain(),
                mountain(),
                llanowar_elves(),
                llanowar_elves(),
                lightning_greaves(),
            ],
        )
        .hand(0, &[path_of_mettle()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (armed, bare) = (elves[0], elves[1]);
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "an Elf across the table"
    );
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are out");
    assert!(
        !keywords(&engine, armed).contains(KeywordSet::HASTE),
        "nothing is equipped yet, so nothing has haste"
    );

    // Equip {0} (CR 702.6). Its whole price is the tap symbol, so the pool the
    // enchantment below is paid out of is exactly what the four lands hold —
    // and the ability index is read out of the offer rather than guessed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == greaves)
        .expect("Equip {0} is the only activated ability the Greaves print");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("equip costs its own tap symbol and no mana at all");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options.len(),
        2,
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        options.contains(&armed) && options.contains(&bare),
        "and the Elf across the table is not one of them: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![armed],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(armed))
    });

    let granted = keywords(&engine, armed);
    assert!(
        granted.contains(KeywordSet::HASTE),
        "the Greaves grant haste, the fourth of the four keywords the trigger \
         names: {granted:?}"
    );
    assert_eq!(
        pt(&engine, armed),
        (1, 1),
        "and they change no body, so it is still the printed 1/1 the bare Elf is"
    );

    // The Elves are named as the printing kept back, because they are the
    // creatures this test reads afterwards: what is tapped is the two Plains
    // and the two Mountains, which is {W}{W}{R}{R} for a {R}{W}.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Plains and two Mountains, and neither Elf tapped for anything"
    );
    cast_front_face(&mut engine, p0, path_of_mettle());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, path_of_mettle()).is_some(),
        "the enchantment resolved onto the table"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![armed],
        "1 damage on a printed 1/1 is lethal (CR 704.5g), so the only Elf of \
         mine still standing is the one the trigger's filter excluded"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the bare Elf the trigger did not exclude is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "\"each creature\" names no controller, so the Elf across the table \
         died to the same resolution"
    );
    assert!(
        keywords(&engine, armed).contains(KeywordSet::HASTE),
        "and the survivor kept the keyword that spared it"
    );
}
