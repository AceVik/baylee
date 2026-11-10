//! Hunting Grounds — {G}{W} — Enchantment
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this enchantment has "Whenever an opponent casts a spell, you may put a creature card from your hand onto the battlefield."
//! Set: DMR #191 — Dominaria Remastered | Scryfall ID: 6e544772-5ab8-47dd-8a6e-686d385a6ee7 | Oracle ID: 2f3fec87-3dd2-4e5d-835c-eae535da4ecb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HUNTING_GROUNDS,
    oracle_id = "2f3fec87-3dd2-4e5d-835c-eae535da4ecb",
    scryfall_id = "6e544772-5ab8-47dd-8a6e-686d385a6ee7",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Hunting Grounds",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
