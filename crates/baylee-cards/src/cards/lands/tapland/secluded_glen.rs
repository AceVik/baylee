//! Secluded Glen — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Faerie card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {U} or {B}.
//! Set: LRW #271 — Lorwyn | Scryfall ID: 9e4afa65-7933-4a64-b50f-a9a9f832b112 | Oracle ID: 09f52275-99e6-45e0-b2db-cafe26d5fb91
// PARTIAL — {T}: Add {U} or {B}; the as-enters clause is not expressible.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SECLUDED_GLEN,
    oracle_id = "09f52275-99e6-45e0-b2db-cafe26d5fb91",
    scryfall_id = "9e4afa65-7933-4a64-b50f-a9a9f832b112",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(name = "Secluded Glen", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no EnterModifier reads a card from the controller's hand; the land enters untapped instead of asking to reveal a Faerie"
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Black,
    ])])],
);

// NOT SUPPORTED: "As this land enters, you may reveal a Faerie card from your
// hand. If you don't, this land enters tapped." — every `EnterModifier` that
// gates "enters tapped" (`TappedUnless`, `TappedUnlessCount`, …) tests a
// permanent on the battlefield, and none of them asks the controller to
// reveal a card, so the clause is dropped and the land always arrives
// untapped.
