//! Crookclaw Elder — {5}{U} — Creature — Bird Wizard
//! Oracle: Flying
//! Oracle: Tap two untapped Birds you control: Draw a card.
//! Oracle: Tap two untapped Wizards you control: Target creature gains flying until end of turn.
//! Set: LGN #34 — Legions | Scryfall ID: 8ced7275-3935-4bba-877d-81282bd171fd | Oracle ID: 9b851461-d258-41c4-bd0a-d24d5f1be6b2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROOKCLAW_ELDER,
    oracle_id = "9b851461-d258-41c4-bd0a-d24d5f1be6b2",
    scryfall_id = "8ced7275-3935-4bba-877d-81282bd171fd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Crookclaw Elder",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
