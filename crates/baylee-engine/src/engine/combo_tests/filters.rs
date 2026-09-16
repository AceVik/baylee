//! A spell or ability's own filter, read across a board built to fail it in both directions at once, and the cards that widen one by changing what a permanent or a spell *is* — Maskwood Nexus over a creature's types and over a spell's, Liquimetal Coating over an artifact. Each test asks the widened half and the untouched bystander in the same game, because a filter that reached the whole table passes the positive assertion on its own; the same board carries a two-clause filter whose clauses have to fail separately before either can be said to be doing the work. What does not belong here is a filter that comes off a *granted* keyword, which is `grants`, one that decides whose side a replacement reads, which is `doubling`, and one a copy handed the permanent, which is `copied_abilities`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Maskwood Nexus ("creatures you control are every creature type") plus
/// Riptide Laboratory ("{1}{U}, {T}: Return target Wizard **you control** to
/// its owner's hand").
///
/// The combination is the whole test: an Elf Druid is not a Wizard and the
/// Laboratory has nothing to point at, until the Nexus makes it one. What
/// the Nexus does *not* change is whose creature it is, and the bystander
/// says so — the opponent's Snapcaster Mage is a printed Wizard, a legal
/// target for a Laboratory that had read "target Wizard", and must stay
/// unoffered.
#[test]
fn maskwood_makes_your_elf_a_wizard_and_leaves_their_wizard_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, forest())
        .battlefield(
            0,
            &[
                riptide_laboratory(),
                island(),
                island(),
                maskwood_nexus(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, riptide_laboratory()).expect("the laboratory is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their wizard");
    tap_mana_except(&mut engine, p0, lab);

    let index = offered_ability(&engine, lab).expect("the laboratory has something to return");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: lab,
                ability_index: index,
            },
        )
        .unwrap();

    let options = target_options(&engine);
    assert!(
        options.contains(&elves),
        "the Nexus made the Elf a Wizard: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" does not reach the opponent's Wizard: {options:?}"
    );
}

/// The other half of the same claim: without the Nexus the Laboratory is not
/// offered at all, because the only Wizard at the table is across it.
///
/// Written as its own game rather than as a second half of the one above,
/// because the interesting failure is a Laboratory that *is* offered — and
/// a test that had already activated it could not see that.
#[test]
fn without_the_nexus_the_laboratory_has_nothing_to_return() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(71, forest())
        .battlefield(
            0,
            &[riptide_laboratory(), island(), island(), llanowar_elves()],
        )
        .battlefield(1, &[snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, riptide_laboratory()).expect("the laboratory is out");
    tap_mana_except(&mut engine, p0, lab);
    assert!(
        offered_ability(&engine, lab).is_none(),
        "an ability with no legal target is not an action"
    );
}

/// Liquimetal Coating ("{T}: Target permanent becomes an artifact in
/// addition to its other types until end of turn") plus Fracture ("Destroy
/// target artifact, enchantment, or planeswalker").
///
/// This is the combination in the direction that reaches *across* the table,
/// and the one the Coating's own filter bug would have made unreadable: with
/// `Filter::Any` every permanent in the game became an artifact, so Fracture
/// would have offered the whole board and the test would still have passed
/// its positive half. The bystander is the opponent's *second* creature,
/// untouched by the Coating, and it is what makes the assertion mean
/// anything.
#[test]
fn a_plated_creature_becomes_a_legal_fracture_target_and_its_neighbour_does_not() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(72, forest())
        .battlefield(0, &[liquimetal_coating(), plains(), swamp()])
        .hand(0, &[fracture()])
        .battlefield(1, &[llanowar_elves(), snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let coating = on_battlefield(&engine, p0, liquimetal_coating()).expect("the coating is out");
    let plated = on_battlefield(&engine, p1, llanowar_elves()).expect("their elves");
    let bystander = on_battlefield(&engine, p1, snapcaster_mage()).expect("their mage");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: coating,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![plated],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(plated)
            .is_some_and(|o| o.characteristics().types.intersects(TypeSet::ARTIFACT))
    });

    let spell = in_hand(&engine, p0, fracture()).expect("fracture in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let options = target_options(&engine);
    assert!(
        options.contains(&plated),
        "the Coating made their Elf an artifact, so Fracture can destroy it: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "the creature beside it was never plated: {options:?}"
    );
}

/// Skyclave Apparition against a board that is wrong in both directions at
/// once: "nonland, nontoken permanent **you don't control** with mana value
/// **4 or less**" is two clauses, and a test whose board fails only one of
/// them cannot tell which clause is doing the work.
///
/// So the opponent gets a two-drop that must be offered and a five-drop
/// that must not, and I keep a one-drop of my own that must not be offered
/// either — the same permanent the mana-value clause would happily allow.
#[test]
fn skyclave_reaches_their_cheap_permanent_and_neither_their_dear_one_nor_mine() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(77, forest())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[skyclave_apparition()])
        .battlefield(1, &[snapcaster_mage(), sea_gate_loremaster()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my one-drop");
    let cheap = on_battlefield(&engine, p1, snapcaster_mage()).expect("their two-drop");
    let dear = on_battlefield(&engine, p1, sea_gate_loremaster()).expect("their five-drop");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let apparition = in_hand(&engine, p0, skyclave_apparition()).expect("apparition in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: apparition })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let options = target_options(&engine);
    assert!(
        options.contains(&cheap),
        "their two-drop is what the trigger is for: {options:?}"
    );
    assert!(
        !options.contains(&dear),
        "mana value 5 is one over the limit: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "\"you don't control\" does not reach my own board: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cheap],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(cheap)
            .is_none_or(|o| o.zone != Zone::Battlefield)
    });
    assert_eq!(
        engine.state().object(cheap).map(|o| o.zone),
        Some(Zone::Exile),
        "the chosen permanent is exiled, and the bystanders are still where they were"
    );
    assert_eq!(
        engine.state().object(dear).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        engine.state().object(mine).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
}

/// Maskwood Nexus makes a *spell* every creature type, so Reflections of
/// Littjara copies it whatever the card is printed as.
///
/// Reported from live play: with the Nexus and Reflections (Ally) on the
/// battlefield, Jin-Gitaxias — printed a Praetor and no Ally at all — was
/// cast and not copied, while General Tazri's search under the same Nexus
/// did offer non-Ally creature *cards*. The difference is the zone. A card
/// in a library has been sitting in the projected set since the Nexus
/// arrived; a spell is put on the stack a moment before the cast trigger
/// asks what it is, and nothing re-projected it there (entry 43), so the
/// trigger read the printed types and found no Ally.
///
/// Llanowar Elves stands in for Jin-Gitaxias: an Elf Druid, no Ally, one
/// mana, and the test is about the type the Nexus grants rather than the
/// card that has it.
#[test]
fn the_nexus_makes_a_spell_the_chosen_type_for_littjara() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(113, forest())
        .battlefield(
            0,
            &[
                maskwood_nexus(),
                island(),
                island(),
                island(),
                island(),
                island(),
                forest(),
            ],
        )
        .hand(0, &[reflections_of_littjara(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, reflections_of_littjara());
    settle(&mut engine);
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!(
            "the enchantment names a creature type as it enters: {:?}",
            engine.pending()
        )
    };
    let ally = baylee_core::generated::subtypes::creature::ALLY;
    assert!(options.contains(&ally), "Ally is a creature type");
    engine
        .apply(player, PlayerAction::ChooseSubtype(ally))
        .unwrap();
    settle(&mut engine);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        2,
        "the Elf is an Ally while it is a spell, so Reflections copied it",
    );
}
