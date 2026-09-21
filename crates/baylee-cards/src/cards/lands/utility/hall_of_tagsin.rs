//! Hall of Tagsin — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}, {T}: Create a tapped Powerstone token. (It's an artifact with "{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.")
//! Set: BRO #263 — The Brothers' War | Scryfall ID: a8007012-39c5-4247-ba77-1cfcaade37fa | Oracle ID: fd809587-773c-47ec-8676-cefef6d1e38f
// IMPLEMENTED — both mana abilities; the {4}, {T} Powerstone clause is
// NOT SUPPORTED, because no Effect makes a token enter tapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_TAGSIN,
    oracle_id = "fd809587-773c-47ec-8676-cefef6d1e38f",
    scryfall_id = "a8007012-39c5-4247-ba77-1cfcaade37fa",
    faces = &[face!(name = "Hall of Tagsin", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {4}, {T} ability is left off: no Effect creates a token that \
         enters tapped",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "{4}, {T}: Create a tapped Powerstone token."
        // Every token-making Effect — CreateToken, CreateTokenN,
        // CreateTokenForTargetController, … — creates the token untapped:
        // Effect::CreateToken has no `tapped` field, and a TokenDef carries
        // no enter modifiers (EnterModifier lives on FaceDef alone). An
        // untapped Powerstone would contradict the card's own sentence in
        // the one way a player can see, so the ability is left off rather
        // than approximated.
    ],
);
