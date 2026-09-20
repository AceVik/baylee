//! Furycalm Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Mountain or Plains card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {R} or {W}.
//! Set: MSC #247 — Marvel Super Heroes Commander | Scryfall ID: afc12892-4978-4442-ace5-c4dea5c0ee00 | Oracle ID: 651dea9c-2375-4e44-8e65-ba8e40f0c0ef
// PARTIAL — {T}: Add {R} or {W} is a plain mana ability; the as-it-enters
// reveal is not expressible (see the NOT SUPPORTED note below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FURYCALM_SNARL,
    oracle_id = "651dea9c-2375-4e44-8e65-ba8e40f0c0ef",
    scryfall_id = "afc12892-4978-4442-ace5-c4dea5c0ee00",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    coverage = Coverage::Partial(
        "the optional as-it-enters reveal of a Mountain or Plains card from hand"
    ),
    faces = &[face!(name = "Furycalm Snarl", types = TypeSet::LAND,),],
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Red,
        ManaColor::White
    ])])],
);

// NOT SUPPORTED: "As this land enters, you may reveal a Mountain or Plains
// card from your hand. If you don't, this land enters tapped." — no
// EnterModifier asks about a card in hand: TappedUnless/TappedUnlessCount
// read the battlefield, and the reveal is a choice made from hand as the
// land arrives. With no modifier written, the land enters untapped.
