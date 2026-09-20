//! Foreboding Ruins — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Swamp or Mountain card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Set: MSC #244 — Marvel Super Heroes Commander | Scryfall ID: 76fdf6be-e13b-49ee-9d50-8625edb49951 | Oracle ID: 5c87e2fa-77f1-4978-b25f-f14d227301d1
// PARTIAL — {T}: Add {B} or {R} is written; the as-it-enters reveal has no
// EnterModifier variant, so the land always arrives untapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FOREBODING_RUINS,
    oracle_id = "5c87e2fa-77f1-4978-b25f-f14d227301d1",
    scryfall_id = "76fdf6be-e13b-49ee-9d50-8625edb49951",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(name = "Foreboding Ruins", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the as-it-enters clause — you may reveal a Swamp or Mountain card \
         from your hand, and if you don't this land enters tapped — names no \
         EnterModifier variant (nothing in the entry vocabulary reads a card \
         out of a hand)"
    ),
    abilities = &[
        // NOT SUPPORTED: As this land enters, you may reveal a Swamp or
        // Mountain card from your hand. If you don't, this land enters
        // tapped. — EnterModifier has TappedUnless / TappedUnlessCount /
        // TappedOrPayLife and nothing that asks about a card in hand.
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]),
    ],
);
