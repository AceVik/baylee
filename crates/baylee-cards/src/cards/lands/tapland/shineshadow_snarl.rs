//! Shineshadow Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Plains or Swamp card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {B}.
//! Set: SOC #403 — Secrets of Strixhaven Commander | Scryfall ID: 7a198106-4d31-4e10-884a-bdf2316f8088 | Oracle ID: c9fc13d6-bd10-47bc-b2b6-7f67a1f3371e
// PARTIAL — {T}: Add {W} or {B} is implemented; the entry clause is not, so
// this land always enters untapped.
// NOT SUPPORTED: "As this land enters, you may reveal a Plains or Swamp card
// from your hand. If you don't, this land enters tapped." — EnterModifier has
// Tapped, TappedUnless(filter), TappedUnlessCount { .. } and TappedOrPayLife,
// and every one of them that reads a condition reads the battlefield:
// TappedUnless asks about a permanent and is not a choice, so nothing can
// look at a card in a hidden zone or make the arrival depend on that reveal.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHINESHADOW_SNARL,
    oracle_id = "c9fc13d6-bd10-47bc-b2b6-7f67a1f3371e",
    scryfall_id = "7a198106-4d31-4e10-884a-bdf2316f8088",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(name = "Shineshadow Snarl", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the printed entry clause is not expressible — no EnterModifier reads a card in a hidden zone or offers the player a choice (the nearest, TappedUnless(filter), is mandatory and asks about a permanent on the battlefield), so this land enters untapped instead of entering tapped unless a Plains or Swamp card is revealed from hand"
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::White,
        ManaColor::Black,
    ])])],
);
