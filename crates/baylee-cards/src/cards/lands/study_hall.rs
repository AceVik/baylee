//! Study Hall — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color. When you spend this mana to cast your commander, scry X, where X is the number of times it's been cast from the command zone this game.
//! Set: SOC #407 — Secrets of Strixhaven Commander | Scryfall ID: 93f812bc-7663-4504-b566-79e2c0a0f845 | Oracle ID: eb735501-19e7-4910-aa6a-6667fff6f4e5
// IMPLEMENTED — both mana abilities: {T} for {C}, and {1}, {T} for one mana of
// any color. The spend rider that watches where that mana goes is not
// expressible, so the card is Partial.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STUDY_HALL,
    oracle_id = "eb735501-19e7-4910-aa6a-6667fff6f4e5",
    scryfall_id = "93f812bc-7663-4504-b566-79e2c0a0f845",
    faces = &[face!(name = "Study Hall", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the second line's spend rider — scry X when this mana casts your \
         commander, X being its command-zone cast count — has no variant: \
         SpendRider::Scry carries a fixed u8, nothing counts command-zone \
         casts, and no filter can name \"your commander\"",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
    ],
);

// NOT SUPPORTED: "When you spend this mana to cast your commander, scry X, where X is the number of times it's been cast from the command zone this game." — SpendRider::Scry takes a fixed u8, the DSL has no amount for a commander's cast count, and pool mana carries no provenance to match a spend against.
