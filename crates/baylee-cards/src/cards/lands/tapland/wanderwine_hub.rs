//! Wanderwine Hub — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Merfolk card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Set: LRW #280 — Lorwyn | Scryfall ID: ccec69de-7203-4810-a8ec-8748705ee3a2 | Oracle ID: c3b46bd6-b3ef-452d-a916-995c44f1da07
// IMPLEMENTED — {T}: Add {W} or {U}; the enters-tapped clause is NOT
// SUPPORTED and the card says so below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WANDERWINE_HUB,
    oracle_id = "c3b46bd6-b3ef-452d-a916-995c44f1da07",
    scryfall_id = "ccec69de-7203-4810-a8ec-8748705ee3a2",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Wanderwine Hub", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("enters tapped unless you reveal a Merfolk card from your hand"),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::White,
        ManaColor::Blue,
    ])])],
);

// NOT SUPPORTED: "As this land enters, you may reveal a Merfolk card from
// your hand. If you don't, this land enters tapped." — no `EnterModifier`
// asks its controller to reveal a card from hand: every `TappedUnless*`
// variant reads permanents on the battlefield, and the DSL has no reveal
// effect at all. The land therefore enters untapped, and the mana ability
// over it is the whole card. The land prints no basic land type, so its
// mana ability is written out rather than left to the intrinsic shortcut.
