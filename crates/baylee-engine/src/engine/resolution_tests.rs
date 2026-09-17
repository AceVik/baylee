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
