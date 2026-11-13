//! Ashnod's Altar — {3} — Artifact
//! Oracle: Sacrifice a creature: Add {C}{C}.
//! Set: CMM #368 — Commander Masters | Scryfall ID: 3c0f7157-a375-499c-92c7-d47d2e95dbad | Oracle ID: 4d18bcba-a346-445e-a182-6cc30b7e066d
// IMPLEMENTED — a sacrifice outlet that is also a mana rock: eat a creature
// you control and get {C}{C} without using the stack (CR 605.1). No tap, so
// it eats as many creatures a turn as you can feed it, and the question of
// which one is asked per activation by `engine::cost_wizard`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASHNOD_S_ALTAR,
    oracle_id = "4d18bcba-a346-445e-a182-6cc30b7e066d",
    scryfall_id = "3c0f7157-a375-499c-92c7-d47d2e95dbad",
    faces = &[face!(
        name = "Ashnod's Altar",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE)),
        &[Effect::mana(ManaColor::Colorless, 2)]
    ),],
);
