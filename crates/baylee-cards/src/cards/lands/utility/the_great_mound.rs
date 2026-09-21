//! The Great Mound — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Create a tapped Vibranium token. (It's an artifact with indestructible and "{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.")
//! Oracle: {6}, {T}: Draw a card.
//! Set: MSC #120 — Marvel Super Heroes Commander | Scryfall ID: 11b3f96c-138e-431e-a1d6-5ebae4cb6b2f | Oracle ID: 6b78e417-44f8-4ab6-9d3c-e71704fc648e
// PARTIAL — {T} for {C} and the {6} loot are built; the token ability is
// NOT SUPPORTED (see the line beside it).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_GREAT_MOUND,
    oracle_id = "6b78e417-44f8-4ab6-9d3c-e71704fc648e",
    scryfall_id = "11b3f96c-138e-431e-a1d6-5ebae4cb6b2f",
    faces = &[face!(name = "The Great Mound", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no CreateToken variant makes a token enter tapped, no Vibranium token is defined in crates/baylee-cards/src/tokens.rs, and the token's \"This mana can't be spent to cast a nonartifact spell\" is not one of the three SpendRiders"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{3}, {T}: Create a tapped Vibranium token. (It's an
        // artifact with indestructible and \"{T}: Add {C}. This mana can't be
        // spent to cast a nonartifact spell.\")" — every `CreateToken` variant
        // names a token and none of them carries "tapped", the token itself
        // would have to live in `crate::tokens` (a card file may not define
        // one), and `ManaRestriction`/`SpendRider` has no arm for a mana that
        // can't pay for a nonartifact spell.
        activated!(cost!("{6}", TapSelf), &[Effect::draw(1)]),
    ],
);
