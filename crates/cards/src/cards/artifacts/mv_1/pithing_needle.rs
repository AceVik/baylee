//! Pithing Needle — {1} — Artifact
//! Oracle: As this artifact enters, choose a card name.
//! Oracle: Activated abilities of sources with the chosen name can't be activated unless they're mana abilities.
//! Set: 2X2 #312 — Double Masters 2022 | Scryfall ID: 776899f8-e977-42b7-8b54-6f726a349e3c | Oracle ID: a188fe7e-68de-4c7c-806c-bfe8fc7b44bf
// IMPLEMENTED — the name is chosen as it enters (EnterModifier::ChooseCardName)
// and a static locks every non-mana activated ability of every source with
// that name, on the battlefield or in a hand (Modifier::ChosenNameCantActivate).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PITHING_NEEDLE,
    oracle_id = "a188fe7e-68de-4c7c-806c-bfe8fc7b44bf",
    scryfall_id = "776899f8-e977-42b7-8b54-6f726a349e3c",
    faces = &[face!(
        name = "Pithing Needle",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::ChooseCardName],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::ChosenNameCantActivate
    )],
);
