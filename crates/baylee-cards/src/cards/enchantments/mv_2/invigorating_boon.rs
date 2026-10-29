//! Invigorating Boon — {1}{G} — Enchantment
//! Oracle: Whenever a player cycles a card, you may put a +1/+1 counter on target creature.
//! Set: DMR #164 — Dominaria Remastered | Scryfall ID: 6eb5124d-bcd6-4aa4-8fd2-f49c8dc6765a | Oracle ID: d00329c2-e65c-4774-83cd-d68e1d1a6e31
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INVIGORATING_BOON,
    oracle_id = "d00329c2-e65c-4774-83cd-d68e1d1a6e31",
    scryfall_id = "6eb5124d-bcd6-4aa4-8fd2-f49c8dc6765a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Invigorating Boon",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
