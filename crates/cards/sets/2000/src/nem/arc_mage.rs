//! Arc Mage — {2}{R} — Creature — Human Spellshaper
//! Oracle: {2}{R}, {T}, Discard a card: This creature deals 2 damage divided as you choose among one or two targets.
//! Set: NEM #77 — Nemesis | Scryfall ID: 62982dab-4c27-45b3-9740-38fec3df7226 | Oracle ID: 011bb6cc-f1a1-4790-befc-373acabf63b8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARC_MAGE,
    oracle_id = "011bb6cc-f1a1-4790-befc-373acabf63b8",
    scryfall_id = "62982dab-4c27-45b3-9740-38fec3df7226",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Arc Mage",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
