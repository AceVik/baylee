//! Sanctum of Eternity — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Return target commander you own from the battlefield to your hand. Activate only during your turn.
//! Set: C19 #59 — Commander 2019 | Scryfall ID: a680cc6d-9c31-4f5d-808f-dace92bf4346 | Oracle ID: c7d9ff27-f1fc-42e4-a47b-d2e6d68e4035
// PARTIAL — {T}: Add {C}; the second ability has no vocabulary and is
// dropped (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SANCTUM_OF_ETERNITY,
    oracle_id = "c7d9ff27-f1fc-42e4-a47b-d2e6d68e4035",
    scryfall_id = "a680cc6d-9c31-4f5d-808f-dace92bf4346",
    faces = &[face!(name = "Sanctum of Eternity", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the second ability needs a commander predicate in Filter and a \
         \"only during your turn\" activation timing, neither of which exists"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
    // NOT SUPPORTED: "{2}, {T}: Return target commander you own from the
    // battlefield to your hand. Activate only during your turn." — there is
    // no Filter for "is a commander" (Filter::SharesSubtypeWithCommander
    // asks a different question, about a creature sharing a subtype with
    // one), and ActivationTiming offers only InstantSpeed and SorcerySpeed,
    // where this clause is a turn restriction and not a phase one.
);
