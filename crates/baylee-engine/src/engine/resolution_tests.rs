//! What is left of a spell once it has finished resolving (CR 608.2).
//!
//! CR 608.2m lets an instant or sorcery that leaves the stack part-way
//! through its own resolution finish resolving; CR 608.2n then puts *the
//! spell* into its owner's graveyard as the final part. Those two sentences
//! are one rule with a hole in the middle: a card its own effect has already
//! moved is a new object somewhere else, and there is no spell left on the
//! stack for the second sentence to put anywhere.
//!
//! `finalize_spell` asked neither question and moved whatever id it was
//! handed. Three cards print "Exile <this card>" — Spirit Water Revival,
//! Teferi's Protection and Temporal Mastery — and in all three
//! `Effect::ExileSource` exiled the card and the finalisation fetched it
//! straight back out into the graveyard, one step later, with nothing said.
//! Spirit Water Revival is `Coverage::Implemented`, has a whole test module
//! of its own (`waterbend_tests`), and was wrong the entire time: no test
//! had ever asked where the card ended up.

#[allow(clippy::wildcard_imports)] // the shared duel plumbing
use super::testkit::*;

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

fn spirit_water_revival() -> CardIndex {
    card_index("68979160-b5ce-4787-8a1e-1f40e614c3b0")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

/// A spell that exiles itself as it resolves is not then fetched back.
///
/// Both halves are asserted, and either alone would pass against a
/// different mistake: the two cards drawn say the spell *resolved* rather
/// than being countered or fizzled on the way, and the zone says the
/// exile stuck. An `is_none()` on the graveyard alone would also be true
/// of a spell that never resolved at all.
#[test]
fn a_spell_that_exiles_itself_is_not_put_into_a_graveyard_afterwards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[spirit_water_revival()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, spirit_water_revival());
    // Waterbend {6} is an *optional* additional cost (CR 601.2b), and three
    // Islands could not pay it anyway. Declined, so the spell takes its
    // printed branch — "draw two cards" — and the self-exile below is the
    // one sentence that is the same either way.
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("the waterbend question is the caster's");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 2,
        "\"Draw two cards\" — the spell resolved, which is what makes the \
         zone below worth reading"
    );
    assert!(
        in_graveyard(&engine, p0, spirit_water_revival()).is_none(),
        "CR 608.2n puts *the spell* into its owner's graveyard, and the card \
         its own last sentence exiled is not one"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == spirit_water_revival()))
            }),
        "\"Exile Spirit Water Revival\" — it is where the card put itself"
    );
}

/// What a spell did to a permanent spell, it goes on doing to the permanent
/// that spell becomes (CR 400.7a). Purelace cast at Llanowar Elves on the
/// stack makes a white spell and then a white creature: the permanent is a
/// new object (CR 400.7), and the colour effect named the spell's version,
/// so it reached nothing once the Elves resolved — a green creature, from a
/// spell that had been white a moment before.
#[test]
fn a_permanent_spell_keeps_what_was_done_to_it_on_the_stack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let elves = card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3");
    let purelace = card_index("3773001a-8868-49ec-a406-298cf72359c2");
    let plains = card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99");
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let mut engine = Duel::new(3, forest)
        .battlefield(0, &[forest])
        .hand(0, &[elves])
        .battlefield(1, &[plains])
        .hand(1, &[purelace])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, elves);
    let spell = on_stack(&engine, elves).expect("the Elves are a spell");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, purelace);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("target spell or permanent, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&spell),
        "the spell on the stack is a target"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .unwrap();
    let white = baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::White]);
    pass_until(&mut engine, |e| on_stack(e, purelace).is_none());
    let colors =
        |e: &Engine<RegistryLookup>| e.state().object(spell).map(|o| o.characteristics().colors);
    assert_eq!(colors(&engine), Some(white), "a white spell");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, elves).is_some());
    assert_eq!(colors(&engine), Some(white), "and a white creature");
}
