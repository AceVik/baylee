//! Cycle of Life — {1}{G}{G} — Enchantment
//! Oracle: Return this enchantment to its owner's hand: Target creature you cast this turn has base power and toughness 0/1 until your next upkeep. At the beginning of your next upkeep, put a +1/+1 counter on that creature.
//! Set: MIR #211 — Mirage | Scryfall ID: 49b53861-97aa-4fff-9526-7d496d1717a4 | Oracle ID: 5941374b-eeb8-4d0e-9367-f7f0f8b79739
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CYCLE_OF_LIFE,
    oracle_id = "5941374b-eeb8-4d0e-9367-f7f0f8b79739",
    scryfall_id = "49b53861-97aa-4fff-9526-7d496d1717a4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Cycle of Life",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
