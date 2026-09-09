//! Cards played *together*: one card changes what another is legally
//! offered.
//!
//! A single-card test asks whether a card's own rules text works. This
//! module asks the question a game actually poses — whether two cards that
//! have never met produce the answer the rules give — and it asks it through
//! `LegalActions` and `Pending`, because that is the only place a client can
//! see an answer. Every test here carries a **bystander**: a permanent that
//! must *not* be offered. An interaction that reaches one permanent too many
//! looks identical to a working one when the board holds a single candidate,
//! which is how a filter that plated every permanent in the game sat in the
//! pool unnoticed.
//!
//! The bystander is usually across the table, because the two ways a scope
//! goes wrong are symmetrical: an effect that should stay on your own side
//! reaching an opponent's board, and one aimed at an opponent coming back to
//! your own.

use super::testkit::*;
use super::*;

fn island() -> baylee_core::ids::CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn plains() -> baylee_core::ids::CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn swamp() -> baylee_core::ids::CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn forest() -> baylee_core::ids::CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn riptide_laboratory() -> baylee_core::ids::CardIndex {
    card_index("444d50dd-a44a-42db-bbf6-d0978e3bd6a3")
}
fn maskwood_nexus() -> baylee_core::ids::CardIndex {
    card_index("9b2cdbed-c733-409b-b0e4-2c8960c25111")
}
fn snapcaster_mage() -> baylee_core::ids::CardIndex {
    card_index("2bb2eda7-3b38-4c56-870f-c3218a1056f5")
}
fn llanowar_elves() -> baylee_core::ids::CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}
fn liquimetal_coating() -> baylee_core::ids::CardIndex {
    card_index("f4bdc551-c2eb-4a34-a3e3-b4a017c925af")
}
fn fracture() -> baylee_core::ids::CardIndex {
    card_index("f21d0319-0509-4ac1-b6e3-10955a26fd7a")
}
fn glasspool_mimic() -> baylee_core::ids::CardIndex {
    card_index("c178953c-3888-4edd-9d0c-265bd82b1d24")
}
fn earth_king_s_lieutenant() -> baylee_core::ids::CardIndex {
    card_index("9da9248d-1201-447f-b6c2-2b64af4f71c4")
}
fn ondu_cleric() -> baylee_core::ids::CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn werefox_bodyguard() -> baylee_core::ids::CardIndex {
    card_index("d5ee2ced-29f4-430f-962e-2f930b92624c")
}

/// The one *non-mana* activated ability `source` is offering right now.
///
/// `LegalActions::abilities` carries a permanent's mana abilities too —
/// `mana_abilities` is the narrower list of lands that tap for their basic
/// type (CR 305.6), and a land with a printed `{T}: Add {C}` is offered
/// through the general list like anything else. A test that took the first
/// entry would activate Riptide Laboratory's mana ability and then wonder
/// why nothing asked it for a target.
#[track_caller]
fn offered_ability(
    engine: &Engine<RegistryLookup>,
    source: baylee_core::ids::ObjectId,
) -> Option<u32> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let abilities = engine
        .state()
        .object(source)
        .expect("the source is on the battlefield")
        .abilities(&RegistryLookup);
    legal.abilities.iter().find_map(|(id, index)| {
        (*id == source
            && !matches!(
                abilities.get(*index as usize),
                Some(AbilityDef::Activated {
                    mana_ability: true,
                    ..
                })
            ))
        .then_some(*index)
    })
}

/// The options a target choice is offering right now.
#[track_caller]
fn target_options(engine: &Engine<RegistryLookup>) -> Vec<baylee_core::ids::ObjectId> {
    match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("expected a target choice, got {other:?}"),
    }
}

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

/// Glasspool Mimic ("enter as a copy of a creature **you control**") plus
/// Earth King's Lieutenant ("When this creature enters, put a +1/+1 counter
/// on each other Ally creature you control", and "Whenever another Ally you
/// control enters, put a +1/+1 counter on this creature").
///
/// A copy is not a still picture: the Mimic that arrives as a Lieutenant
/// *has* the Lieutenant's enters-trigger and fires it, and the original then
/// sees an Ally enter and grows itself. Three cards' text has to agree for
/// the numbers below to come out, which is the reason to run it rather than
/// reason about it.
#[test]
fn a_mimic_copying_an_ally_fires_what_it_copied_and_cannot_copy_across_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(73, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                earth_king_s_lieutenant(),
                ondu_cleric(),
            ],
        )
        .hand(0, &[glasspool_mimic()])
        .battlefield(1, &[snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lieutenant =
        on_battlefield(&engine, p0, earth_king_s_lieutenant()).expect("lieutenant deployed");
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric deployed");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their mage");
    assert_eq!(pt(&engine, lieutenant), (1, 1));
    assert_eq!(pt(&engine, cleric), (1, 1));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let mimic = in_hand(&engine, p0, glasspool_mimic()).expect("mimic in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mimic })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let options = target_options(&engine);
    assert!(
        options.contains(&lieutenant) && options.contains(&cleric),
        "either of my creatures may be copied: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"a creature you control\" does not reach across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();

    // The copy's own enters-trigger counters every *other* Ally, and the
    // original Lieutenant's rally trigger then counters itself: 1/1 → 3/3
    // for the original, 1/1 → 2/2 for the cleric.
    pass_until(&mut engine, |e| pt(e, lieutenant) == (3, 3));
    assert_eq!(
        pt(&engine, cleric),
        (2, 2),
        "the copy's enters-trigger reached the cleric too"
    );
}

/// Three cards, and the question the first two only half asked: a Glasspool
/// Mimic copying a Snapcaster Mage is a *Wizard*, which is what Riptide
/// Laboratory returns — and what comes back to the hand has to be a Mimic
/// again.
///
/// A copy lasts exactly as long as the object does (CR 707.2a); the object
/// ends at the zone change (CR 400.7). Left as a copy, the card in hand
/// would be a Snapcaster Mage that costs `{2}{U}`, and recasting it would
/// find no enters-as-a-copy ability at all — a 0/0 that dies on arrival.
/// That is the cost of storing copied abilities on the object, so this is
/// the test that says the storing is paid for.
#[test]
fn a_mimic_that_copied_a_wizard_comes_home_a_mimic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(74, forest())
        .battlefield(
            0,
            &[
                riptide_laboratory(),
                island(),
                island(),
                island(),
                island(),
                island(),
                snapcaster_mage(),
            ],
        )
        .hand(0, &[glasspool_mimic()])
        .battlefield(1, &[snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, riptide_laboratory()).expect("the laboratory is out");
    let mine = on_battlefield(&engine, p0, snapcaster_mage()).expect("my wizard");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their wizard");
    tap_mana_except(&mut engine, p0, lab);

    let mimic = in_hand(&engine, p0, glasspool_mimic()).expect("mimic in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mimic })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = target_options(&engine);
    assert!(
        !options.contains(&theirs),
        "the Mimic copies a creature *you* control: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| pt(e, mimic) == (2, 1));

    let index = offered_ability(&engine, lab).expect("a copy of a Wizard is a Wizard");
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
        options.contains(&mimic),
        "the copy is a Wizard the Laboratory can save: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you control\" still does not reach across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mimic],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| in_hand(e, p0, glasspool_mimic()).is_some());

    let back = in_hand(&engine, p0, glasspool_mimic()).expect("the copy came home");
    assert_eq!(back, mimic, "the same handle, one zone later");
    let obj = engine.state().object(back).expect("it is in the hand");
    assert_eq!(
        (obj.base.power.unwrap_or(0), obj.base.toughness.unwrap_or(0)),
        (0, 0),
        "Glasspool Mimic is printed 0/0; the Snapcaster it copied was 2/1"
    );
    assert!(
        obj.original_base.is_none(),
        "the pre-copy base was spent, not kept for a second zone change"
    );
    assert!(
        matches!(
            obj.abilities(&RegistryLookup).first(),
            Some(AbilityDef::CopyOnEnter { .. })
        ),
        "cast again it must still be able to enter as a copy"
    );
}

/// The same pair from the other side: an ability outlives the copy that
/// activated it.
///
/// Werefox Bodyguard's second ability sacrifices itself as part of its cost
/// (`{1}{W}, Sacrifice this creature: You gain 2 life`), so a Glasspool Mimic
/// copying it is in the graveyard *before* the ability resolves — and stops
/// being a copy on the way (CR 400.7). The ability does not care: it has been
/// its own object on the stack since it was activated (CR 608.2). Read back
/// off the source at resolution time it would find Glasspool Mimic's own
/// one-entry list and index 1 in it, which is nothing at all.
#[test]
fn an_ability_the_copy_paid_for_with_its_life_still_resolves() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(75, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                plains(),
                plains(),
                plains(),
                werefox_bodyguard(),
            ],
        )
        .hand(0, &[glasspool_mimic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fox = on_battlefield(&engine, p0, werefox_bodyguard()).expect("the bodyguard is out");
    let life = engine.state().players[0].life;
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }

    let mimic = in_hand(&engine, p0, glasspool_mimic()).expect("mimic in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mimic })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![fox] })
        .unwrap();
    pass_until(&mut engine, |e| pt(e, mimic) == (2, 2));
    // The copied enters-trigger is "exile up to one **other** target non-Fox
    // creature", and both creatures at the table are the same Fox: the choice
    // arrives with nothing in it and is answered with nothing.
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });

    // No tap in the cost, so the copy may do this the turn it arrives.
    let index = offered_ability(&engine, mimic).expect("the copy carries what it copied");
    assert_eq!(index, 1, "the sacrifice ability, not the enters-trigger");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mimic,
                ability_index: index,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[0].life != life);

    assert_eq!(
        engine.state().players[0].life,
        life + 2,
        "the ability resolved from the graveyard side of its own cost"
    );
    let obj = engine.state().object(mimic).expect("the copy is in a zone");
    assert_eq!(obj.zone, crate::zone::Zone::Graveyard, "sacrificed");
    assert_eq!(
        (obj.base.power.unwrap_or(0), obj.base.toughness.unwrap_or(0)),
        (0, 0),
        "the card in the graveyard is Glasspool Mimic, not the 2/2 it copied"
    );
}
