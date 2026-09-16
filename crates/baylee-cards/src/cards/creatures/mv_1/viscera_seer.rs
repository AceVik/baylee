//! Viscera Seer — {B} — Creature — Vampire Wizard
//! Oracle: Sacrifice a creature: Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
//! Set: SOC #229 — Secrets of Strixhaven Commander | Scryfall ID: f511830b-1c1f-4d30-aa5d-4314726d142e | Oracle ID: f82a4e85-526d-4456-b700-7760043a31be
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VISCERA_SEER,
    oracle_id = "f82a4e85-526d-4456-b700-7760043a31be",
    scryfall_id = "f511830b-1c1f-4d30-aa5d-4314726d142e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Viscera Seer",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
