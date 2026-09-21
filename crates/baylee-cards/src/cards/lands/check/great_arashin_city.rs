//! Great Arashin City — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Forest or a Plains.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token.
//! Set: TDM #257 — Tarkir: Dragonstorm | Scryfall ID: ecba23b6-9f3a-431e-bc22-f1fb04d27b68 | Oracle ID: f40f374b-acaf-459d-9ccd-b0b22d1a3f28
// PARTIAL — checkland entry condition and tap for {B}; activation cost requiring
// exiling a creature card from your graveyard is not expressible in CostPart.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FOREST_OR_PLAINS: Filter = Filter::And(&[
    Filter::LAND,
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::land::FOREST),
        Filter::HasSubtype(subtypes::land::PLAINS),
    ]),
]);

card!(
    index = index::GREAT_ARASHIN_CITY,
    oracle_id = "f40f374b-acaf-459d-9ccd-b0b22d1a3f28",
    scryfall_id = "ecba23b6-9f3a-431e-bc22-f1fb04d27b68",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "exiling a creature card from your graveyard as an activation cost is not expressible (CostPart has no ExileFromGraveyard variant)",
    ),
    faces = &[face!(
        name = "Great Arashin City",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&FOREST_OR_PLAINS)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        // NOT SUPPORTED: {1}{B}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token.
    ],
);
