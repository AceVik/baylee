//! Shineshadow Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Plains or Swamp card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {B}.
//! Set: SOC #403 — Secrets of Strixhaven Commander | Scryfall ID: 7a198106-4d31-4e10-884a-bdf2316f8088 | Oracle ID: c9fc13d6-bd10-47bc-b2b6-7f67a1f3371e
// IMPLEMENTED — reveal a Plains or Swamp card from hand or enter tapped; {W} or {B}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The basic land type, not `Filter::BASIC_LAND`: "a Plains or Swamp card" is
/// any card with the type, so a dual land or a shockland in hand
/// reveals for this one exactly as a basic does.
///
/// `ControlledByYou` would be noise — the menu is built from the
/// controller's own hand.
static REVEAL: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::PLAINS),
    Filter::HasSubtype(subtypes::land::SWAMP),
]);

card!(
    index = index::SHINESHADOW_SNARL,
    oracle_id = "c9fc13d6-bd10-47bc-b2b6-7f67a1f3371e",
    scryfall_id = "7a198106-4d31-4e10-884a-bdf2316f8088",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Shineshadow Snarl",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&REVEAL)],
    )],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::White,
        ManaColor::Black,
    ])])],
);
