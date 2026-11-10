//! Symbiotic Deployment — {2}{G} — Enchantment
//! Oracle: Skip your draw step.
//! Oracle: {1}, Tap two untapped creatures you control: Draw a card.
//! Set: APC #88 — Apocalypse | Scryfall ID: a6e2b7e9-d52b-478e-b118-e890a81fd471 | Oracle ID: a6872d7a-b647-4baf-bd1e-06707ccbb71a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYMBIOTIC_DEPLOYMENT,
    oracle_id = "a6872d7a-b647-4baf-bd1e-06707ccbb71a",
    scryfall_id = "a6e2b7e9-d52b-478e-b118-e890a81fd471",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Symbiotic Deployment",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
