//! Auntie's Hovel — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Goblin card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Set: LRW #267 — Lorwyn | Scryfall ID: 098685c9-cd85-4279-a3b5-b495485bba35 | Oracle ID: 245469ff-72b6-4846-8a82-a1d29f4d09bb
// PARTIAL — the mana line only: {T} for {B} or {R}. The entry clause needs a
// condition read out of a hidden zone, which no EnterModifier carries.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AUNTIE_S_HOVEL,
    oracle_id = "245469ff-72b6-4846-8a82-a1d29f4d09bb",
    scryfall_id = "098685c9-cd85-4279-a3b5-b495485bba35",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    // NOT SUPPORTED: "As this land enters, you may reveal a Goblin card from
    // your hand. If you don't, this land enters tapped." — every
    // EnterModifier that asks about a card asks about a permanent you
    // control (TappedUnless is the checkland's "unless you control a
    // Plains"), and none of them reveals a card from hand, so the land
    // enters untapped where the printing says tapped.
    faces = &[face!(name = "Auntie's Hovel", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("entry: reveal a Goblin card from hand, else tapped"),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Black,
        ManaColor::Red,
    ])])],
);
