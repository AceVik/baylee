//! Tectonic Edge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Destroy target nonbasic land. Activate only if an opponent controls four or more lands.
//! Set: C14 #313 — Commander 2014 | Scryfall ID: 94b9d7af-3e6f-4227-bb7d-a17d6f250535 | Oracle ID: 4927150d-7ff6-4232-b20e-d2ea245ac710
// IMPLEMENTED — {T} for {C}, and the destruction behind the gate the card
// prints: it had been shipping ungated, which is a Wasteland.

use baylee_cards_dsl::prelude::*;

/// Scoped to the other side of the table even though
/// `Condition::OpponentControlCount` already walks one opponent at a time:
/// the filter is what the card's own sentence says, and `xtask validate`
/// holds a card printing "an opponent controls" to a filter that says so.
static OPPONENT_LAND: Filter = Filter::And(&[Filter::LAND, Filter::ControlledByOpponent]);

card!(
    index = index::TECTONIC_EDGE,
    oracle_id = "4927150d-7ff6-4232-b20e-d2ea245ac710",
    scryfall_id = "94b9d7af-3e6f-4227-bb7d-a17d6f250535",
    faces = &[face!(name = "Tectonic Edge", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // Four lands an **opponent** controls, not four on the table: on a
        // multiplayer board the sentence is true the moment one seat
        // reaches four, and false while three seats hold three each.
        activated!(
            cost!("{1}", TapSelf, SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(&Filter::NONBASIC_LAND))],
            target = Some(TargetSpec::Object(&Filter::NONBASIC_LAND)),
            condition = Some(Condition::OpponentControlCount(&OPPONENT_LAND, 4)),
        ),
    ],
);
