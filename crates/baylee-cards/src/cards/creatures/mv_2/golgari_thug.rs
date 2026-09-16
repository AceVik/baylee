//! Golgari Thug — {1}{B} — Creature — Human Warrior
//! Oracle: When this creature dies, put target creature card from your graveyard on top of your library.
//! Oracle: Dredge 4 (If you would draw a card, you may mill four cards instead. If you do, return this card from your graveyard to your hand.)
//! Set: RVR #76 — Ravnica Remastered | Scryfall ID: 6a8c5d3b-71b4-4e99-82e9-dfc0b98698b0 | Oracle ID: a426a258-fd8b-489c-8642-9868ee47de85
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOLGARI_THUG,
    oracle_id = "a426a258-fd8b-489c-8642-9868ee47de85",
    scryfall_id = "6a8c5d3b-71b4-4e99-82e9-dfc0b98698b0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Golgari Thug",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
