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
use crate::choice::YesNoPrompt;

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
fn karn_the_great_creator() -> baylee_core::ids::CardIndex {
    card_index("a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1")
}
/// An artifact *land*, so the ability Karn's lock has to stop is a mana
/// ability — the case that says the lock reads CR 605.1 rather than
/// treating a mana ability as something other than an activated one.
fn vault_of_whispers() -> baylee_core::ids::CardIndex {
    card_index("09496421-74e4-466a-9546-56f2a0c8eef4")
}
fn skyclave_apparition() -> baylee_core::ids::CardIndex {
    card_index("d90af00a-d322-4265-9954-7b1e80702e18")
}
/// Mana value five, and nothing else about it matters: it is here to be
/// one over Skyclave Apparition's limit.
fn sea_gate_loremaster() -> baylee_core::ids::CardIndex {
    card_index("6eed122b-9760-47fd-8ba2-adeda8054e0d")
}
fn mycosynth_lattice() -> baylee_core::ids::CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}
fn chromatic_lantern() -> baylee_core::ids::CardIndex {
    card_index("539f5396-d99a-417d-a84c-dff7930b5900")
}
fn privileged_position() -> baylee_core::ids::CardIndex {
    card_index("abd62af0-c17d-4f62-af15-9ea83037b990")
}
fn lightning_greaves() -> baylee_core::ids::CardIndex {
    card_index("ca204b66-8d0c-431a-8d34-282f7c2d17da")
}
fn vindicate() -> baylee_core::ids::CardIndex {
    card_index("63c1ac21-e3d8-40c2-8c09-3f31c52992ef")
}
fn doubling_season() -> baylee_core::ids::CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}
fn panharmonicon() -> baylee_core::ids::CardIndex {
    card_index("76678885-3674-443d-b9a2-2a460cf6aac0")
}
fn darksteel_forge() -> baylee_core::ids::CardIndex {
    card_index("9b3bec05-441f-4fdf-8b51-69fa8613fcd4")
}
fn crib_swap() -> baylee_core::ids::CardIndex {
    card_index("2987c385-011a-4032-a516-a46d1e9dc9e8")
}
fn rite_of_replication() -> baylee_core::ids::CardIndex {
    card_index("fb60739e-1dc3-481d-a056-ad72e665c680")
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

/// Whether `LegalActions` is offering *any* ability of `source` right now,
/// a mana ability included.
///
/// [`offered_ability`] deliberately steps over mana abilities, because most
/// tests here are about the one ability a permanent has that is not one.
/// Karn's lock is the opposite question — it stops every activated ability
/// of an artifact, and a mana ability is one (CR 605.1).
#[track_caller]
fn offers_an_ability(engine: &Engine<RegistryLookup>, source: baylee_core::ids::ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, _)| *id == source) || legal.mana_abilities.contains(&source)
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

/// Karn, the Great Creator plus an artifact land across the table.
///
/// `karns_lock_spares_a_teammate` proved the lock refuses the right seat;
/// this asks the other half of the same question, which nothing asked:
/// whether the seat it locks is still being *offered* what it cannot do.
/// It was. The refusal lived in `apply` alone, and that is invisible from
/// Karn's own side of the table — the abilities the lock stops are on the
/// opponent's board, and an opponent's board is not what a test driving
/// Karn looks at.
///
/// Two bystanders, because the lock has two edges. My own Llanowar Elves
/// taps for mana exactly as before — the lock is about artifacts, not about
/// me — and Karn's controller keeps their own Vault, which is the sentence
/// `karns_lock_spares_a_teammate` was written for, read here through the
/// offer instead of through the refusal.
#[test]
fn karns_lock_takes_their_artifact_off_the_list_and_leaves_the_rest_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(76, forest())
        .battlefield(0, &[vault_of_whispers(), llanowar_elves()])
        .battlefield(1, &[karn_the_great_creator(), vault_of_whispers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_vault = on_battlefield(&engine, p0, vault_of_whispers()).expect("my vault");
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves");
    let their_vault = on_battlefield(&engine, p1, vault_of_whispers()).expect("their vault");

    assert!(
        !offers_an_ability(&engine, my_vault),
        "Karn's lock stops my artifact land's mana ability, so it must not be offered"
    );
    assert!(
        offers_an_ability(&engine, my_elves),
        "the lock is about artifacts; an Elf Druid taps for mana as before"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: my_vault,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the two probes have to agree: what is not offered is not applied"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "expected the opponent to hold priority, got {:?}",
        engine.pending()
    );
    assert!(
        offers_an_ability(&engine, their_vault),
        "the lock spares its own controller: {:?}",
        engine.pending()
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: their_vault,
                ability_index: 0,
            },
        )
        .expect("Karn's controller may still tap their own artifact land");
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

/// Karn, the Great Creator plus Mycosynth Lattice — the lock the pair is
/// famous for — with a Chromatic Lantern under it, so that all three doors
/// onto `LegalActions` are shut at once.
///
/// [`karns_lock_takes_their_artifact_off_the_list_and_leaves_the_rest_alone`]
/// reaches the lock through `LegalActions::abilities`, because a Vault of
/// Whispers prints its own `{T}: Add {B}`. A Plains prints nothing: its mana
/// comes off the type line (CR 305.6) and is offered through the separate
/// `mana_abilities` list. The Lantern adds the third — "lands you control
/// have `{T}`: Add one mana of any color" is *granted*, and a granted
/// ability is offered from its own loop under a synthetic index. All three
/// are activated abilities of an artifact once the Lattice has spoken, and
/// each was a separate `push` that had to learn the same word.
///
/// The bystander is across the table and is the same card: Karn's controller
/// keeps their own Plains, because the lock reads "artifacts your opponents
/// control" and the Lattice does not change whose permanent anything is.
#[test]
fn karn_and_the_lattice_lock_their_basic_lands_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(78, forest())
        .battlefield(0, &[plains(), llanowar_elves(), chromatic_lantern()])
        .battlefield(
            1,
            &[karn_the_great_creator(), mycosynth_lattice(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_plains = on_battlefield(&engine, p0, plains()).expect("my plains");
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves");
    let my_lantern = on_battlefield(&engine, p0, chromatic_lantern()).expect("my lantern");
    let their_plains = on_battlefield(&engine, p1, plains()).expect("their plains");

    assert!(
        !offers_an_ability(&engine, my_plains),
        "under the Lattice my Plains is an artifact, so neither its own mana \
         ability nor the one the Lantern grants it may be offered"
    );
    assert!(
        !offers_an_ability(&engine, my_elves),
        "so is my Elf Druid, and so is its"
    );
    assert!(
        !offers_an_ability(&engine, my_lantern),
        "the Lantern is an artifact whatever the Lattice says"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: my_plains })
            .is_err(),
        "the action validates against the offered list, so taking the land \
         off it is what refuses the tap"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        offers_an_ability(&engine, their_plains),
        "the Lattice does not change whose permanent anything is: {:?}",
        engine.pending()
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: their_plains,
            },
        )
        .expect("Karn's controller taps their own land as before");
}

/// Both seats holding Vindicate ("Destroy target permanent"), with a
/// Privileged Position and an Elf on your side and a Wizard on theirs.
///
/// Vindicate is the removal to ask with because it targets *any* permanent:
/// the options it offers are the whole table, so what is missing from them
/// is a statement about the grant and not about the spell's own filter.
fn a_table_under_a_privileged_position() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(80, forest())
        .battlefield(
            0,
            &[
                privileged_position(),
                llanowar_elves(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[snapcaster_mage(), plains(), swamp(), forest()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// Taps everything that makes mana for `seat` and casts `card` from its
/// hand, leaving the engine on the spell's target choice.
#[track_caller]
fn cast_from_hand(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) {
    let spell = in_hand(engine, seat, card).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine
        .apply(seat, PlayerAction::CastSpell { card: spell })
        .unwrap();
}

/// Privileged Position ("**Other** permanents **you control** have
/// hexproof") against an opponent's Vindicate.
///
/// Three answers on one board, and the card is wrong if any of them flips.
/// Your Elf is hidden, because that is what the grant is for. Their Wizard
/// is not, because a grant that reached across the table would read exactly
/// the same on a board with one creature on it. And the Position **itself**
/// is not, because it says "other" — the word that makes the enchantment
/// the one thing an opponent can answer it with, and a filter written
/// `ControlledByYou` alone would quietly protect it.
#[test]
fn a_privileged_position_hides_your_board_from_them_but_not_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_privileged_position();
    // Not `reach_main_phase`: reaching the *other* seat's turn crosses a
    // combat phase, and that helper answers priority and nothing else.
    reach_their_main_phase(&mut engine, p1);

    let position = on_battlefield(&engine, p0, privileged_position()).expect("your enchantment");
    let yours = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their mage");

    cast_from_hand(&mut engine, p1, vindicate());
    let options = target_options(&engine);
    assert!(
        !options.contains(&yours),
        "their Vindicate may not target a creature the Position gave \
         hexproof (CR 702.11b): {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "the grant is to permanents *you* control, so their own creature is \
         still a legal target: {options:?}"
    );
    assert!(
        options.contains(&position),
        "the Position says \"other\" and does not protect itself: {options:?}"
    );
}

/// The same board, the same spell, cast by the seat that owns the grant.
///
/// Hexproof is "can't be the target of spells or abilities *your opponents*
/// control" (CR 702.11b), so this is the half that a `Filter::ControlledBy`
/// mistake would take away. A creature under a Privileged Position that its
/// own controller could no longer target would break every aura, every
/// pump spell and every equip in the deck built around it — and no test
/// asking the opponent's question would notice.
#[test]
fn your_own_removal_still_reaches_the_creature_you_gave_hexproof() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_privileged_position();
    reach_main_phase(&mut engine, p0);

    let yours = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their mage");

    cast_from_hand(&mut engine, p0, vindicate());
    let options = target_options(&engine);
    assert!(
        options.contains(&yours),
        "hexproof stops opponents only, so your own spell still sees your \
         own creature: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and nothing about the Position was ever between you and their \
         board: {options:?}"
    );
}

/// Lightning Greaves ("Equipped creature has haste and **shroud**") and the
/// controller's own Vindicate.
///
/// The counterpart to the two above, on the distinction the two keywords
/// exist for: shroud is "can't be the target of spells or abilities"
/// (CR 702.18b) full stop, so the same seat that granted it is refused —
/// which is the whole reason a player equips Greaves and then complains
/// they cannot aura the creature. The unequipped creature beside it is the
/// bystander, and it says the refusal came from the attachment rather than
/// from the spell.
#[test]
fn greaves_hide_the_creature_they_are_on_from_you_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(81, forest())
        .battlefield(
            0,
            &[
                lightning_greaves(),
                llanowar_elves(),
                snapcaster_mage(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves");
    let equipped = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let bystander = on_battlefield(&engine, p0, snapcaster_mage()).expect("your mage");
    let across = on_battlefield(&engine, p1, ondu_cleric()).expect("their cleric");

    let equip = offered_ability(&engine, greaves).expect("equip {0} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: greaves,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![equipped],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().object(equipped).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::SHROUD)
        })
    });

    cast_from_hand(&mut engine, p0, vindicate());
    let options = target_options(&engine);
    assert!(
        !options.contains(&equipped),
        "shroud refuses its own controller as well (CR 702.18b): {options:?}"
    );
    assert!(
        options.contains(&bystander),
        "the creature beside it is wearing nothing: {options:?}"
    );
    assert!(
        options.contains(&greaves),
        "the Equipment grants to the creature it is attached to, never to \
         itself: {options:?}"
    );
    assert!(
        options.contains(&across),
        "and the board across the table is untouched by any of it: {options:?}"
    );
}

/// How many tokens `seat` controls.
///
/// Counted by `token` rather than by "a permanent the test did not seed": a
/// token is the one permanent that carries its definition instead of a
/// card, which is exactly the object Doubling Season's first sentence is
/// about.
#[must_use]
fn tokens_controlled(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.token.is_some())
        })
        .count()
}

/// Taps everything `seat` has and activates the one non-mana ability
/// `source` is offering, leaving it on the stack.
#[track_caller]
fn activate(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    source: baylee_core::ids::ObjectId,
) {
    tap_mana_except(engine, seat, source);
    let index = offered_ability(engine, source).expect("the permanent is offering its ability");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .unwrap();
}

/// Doubling Season ("If an effect would create one or more tokens under
/// **your** control, it creates twice that many of those tokens instead")
/// with a Maskwood Nexus on each side of the table.
///
/// The same token maker on both sides is the whole design of the test: one
/// ability, activated by two seats, so the only thing that differs between
/// the two counts below is who controls the enchantment. A replacement that
/// asked the *resolving* effect's own controller whether it controlled the
/// effect — which is a question that answers yes for everyone — would double
/// both, and a board with a single Nexus on it could never say so.
#[test]
fn doubling_season_doubles_your_tokens_and_not_the_ones_across_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(82, forest())
        .battlefield(
            0,
            &[
                doubling_season(),
                maskwood_nexus(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .battlefield(1, &[maskwood_nexus(), plains(), swamp(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, maskwood_nexus()).expect("my nexus");
    activate(&mut engine, p0, mine);
    pass_until(&mut engine, |e| tokens_controlled(e, p0) > 0);
    assert_eq!(
        tokens_controlled(&engine, p0),
        2,
        "one Shapeshifter is created twice under my own Doubling Season"
    );

    reach_their_main_phase(&mut engine, p1);
    let theirs = on_battlefield(&engine, p1, maskwood_nexus()).expect("their nexus");
    activate(&mut engine, p1, theirs);
    pass_until(&mut engine, |e| tokens_controlled(e, p1) > 0);
    assert_eq!(
        tokens_controlled(&engine, p1),
        1,
        "their Nexus creates its token under *their* control, which is not \
         what my enchantment replaces"
    );
    assert_eq!(
        tokens_controlled(&engine, p0),
        2,
        "and nothing on their turn arrived on my side of the table"
    );
}

/// Panharmonicon ("If an artifact or creature entering causes a triggered
/// ability of a permanent **you control** to trigger, that ability triggers
/// an additional time") over Earth King's Lieutenant ("When this creature
/// enters, put a +1/+1 counter on each other Ally creature you control").
///
/// The counters are the readout: the Lieutenant's trigger places exactly one
/// per firing, so an Ondu Cleric that ends at 3/3 was given two and the
/// trigger fired twice. Both seats hold a Cleric and cast a Lieutenant, so
/// the artifact's controller is the only difference between the two numbers
/// — and the Cleric across the table is the bystander for the *trigger's*
/// own "you control" at the same time.
#[test]
fn panharmonicon_fires_your_enters_trigger_twice_and_theirs_once() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[panharmonicon(), ondu_cleric(), forest(), plains()])
        .hand(0, &[earth_king_s_lieutenant()])
        .battlefield(1, &[ondu_cleric(), forest(), plains()])
        .hand(1, &[earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("my cleric");
    let their_cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("their cleric");

    cast_from_hand(&mut engine, p0, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "a 1/1 Ally under two firings of \"a +1/+1 counter on each other \
         Ally you control\""
    );
    assert_eq!(
        pt(&engine, their_cleric),
        (1, 1),
        "the Lieutenant's own trigger says \"you control\", so their Ally \
         was never in it"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, their_cleric),
        (2, 2),
        "their Lieutenant is not a permanent I control, so my Panharmonicon \
         does not multiply its trigger"
    );
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "and my own Ally took nothing from their turn"
    );
}

/// Darksteel Forge ("**Artifacts you control** have indestructible"), a
/// Liquimetal Coating on each side and an Elf beside the Forge, with a
/// Vindicate ("Destroy target permanent") in each hand.
///
/// Indestructible is not hexproof, which is why every test below reads the
/// battlefield rather than the options list: the permanent is still a legal
/// target and the spell still resolves — what it fails to do is destroy it
/// (CR 702.12b).
fn a_table_under_a_darksteel_forge(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, forest())
        .battlefield(
            0,
            &[
                darksteel_forge(),
                liquimetal_coating(),
                llanowar_elves(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[liquimetal_coating(), plains(), swamp(), forest()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// Resolves `seat`'s Vindicate onto `victim` and returns whether it is still
/// on the battlefield afterwards.
#[track_caller]
fn vindicated(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    victim: baylee_core::ids::ObjectId,
) -> bool {
    cast_from_hand(engine, seat, vindicate());
    let options = target_options(engine);
    assert!(
        options.contains(&victim),
        "indestructible does not stop targeting, so the spell has to be \
         allowed to point at it before its failure means anything: \
         {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .contains(&victim)
}

/// The Forge under their removal: my artifact survives a spell that resolved.
#[test]
fn a_darksteel_forge_keeps_your_artifact_through_their_removal() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_darksteel_forge(84);
    reach_their_main_phase(&mut engine, p1);

    let mine = on_battlefield(&engine, p0, liquimetal_coating()).expect("my coating");
    assert!(
        vindicated(&mut engine, p1, mine),
        "\"artifacts you control have indestructible\" — destruction does \
         nothing to it (CR 702.12b)"
    );
}

/// The counter-probe on the same board: the grant is to **artifacts** I
/// control, not to everything I control.
///
/// Its own game rather than a second spell in the one above, because the
/// interesting failure is a Forge that saved the Elf too — and a test that
/// had already spent the turn's mana could not ask.
#[test]
fn the_forge_is_not_a_shield_over_the_rest_of_your_board() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_darksteel_forge(85);
    reach_their_main_phase(&mut engine, p1);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves");
    assert!(
        !vindicated(&mut engine, p1, elf),
        "the Elf is not an artifact and the Forge never mentioned it"
    );
}

/// And the direction across the table: their artifact is not mine, so my own
/// Forge does not save it from my own Vindicate.
#[test]
fn the_forge_does_not_reach_the_artifact_across_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_darksteel_forge(86);
    reach_main_phase(&mut engine, p0);

    let theirs = on_battlefield(&engine, p1, liquimetal_coating()).expect("their coating");
    assert!(
        !vindicated(&mut engine, p0, theirs),
        "\"artifacts **you** control\" is the whole scope of the grant"
    );
}

/// Doubling Season's **second** sentence ("If an effect would put one or
/// more counters on a permanent **you control**, it puts twice that many of
/// those counters on that permanent instead") over the same Earth King's
/// Lieutenant.
///
/// Deliberately the same board and the same two numbers as
/// [`panharmonicon_fires_your_enters_trigger_twice_and_theirs_once`],
/// reached the other way round: there one counter was placed twice, here
/// two counters are placed once. Read together they say the two rules are
/// separate machines that happen to agree here — and the pair is what a
/// board with only one of the enchantments on it can never show.
#[test]
fn doubling_season_doubles_the_counters_on_your_own_permanents_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(87, forest())
        .battlefield(0, &[doubling_season(), ondu_cleric(), forest(), plains()])
        .hand(0, &[earth_king_s_lieutenant()])
        .battlefield(1, &[ondu_cleric(), forest(), plains()])
        .hand(1, &[earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("my cleric");
    let their_cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("their cleric");

    cast_from_hand(&mut engine, p0, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "one +1/+1 counter, put on a permanent I control, is put twice"
    );
    assert_eq!(
        pt(&engine, their_cleric),
        (1, 1),
        "the Lieutenant's trigger never reached their Ally to begin with"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, their_cleric),
        (2, 2),
        "their Ally is not a permanent I control, so my enchantment does \
         not replace the counter going onto it"
    );
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "and my own Ally took nothing from their turn"
    );
}

/// Permanents `seat` controls that no card stands behind.
///
/// [`tokens_controlled`] reads `token`, which is the token *definition* a
/// Treasure or a Shapeshifter was stamped out of, and a token copy of a
/// creature has none: `CreateTokenCopyOf` builds its object out of the
/// copied characteristics and stamps no definition on it. So the engine has
/// two notions of "this is a token" and the copy branches only satisfy the
/// second — the absence of a card behind the permanent — which is what this
/// counts.
#[must_use]
fn cardless_permanents(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.card.is_none())
        })
        .count()
}

/// A duel where seat 0 holds `spell` and seat 1 has a creature to point it
/// at, with a Doubling Season on the seat `season` names.
///
/// Moving one enchantment across the table is the only difference between
/// the two tests that use each table, which is what makes the pair of
/// numbers mean anything: a rule read off the wrong seat gives the same
/// answer on a board that has only one Doubling Season on it.
fn a_table_with_a_season_on_one_side(
    seed: u64,
    season: usize,
    lands: &[baylee_core::ids::CardIndex],
    spell: baylee_core::ids::CardIndex,
) -> Engine<RegistryLookup> {
    let mut mine = lands.to_vec();
    let mut theirs = vec![llanowar_elves(), forest()];
    if season == 0 {
        mine.push(doubling_season());
    } else {
        theirs.push(doubling_season());
    }
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &mine)
        .hand(0, &[spell])
        .battlefield(1, &theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Casts seat 0's spell at seat 1's Llanowar Elves and lets it resolve.
///
/// The wizard asks its questions in its own order and the spell is not on
/// the stack until the last of them is answered, so the loop answers
/// whatever is in front of it rather than assuming a sequence. Rite of
/// Replication is what made that necessary: its kicker is asked *after* the
/// target choice, and a test that passed priority in between found an empty
/// stack, called the spell resolved and counted a board nothing had
/// happened to yet.
#[track_caller]
fn aim_at_their_elf(
    engine: &mut Engine<RegistryLookup>,
    spell: baylee_core::ids::CardIndex,
    pay_extra: bool,
) -> baylee_core::ids::ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let victim = on_battlefield(engine, p1, llanowar_elves()).expect("their elf");
    cast_from_hand(engine, p0, spell);
    let mut aimed = false;
    loop {
        match engine.pending() {
            // `pay_extra` answers every optional additional cost the same
            // way, which is all these tests need: the only spell here that
            // has one is Rite of Replication, and its kicker is the whole
            // question in the test that pays it.
            Pending::YesNo { .. } => {
                engine.apply(p0, PlayerAction::YesNo(pay_extra)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                let options = target_options(engine);
                assert!(
                    options.contains(&victim),
                    "their creature is a legal target"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![victim],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            _ => break,
        }
    }
    assert!(aimed, "the spell asked for its target");
    pass_until(engine, stack_is_empty);
    victim
}

/// Crib Swap ("Exile target creature. **Its controller** creates a 1/1
/// colorless Shapeshifter creature token") cast under my own Doubling
/// Season.
///
/// CR 614.1 asks whose control the tokens would be created *under*, not
/// whose spell is creating them, and this spell deliberately hands them to
/// the player whose creature was just exiled. So my enchantment has nothing
/// to replace here: the Shapeshifter is theirs, and my removal must not
/// make it a pair of them.
#[test]
fn my_season_does_not_double_the_shapeshifter_my_own_removal_hands_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine =
        a_table_with_a_season_on_one_side(88, 0, &[plains(), plains(), plains()], crib_swap());
    aim_at_their_elf(&mut engine, crib_swap(), false);

    assert_eq!(
        tokens_controlled(&engine, p1),
        1,
        "the token is created under the exiled creature's controller, and \
         my Doubling Season is on the other side of the table from it"
    );
    assert_eq!(
        tokens_controlled(&engine, p0),
        0,
        "and nothing arrived on my own board"
    );
}

/// The same spell, the same board, the enchantment moved one seat: their
/// Doubling Season doubles the token my removal hands them.
///
/// The mirror of the test above and the half that has to fail before the
/// fix, because a branch that consults no replacement at all passes the
/// first one for the wrong reason.
#[test]
fn their_season_doubles_the_shapeshifter_my_removal_hands_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine =
        a_table_with_a_season_on_one_side(89, 1, &[plains(), plains(), plains()], crib_swap());
    aim_at_their_elf(&mut engine, crib_swap(), false);

    assert_eq!(
        tokens_controlled(&engine, p1),
        2,
        "the tokens are created under their control, which is exactly what \
         their own Doubling Season replaces"
    );
    assert_eq!(
        tokens_controlled(&engine, p0),
        0,
        "and my side of the table gained nothing from their enchantment"
    );
}

/// Rite of Replication ("Create a token that's a copy of target creature")
/// under my own Doubling Season.
///
/// A token copy is a token, so the same rule applies to it — and this is
/// the direction the previous pair cannot reach: the copy is created under
/// *my* control however far away the creature it copies is, so my
/// enchantment doubles it and their creature is untouched.
#[test]
fn my_season_doubles_the_copy_i_make_of_their_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_with_a_season_on_one_side(
        90,
        0,
        &[island(), island(), island(), island()],
        rite_of_replication(),
    );
    let victim = aim_at_their_elf(&mut engine, rite_of_replication(), false);

    assert_eq!(
        cardless_permanents(&engine, p0),
        2,
        "one token copy created twice, under my control and my own \
         Doubling Season"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature that was copied is still theirs and still there"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "and copying their creature put nothing on their side of the table"
    );
    assert_eq!(
        engine
            .state()
            .object(victim)
            .map(|o| o.controller)
            .expect("the original is still an object"),
        p1,
        "copying a permanent does not take it"
    );
}

/// The same cast with the Doubling Season across the table: one copy.
///
/// Their enchantment reads "under **your** control", and the copy is
/// created under mine, so it has nothing to say about a spell of mine that
/// happens to point at a creature of theirs. Without this half, a branch
/// that doubled unconditionally would look right.
#[test]
fn their_season_does_not_double_the_copy_i_make_of_their_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_with_a_season_on_one_side(
        91,
        1,
        &[island(), island(), island(), island()],
        rite_of_replication(),
    );
    aim_at_their_elf(&mut engine, rite_of_replication(), false);

    assert_eq!(
        cardless_permanents(&engine, p0),
        1,
        "the copy is created under my control, and their Doubling Season \
         replaces nothing that happens on my side of the table"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "and their own board gained nothing from copying their creature"
    );
}

/// The same Rite of Replication, kicked, under my own Doubling Season: ten
/// copies.
///
/// Kicker turns "create a token that's a copy" into five of them, and the
/// replacement multiplies the total rather than the printed one, so this is
/// the arithmetic the doubled branch has to get right and the first test to
/// send a kicked spell through it at all. Nine Islands is exactly
/// {7}{U}{U}, which is also what says the wizard charged for the kicker.
#[test]
fn a_kicked_rite_under_my_season_makes_ten_copies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_with_a_season_on_one_side(
        92,
        0,
        &[
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
        ],
        rite_of_replication(),
    );
    aim_at_their_elf(&mut engine, rite_of_replication(), true);

    assert_eq!(
        cardless_permanents(&engine, p0),
        10,
        "five copies from the kicker, doubled by my own Doubling Season"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "and none of them arrived on the side of the table the original is on"
    );
}

/// The copy Rite of Replication makes is a copy of the creature's **rules
/// text** as well as its characteristics (CR 707.2).
///
/// The proof is a colour the rest of the board cannot make: seat 0 has four
/// Islands and nothing green, so a `{G}` in its pool came out of the copy of
/// their Llanowar Elves or out of nowhere. The bystanders are those Islands,
/// which were offering their own mana all along, and the Elf across the
/// table, which is still theirs and still untapped afterwards.
#[test]
fn a_token_copy_carries_the_rules_text_of_the_creature_it_copies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(93, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[rite_of_replication()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let victim = aim_at_their_elf(&mut engine, rite_of_replication(), false);
    let copy = the_copy_on(&engine, p0);

    // CR 302.6: it arrived this turn, so its `{T}` is not offered yet. The
    // seat is named because a list belonging to the other one would be
    // silent about the copy for a reason that has nothing to do with the
    // claim.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the resolved spell hands priority back");
    };
    assert_eq!(player, p0, "to the seat that cast it");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == copy),
        "a creature that entered this turn cannot tap"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase grants priority");
    };
    let index = legal
        .abilities
        .iter()
        .find_map(|(id, index)| (*id == copy).then_some(*index))
        .expect("the copy offers the Elf's mana ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: copy,
                ability_index: index,
            },
        )
        .expect("an offered ability is activatable");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(baylee_core::mana::ManaColor::Green),
        1,
        "four Islands make no green — the copy did"
    );
    assert!(
        engine
            .state()
            .object(copy)
            .is_some_and(|o| o.card.is_none()),
        "and it is a token while it does it (CR 707.10)"
    );
    let original = engine.state().object(victim).expect("their Elf");
    assert_eq!(
        original.controller, p1,
        "the creature copied is still theirs"
    );
    assert!(
        !original.status.contains(crate::object::Status::TAPPED),
        "and tapping the copy did not tap it"
    );
}

/// The one card-less permanent seat `seat` controls.
///
/// Asserting there is exactly one is the point: a test that took the first
/// of several would pass while the rest were inert.
#[track_caller]
fn the_copy_on(engine: &Engine<RegistryLookup>, seat: PlayerId) -> baylee_core::ids::ObjectId {
    let mut found: Vec<baylee_core::ids::ObjectId> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_none())
        })
        .collect();
    assert_eq!(found.len(), 1, "exactly one token copy was created");
    found.pop().expect("the copy")
}

fn esper_sentinel() -> baylee_core::ids::CardIndex {
    card_index("5def9f38-0a0b-4e8d-9f9d-29dcb46520b4")
}
fn sword_of_hearth_and_home() -> baylee_core::ids::CardIndex {
    card_index("913e6182-706a-4872-8c8a-e146b0ae0738")
}
fn brainstorm() -> baylee_core::ids::CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}

/// Seat 1 casts a noncreature spell into seat 0's Esper Sentinel, and this
/// is the number the Sentinel asks them for.
///
/// The seat must be *able* to pay or there is no question to read:
/// `PlayerMayPayOr` runs its fallback outright when the pool cannot cover
/// the tax. Four Islands is enough for Brainstorm and the largest tax
/// either half of this pair asks.
#[track_caller]
fn the_tax_the_sentinel_asks_for(seed: u64, equip: bool) -> u16 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, forest())
        .battlefield(
            0,
            &[
                esper_sentinel(),
                sword_of_hearth_and_home(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    if equip {
        let sword = on_battlefield(&engine, p0, sword_of_hearth_and_home()).expect("the sword");
        let sentinel = on_battlefield(&engine, p0, esper_sentinel()).expect("the sentinel");
        activate(&mut engine, p0, sword);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![sentinel],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            e.state()
                .object(sword)
                .is_some_and(|o| o.attached_to == Some(sentinel))
        });
    }
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, brainstorm());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        panic!("the Sentinel asks for its tax, got {:?}", engine.pending())
    };
    assert_eq!(*player, p1, "the tax is asked of the caster, not of me");
    *mana
}

/// Esper Sentinel taxes `{X}`, where X is **its own power** — so a Sword of
/// Hearth and Home on it turns a `{1}` tax into `{3}`.
///
/// The pair is the evidence, not either half. The card was written as a flat
/// `mana: 1`, which is the right answer for an unequipped 1/1 and stays the
/// right answer forever: the unequipped test below passes against the wrong
/// card and the equipped one does not, so it is the equipped number that
/// says the amount is being read off the creature at all.
#[test]
fn a_sword_on_the_sentinel_raises_the_tax_it_asks_for() {
    assert_eq!(
        the_tax_the_sentinel_asks_for(93, true),
        3,
        "a 1/1 wearing +2/+2 taxes {{3}}"
    );
}

#[test]
fn an_unequipped_sentinel_asks_for_its_printed_one() {
    assert_eq!(
        the_tax_the_sentinel_asks_for(94, false),
        1,
        "the same card with nothing on it taxes {{1}}"
    );
}

fn teferi_time_raveler() -> baylee_core::ids::CardIndex {
    card_index("ae7604bb-4818-45a3-960c-cf3d83f15964")
}

/// Whether the stack has been emptied.
fn stack_is_clear(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .is_empty()
}

/// Karn, the Great Creator's `+1` with nothing to point at.
///
/// "Up to **one** target noncreature artifact" was written as a bare
/// `TargetSpec`, which the engine reads as *exactly* one — so on a board
/// with no artifact on it the ability was not offered at all, and a walker
/// that should have ticked to 6 sat at 5. That is a loyalty the printing
/// allows and this engine refused.
///
/// The assertion with teeth is the last one. `Filter::This` inside a
/// targeted ability means *the target*, and the effect that registers it
/// falls back to the ability's own source when there is no target: an
/// unguarded "up to one" would make Karn himself an artifact creature with
/// power and toughness equal to a mana value nobody chose, and the next
/// state-based check would sweep the walker into the graveyard. Offering
/// the ability and resolving it are two different fixes, and only this
/// checks the second.
#[test]
fn karns_plus_one_ticks_up_with_nothing_to_point_at() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(95, forest())
        .battlefield(0, &[karn_the_great_creator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    assert!(
        offers_an_ability(&engine, karn),
        "an ability that may target nothing is offered with nothing on the board"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 1,
            },
        )
        .expect("the +1 may be activated with no target");
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "a choice with nothing in it is not a question: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_clear);

    let walker = engine
        .state()
        .object(karn)
        .expect("the walker is still an object");
    assert_eq!(
        walker.counters.get(baylee_cards_dsl::CounterKind::Loyalty),
        6,
        "the +1 is the whole point of activating it with no target"
    );
    assert!(
        !walker
            .characteristics()
            .types
            .intersects(baylee_core::types::TypeSet::CREATURE),
        "the animation had no target and must not have fallen back onto Karn"
    );
    assert!(
        on_battlefield(&engine, p0, karn_the_great_creator()).is_some(),
        "and Karn is still on the battlefield"
    );
}

/// Teferi, Time Raveler's `−3` with nothing to bounce.
///
/// The same sentence, and the half that costs a card: "Return up to one
/// target artifact, creature, or enchantment to its owner's hand. **Draw a
/// card.**" Read as exactly one target, an empty board made the whole
/// ability unactivatable — the draw included — which is a card a player
/// simply never got.
#[test]
fn teferis_minus_three_draws_with_nothing_to_bounce() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(96, forest())
        .battlefield(0, &[teferi_time_raveler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let teferi = on_battlefield(&engine, p0, teferi_time_raveler()).expect("teferi deployed");
    let before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: teferi,
                ability_index: 2,
            },
        )
        .expect("the -3 may be activated with nothing to return");
    pass_until(&mut engine, stack_is_clear);
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        before + 1,
        "the card is drawn whether or not anything was returned"
    );
}

fn swiftfoot_boots() -> baylee_core::ids::CardIndex {
    card_index("c8b143ad-43ec-4e0d-a440-e348daa31391")
}

/// Swiftfoot Boots ("Equipped creature has **hexproof** and haste") against
/// the opponent's Vindicate.
///
/// The Boots are Lightning Greaves' shape with the other keyword in it, and
/// the pair is why the engine keeps two bits rather than one: shroud refuses
/// everybody, hexproof refuses opponents (CR 702.11b). Both grants arrive
/// through an *attachment*, which is the part a static filter can get wrong
/// in a way the Position's own tests could never see — `AttachedToBySource`
/// reaching every creature you control reads exactly like a working Equipment
/// while one creature is wearing it. The unequipped mage beside it is the
/// bystander that says otherwise.
#[test]
fn the_boots_hide_the_creature_they_are_on_from_the_other_seat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(
            0,
            &[
                swiftfoot_boots(),
                llanowar_elves(),
                snapcaster_mage(),
                forest(),
            ],
        )
        .battlefield(1, &[ondu_cleric(), plains(), swamp(), forest()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let boots = on_battlefield(&engine, p0, swiftfoot_boots()).expect("the Boots");
    let equipped = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let bystander = on_battlefield(&engine, p0, snapcaster_mage()).expect("your mage");

    // Equip is {1} here rather than the Greaves' {0}, so the mana has to be
    // floating before the ability is offered at all.
    tap_mana_except(&mut engine, p0, boots);
    let equip = offered_ability(&engine, boots).expect("equip {1} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: boots,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![equipped],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().object(equipped).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        })
    });
    assert!(
        engine.state().object(equipped).is_some_and(|o| o
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::HASTE)),
        "the Boots grant both halves of their sentence"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    let options = target_options(&engine);
    assert!(
        !options.contains(&equipped),
        "hexproof is a refusal to the seat across the table: {options:?}"
    );
    assert!(
        options.contains(&bystander),
        "and it stops at the creature the Boots are on: {options:?}"
    );
    assert!(
        options.contains(&boots),
        "the Equipment grants to what it is attached to, never to itself: \
         {options:?}"
    );
}

/// The same Boots, the same creature, and the seat that put them there.
///
/// This is the half that tells the two keywords apart, and the one a
/// Greaves-shaped copy-paste would take away: a creature its own controller
/// could no longer target is a creature that can never be equipped again,
/// auraed or pumped, and every assertion in the test above would still pass.
#[test]
fn your_own_spell_still_reaches_the_creature_wearing_your_boots() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(98, forest())
        .battlefield(
            0,
            &[
                swiftfoot_boots(),
                llanowar_elves(),
                snapcaster_mage(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let boots = on_battlefield(&engine, p0, swiftfoot_boots()).expect("the Boots");
    let equipped = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");

    // Generic mana is spent white first, so the extra Plains and Swamp are
    // what leave Vindicate's own {1}{W}{B} payable after the equip.
    tap_mana_except(&mut engine, p0, boots);
    let equip = offered_ability(&engine, boots).expect("equip {1} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: boots,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![equipped],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().object(equipped).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        })
    });

    cast_from_hand(&mut engine, p0, vindicate());
    let options = target_options(&engine);
    assert!(
        options.contains(&equipped),
        "hexproof is not shroud: the seat that granted it may still point \
         at the creature (CR 702.11b): {options:?}"
    );
}

fn storm_of_saruman() -> baylee_core::ids::CardIndex {
    card_index("cf5f4860-e805-46a3-9352-a2c583e33403")
}
fn reflections_of_littjara() -> baylee_core::ids::CardIndex {
    card_index("c3fdfb94-2d10-4743-864c-a59fdd57d8b7")
}

/// How many permanents `seat` controls that a player would call `card`.
///
/// By **name**, because a copy of a permanent spell becomes a token as it
/// resolves (CR 707.10) and a token carries no card at all — so counting
/// cards would answer "one Elf" at a board holding two, and the copy would
/// be invisible to exactly the tests written to see it.
/// [`cardless_permanents`] is the other half of the pair: this one says how
/// many are there, that one says how many of them are tokens.
fn permanents_of(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) -> usize {
    let printed = baylee_cards::by_index(card).map_or("", |def| def.faces[0].name);
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat
                    && engine.state().names.get(o.characteristics().name) == printed
            })
        })
        .count()
}

/// Answers every question the stack asks until it is empty again.
///
/// [`pass_until`] passes priority and nothing else, which is enough while a
/// spell answers all its questions before it is on the stack. A copy effect
/// asks *after* that — the copying trigger picks the spell it copies, and the
/// copy may be pointed somewhere new (CR 707.10c) — so a test about copies
/// has to answer whatever arrives, in whatever order the wizard asks.
///
/// A choice made *as a permanent enters* is handed back instead: it names
/// something only the test knows — which creature type an enchantment is
/// about — and answering it with "whatever was first" would quietly decide
/// the thing under test.
#[track_caller]
fn settle(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::ChooseSubtype { .. } => return,
            Pending::Priority { player, .. } => {
                if stack_is_empty(engine) {
                    return;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            } => {
                let objects = options.into_iter().take(min as usize).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            other => panic!("unexpected while settling the stack: {other:?}"),
        }
    }
    panic!("the stack never emptied");
}

/// Storm of Saruman ("Whenever you cast your **second** spell each turn,
/// copy it") and two creature spells in one turn.
///
/// The count is the card, and it is read off a per-turn counter rather than
/// off the stack — so the two halves have to be asked in one game: the first
/// spell resolves alone, and the second arrives with a copy beside it. A
/// trigger that fired on every cast would pass the second assertion and fail
/// the first, which is why the board is measured between the two spells and
/// not only at the end.
#[test]
fn storm_of_saruman_copies_your_second_spell_and_not_your_first() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[storm_of_saruman(), forest(), forest()])
        .hand(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        1,
        "the first spell of the turn is nobody's second"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        3,
        "the second spell arrives with a copy of itself beside it"
    );
    assert_eq!(
        permanents_of(&engine, p1, llanowar_elves()),
        0,
        "the copy arrives under the caster's control, not across the table"
    );
}

/// Reflections of Littjara ("Whenever you cast a spell of the chosen type,
/// copy that spell") with Storm of Saruman beside it, and one Elf cast into
/// both of them.
///
/// This is the rule that a copy is **put** on the stack rather than cast
/// (CR 707.10), and it is unreadable with one copy effect on the board: a
/// single enchantment copying its own copy would be caught only by whatever
/// stops a loop. Two of them make the arithmetic say it out loud. The Elf is
/// the turn's second spell, so exactly two triggers see it and exactly two
/// tokens arrive — while a copy that counted as a cast would be a spell of
/// the chosen type as well, and Reflections would answer its own answer.
#[test]
fn a_copy_is_put_on_the_stack_and_never_cast_again() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(102, forest())
        .battlefield(
            0,
            &[
                storm_of_saruman(),
                island(),
                island(),
                island(),
                island(),
                island(),
                forest(),
            ],
        )
        .hand(0, &[reflections_of_littjara(), llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The enchantment is the turn's first spell, so nothing copies it. Its
    // own question is asked as it enters, which is on resolution.
    cast_from_hand(&mut engine, p0, reflections_of_littjara());
    settle(&mut engine);
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!(
            "the enchantment names a creature type as it enters: {:?}",
            engine.pending()
        )
    };
    let elf = baylee_core::generated::subtypes::creature::ELF;
    assert!(
        options.contains(&elf),
        "Elf is a creature type: {options:?}"
    );
    engine
        .apply(player, PlayerAction::ChooseSubtype(elf))
        .unwrap();
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        0,
        "one spell so far, and it copied nothing"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        3,
        "the Elf, one copy from each enchantment — and none from the \
         copies, which were never cast"
    );
    assert_eq!(
        permanents_of(&engine, p1, llanowar_elves()),
        0,
        "none of it reached the other side of the table"
    );
}

fn helm_of_the_host() -> baylee_core::ids::CardIndex {
    card_index("83b43aba-bf9c-4da2-967d-9daa632e97d2")
}
/// A legendary creature with a body and nothing that fires on its own: its
/// enter trigger cannot go off from a battlefield the harness laid out, and
/// its tap ability is only ever offered.
fn loran_of_the_third_path() -> baylee_core::ids::CardIndex {
    card_index("b3d81980-76f2-44e2-b1c9-01e30c726312")
}

/// Equips the Helm to Loran and walks to the beginning of combat, where its
/// trigger is waiting.
#[track_caller]
fn a_helm_on_a_legend(seed: u64, season_for: Option<usize>) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut mine = vec![
        helm_of_the_host(),
        loran_of_the_third_path(),
        plains(),
        plains(),
        plains(),
        plains(),
        plains(),
    ];
    let mut theirs = vec![ondu_cleric()];
    match season_for {
        Some(0) => mine.push(doubling_season()),
        Some(_) => theirs.push(doubling_season()),
        None => {}
    }
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &mine)
        .battlefield(1, &theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let helm = on_battlefield(&engine, p0, helm_of_the_host()).expect("the Helm");
    let legend = on_battlefield(&engine, p0, loran_of_the_third_path()).expect("the legend");
    tap_mana_except(&mut engine, p0, helm);
    let equip = offered_ability(&engine, helm).expect("equip {5} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: helm,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(helm)
            .is_some_and(|o| o.attached_to == Some(legend))
    });
    pass_until(&mut engine, |e| cardless_permanents(e, p0) > 0);
    engine
}

/// Helm of the Host ("At the beginning of combat on your turn, create a
/// token that's a copy of equipped creature, except the token isn't
/// legendary") on a legendary creature, under its controller's Doubling
/// Season.
///
/// Two rules meet on one trigger and each would hide the other. The copy is
/// a **token**, so Doubling Season doubles it — and the token is **not
/// legendary**, so the state-based action that keeps one of each legend
/// (CR 704.5j) takes neither of them, nor the original. Drop the supertype
/// mod and this board collapses to a single permanent with the player asked
/// which one to keep; drop the token-ness and the Season has nothing to
/// double.
#[test]
fn a_helm_on_a_legend_makes_two_copies_the_legend_rule_lets_stand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let engine = a_helm_on_a_legend(103, Some(0));

    assert_eq!(
        cardless_permanents(&engine, p0),
        2,
        "one token from the Helm, doubled by the Season"
    );
    assert!(
        on_battlefield(&engine, p0, loran_of_the_third_path()).is_some(),
        "and the legend the Helm is on is still standing"
    );
    for id in engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
    {
        let Some(obj) = engine.state().object(*id) else {
            continue;
        };
        if obj.controller == p0 && obj.card.is_none() {
            let c = obj.characteristics();
            assert!(
                !c.supertypes
                    .contains(baylee_core::types::SupertypeSet::LEGENDARY),
                "the printing says the token isn't legendary"
            );
            assert!(
                c.keywords.contains(baylee_cards_dsl::KeywordSet::HASTE),
                "and that it gains haste"
            );
        }
    }
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "the tokens are the Helm controller's"
    );
}

/// The same Helm, with the Doubling Season across the table.
///
/// A doubling that read "a token is created" rather than "*you* create a
/// token" (CR 614.12) would double this too, and the test above could not
/// tell the difference: two tokens is two tokens whichever enchantment
/// caused them.
#[test]
fn their_doubling_season_does_not_double_the_helms_token() {
    let p0 = PlayerId::new(0);
    let engine = a_helm_on_a_legend(104, Some(1));
    assert_eq!(
        cardless_permanents(&engine, p0),
        1,
        "their Season doubles their tokens, and this one is mine"
    );
}

fn emeritus_of_woe() -> baylee_core::ids::CardIndex {
    card_index("93056597-b964-421f-be2f-e92abef1c2a4")
}

/// How many *abilities* are waiting on the stack.
///
/// A trigger is an object in the stack zone like a spell is, so counting the
/// zone answers the wrong question: the test below wants to know whether the
/// Sentinel fired a second time while two spells stand there unresolved.
fn abilities_on_the_stack(engine: &Engine<RegistryLookup>) -> usize {
    let state = engine.state();
    state
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|o| o.kind == crate::object::ObjectKind::AbilityOnStack)
        })
        .count()
}

/// How many *spells* `seat` has standing on the stack.
fn spells_on_the_stack(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    let state = engine.state();
    state
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|o| o.kind == crate::object::ObjectKind::Spell && o.controller == seat)
        })
        .count()
}

/// Advances until `want` holds, answering a resolving trigger's target
/// choice on the way — [`settle`] with a stop condition of its own, because
/// these two tests want to read the stack *while* something is still on it.
#[track_caller]
fn advance_until(
    engine: &mut Engine<RegistryLookup>,
    want: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..200 {
        if want(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            // One arm on purpose: both prompts are answered by
            // `ChooseObjects` over a list of object ids, and taking `min` of
            // them is the same shrug in both cases.
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            }
            | Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                let objects = options.into_iter().take(min as usize).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            other => panic!("unexpected while advancing: {other:?}"),
        }
    }
    panic!("the game never reached what the test was waiting for");
}

/// Emeritus of Woe's prepared cast into an opponent's Esper Sentinel
/// ("whenever an opponent casts their **first** noncreature spell each turn,
/// draw a card unless that player pays {X}").
///
/// The card says "you may **cast** a copy of its spell", so a prepared cast
/// is a cast and the turn has to count it. It did not: `start_prepared_cast`
/// journalled `SpellCast` and left `per_turn.noncreature_spells` alone, and
/// the two halves of that are what this test asks in one game. The Sentinel
/// did not tax the tutor — and then taxed the Brainstorm after it, which is
/// the turn's *second* noncreature spell and should have been free. A fix
/// that only stopped the second wrong would pass the last assertion here and
/// fail the first.
#[test]
fn the_sentinel_taxes_a_prepared_cast_and_leaves_the_spell_after_it_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .battlefield(0, &[emeritus_of_woe(), swamp(), swamp(), swamp(), island()])
        .hand(0, &[brainstorm()])
        .battlefield(1, &[esper_sentinel(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let emeritus = on_battlefield(&engine, p0, emeritus_of_woe()).expect("the Warlock");
    let island = on_battlefield(&engine, p0, island()).expect("the Island");
    // The blue stays untapped on purpose: generic mana is paid in colour
    // order, so an Island in the pool would pay the tutor's {1} and leave
    // Brainstorm uncastable.
    tap_mana_except(&mut engine, p0, island);

    let cast = offered_ability(&engine, emeritus).expect("the prepared cast is offered");
    assert_eq!(
        cast,
        crate::choice::PREPARED_CAST,
        "the prepared cast is the synthetic index, not a printed ability"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: emeritus,
                ability_index: cast,
            },
        )
        .unwrap();

    advance_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the loop above stopped on the tax")
    };
    assert_eq!(player, p0, "the tax is asked of whoever cast the spell");
    assert_eq!(mana, 1, "an unequipped Sentinel taxes {{1}}");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    // The tutor is still on the stack; Brainstorm is an instant and joins it.
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the trigger resolved and the turn's player has priority again: {:?}",
        engine.pending()
    );
    cast_from_hand(&mut engine, p0, brainstorm());
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        2,
        "the tutor and the Brainstorm are both waiting"
    );
    assert_eq!(
        abilities_on_the_stack(&engine),
        0,
        "the Sentinel already had its first noncreature spell this turn"
    );
}

/// The same prepared cast, counted by Storm of Saruman instead ("whenever
/// you cast your **second** spell each turn, copy it").
///
/// This is the other counter the prepared cast walked past —
/// `per_turn.spells_cast` — and it needs its own game, because a spell can
/// be a creature spell and still be somebody's second. An Elf from hand is
/// the first, the tutor is the second, and the copy beside it is the whole
/// assertion: with the counter unbumped the trigger looks at a count of one
/// and never fires.
#[test]
fn a_prepared_cast_is_the_second_spell_saruman_copies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(106, forest())
        .battlefield(
            0,
            &[
                storm_of_saruman(),
                emeritus_of_woe(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The turn's first spell. `cast_from_hand` taps everything, so the two
    // Swamps are floating for the prepared cast that follows — a pool only
    // empties at the end of a step, and this all happens in one main phase.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        1,
        "the first spell of the turn is nobody's second"
    );

    let emeritus = on_battlefield(&engine, p0, emeritus_of_woe()).expect("the Warlock");
    let cast = offered_ability(&engine, emeritus).expect("the prepared cast is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: emeritus,
                ability_index: cast,
            },
        )
        .unwrap();
    advance_until(&mut engine, |e| spells_on_the_stack(e, p0) == 2);
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        2,
        "the tutor is the turn's second spell, so a copy stands beside it"
    );
    assert_eq!(
        spells_on_the_stack(&engine, p1),
        0,
        "the copy is the caster's, not the other seat's"
    );
}

/// The same prepared cast, all the way through: the linked spell resolves,
/// the tutor actually searches, and the Warlock is no longer prepared.
///
/// The two tests above both stop with the tutor still on the stack, which is
/// exactly where two further defects were hiding. A fresh object starts in
/// its owner's library and `Zones::insert` does not say otherwise, so the
/// spell resolved *out of the library*: its id stayed on the stack and in
/// `stack_projectable`, which the very next `refresh_characteristics` reports
/// as drift. And `resolve_stack_top` reads a spell's effects off
/// `GameObject::card`, which `new_bare` leaves `None`, so the tutor resolved
/// to nothing at all — both seats passed and no library was ever searched.
#[test]
fn a_prepared_cast_resolves_and_unprepares_the_warlock() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .battlefield(0, &[emeritus_of_woe(), swamp(), swamp(), swamp()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let emeritus = on_battlefield(&engine, p0, emeritus_of_woe()).expect("the Warlock");
    tap_mana_except(&mut engine, p0, emeritus);
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .len();

    let cast = offered_ability(&engine, emeritus).expect("the prepared cast is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: emeritus,
                ability_index: cast,
            },
        )
        .unwrap();

    // Both seats pass, the tutor resolves, and the search it asks for is
    // answered on the way through by `advance_until`.
    advance_until(&mut engine, |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len()
            > hand_before
    });

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
            .len(),
        library_before - 1,
        "the tutored card left the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .is_empty(),
        "the spell left the stack rather than being destroyed underneath it"
    );
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        0,
        "and nothing of it is still standing there"
    );

    let riders = &engine.state().object(emeritus).expect("the Warlock").riders;
    assert!(
        !riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Prepared)),
        "casting the prepared spell is what unprepares the card that held it"
    );
    assert!(
        offered_ability(&engine, emeritus).is_none(),
        "so it is not offered a second time"
    );
    // CR 704.5e: a copy of a spell in a zone other than the stack ceases to
    // exist. The Warlock's spell is a copy of a card nobody put in a deck,
    // so a graveyard is the one place it must never reach — a Demonic Tutor
    // sitting there is a card that could be flashed back, delved away or
    // counted by a threshold, and it was never in the game.
    assert_eq!(
        in_graveyard(&engine, p0, demonic_tutor()),
        0,
        "the copy ceased to exist rather than becoming a card in a graveyard"
    );
}

/// The other half of CR 704.5e, at the other place a spell copy is made:
/// `Effect::CopyTargetSpell`.
///
/// Storm of Saruman copies the turn's second spell, so a Brainstorm cast
/// second resolves twice — and exactly *one* Brainstorm may be in the
/// graveyard afterwards. The copy going there instead of ceasing to exist
/// is the same defect the prepared cast had, at the site the prepared cast
/// was modelled on.
#[test]
fn a_copied_spell_leaves_one_card_in_the_graveyard_not_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, island())
        .battlefield(
            0,
            &[storm_of_saruman(), forest(), island(), island(), island()],
        )
        .hand(0, &[llanowar_elves(), brainstorm()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The turn's first spell, so the Brainstorm after it is the second.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    cast_from_hand(&mut engine, p0, brainstorm());
    // The copy is made by a trigger, so it is one resolution away.
    advance_until(&mut engine, |e| spells_on_the_stack(e, p0) == 2);
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        2,
        "the copy stands beside the spell it was made from"
    );

    // Both resolve, each asking which two cards go back on the library.
    advance_until(&mut engine, |e| spells_on_the_stack(e, p0) == 0);
    assert_eq!(
        in_graveyard(&engine, p0, brainstorm()),
        1,
        "the copy ceased to exist; only the card that was cast is a card"
    );
}

/// Demonic Tutor, the spell Emeritus of Woe prepares. It is in nobody's
/// deck: the ability links it by `CardIndex` out of the registry.
fn demonic_tutor() -> baylee_core::ids::CardIndex {
    card_index("82004860-e589-4e38-8d61-8c0210e4ea39")
}

/// How many cards of one printing are in `seat`'s graveyard.
fn in_graveyard(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(seat))
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// CR 707.10: a copy of a **permanent** spell becomes a token as it
/// resolves — the sentence Storm of Saruman prints in its own reminder
/// text.
///
/// Two Elves cast in one turn puts three on the board, and until now all
/// three were cards: the copy resolved into a permanent still carrying the
/// card it was copied from, so `Filter::IsToken` said no, and a copy that
/// died left a second Llanowar Elves in the graveyard for anything reading
/// that zone to find. The board count is asserted beside the token count
/// on purpose — a copy that simply failed to arrive would pass the second
/// assertion on its own.
#[test]
fn the_copy_of_a_creature_spell_arrives_as_a_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(109, forest())
        .battlefield(0, &[storm_of_saruman(), forest(), forest()])
        .hand(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        cardless_permanents(&engine, p0),
        0,
        "the first spell of the turn is copied by nothing"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        3,
        "the second spell arrives with a copy of itself beside it"
    );
    assert_eq!(
        cardless_permanents(&engine, p0),
        1,
        "and exactly one of the three is the token the copy became"
    );
    assert_eq!(
        in_graveyard(&engine, p0, llanowar_elves()),
        0,
        "nothing was a card that should not have been one"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "the token is the caster's, not the other seat's"
    );
}
