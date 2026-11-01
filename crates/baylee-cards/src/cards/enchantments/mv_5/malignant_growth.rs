//! Malignant Growth — {3}{G}{U} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your upkeep, put a growth counter on this enchantment.
//! Oracle: At the beginning of each opponent's draw step, that player draws an additional card for each growth counter on this enchantment, then this enchantment deals damage to the player equal to the number of cards they drew this way.
//! Set: MIR #272 — Mirage | Scryfall ID: 760b6703-ac92-45f6-8c32-60f760eba866 | Oracle ID: b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MALIGNANT_GROWTH,
    oracle_id = "b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc",
    scryfall_id = "760b6703-ac92-45f6-8c32-60f760eba866",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Malignant Growth",
        mana_cost = mana!("{3}{G}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
