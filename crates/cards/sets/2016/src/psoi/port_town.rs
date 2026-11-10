//! Port Town — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Plains or Island card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Set: MSC #256 — Marvel Super Heroes Commander | Scryfall ID: 3f962cab-c058-41f5-b7e1-5b063f374eb0 | Oracle ID: 458d2b12-f578-4392-98d3-c3bc83f316c4
// IMPLEMENTED — reveal a Plains or Island card from hand or enter tapped; {W} or {U}.
// IMPLEMENTED — the mana ability, which chooses {W} or {U} on resolution.
// The entry clause is not: no EnterModifier says "reveal a card from your
// hand, or this enters tapped".

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Plains or Island card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::PLAINS),
    Filter::HasSubtype(subtypes::land::ISLAND),
]);

card!(
    index = index::PORT_TOWN,
    oracle_id = "458d2b12-f578-4392-98d3-c3bc83f316c4",
    scryfall_id = "3f962cab-c058-41f5-b7e1-5b063f374eb0",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Port Town",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::White,
        ManaColor::Blue,
    ])])],
);
