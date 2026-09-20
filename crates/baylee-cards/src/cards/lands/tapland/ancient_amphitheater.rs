//! Ancient Amphitheater — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Giant card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {R} or {W}.
//! Set: CM2 #232 — Commander Anthology Volume II | Scryfall ID: c68137c5-c2f9-4b0d-ac4b-12c519854166 | Oracle ID: 7211221d-d4d8-4bbe-9d2a-b82e005bfe8a
// PARTIAL — the mana ability is built; the entry clause stays unbuilt.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANCIENT_AMPHITHEATER,
    oracle_id = "7211221d-d4d8-4bbe-9d2a-b82e005bfe8a",
    scryfall_id = "c68137c5-c2f9-4b0d-ac4b-12c519854166",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(name = "Ancient Amphitheater", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the reveal-a-Giant entry clause: no EnterModifier reads a card in hand",
    ),
    abilities = &[
        // NOT SUPPORTED: "As this land enters, you may reveal a Giant card
        // from your hand. If you don't, this land enters tapped." No
        // EnterModifier reads a card in hand (TappedUnless asks about
        // permanents on the battlefield), so the land always enters
        // untapped.
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Red, ManaColor::White,])]),
    ],
);
