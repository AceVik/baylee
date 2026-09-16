//! Grist, the Hunger Tide — {1}{B}{G} — Legendary Planeswalker — Grist
//! Oracle: As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.
//! Oracle: +1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way, put a loyalty counter on Grist and repeat this process.
//! Oracle: −2: You may sacrifice a creature. When you do, destroy target creature or planeswalker.
//! Oracle: −5: Each opponent loses life equal to the number of creature cards in your graveyard.
//! Set: DSC #220 — Duskmourn: House of Horror Commander | Scryfall ID: 1925dc45-4dee-4772-aa16-3b4ca54be6c7 | Oracle ID: 0efb0d7e-dea0-4817-a243-15066e9ef333
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRIST_THE_HUNGER_TIDE,
    oracle_id = "0efb0d7e-dea0-4817-a243-15066e9ef333",
    scryfall_id = "1925dc45-4dee-4772-aa16-3b4ca54be6c7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Grist, the Hunger Tide",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::GRIST],
        loyalty = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
