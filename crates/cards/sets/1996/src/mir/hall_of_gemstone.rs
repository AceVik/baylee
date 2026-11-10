//! Hall of Gemstone — {1}{G}{G} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses a color. Until end of turn, lands tapped for mana produce mana of the chosen color instead of any other color.
//! Set: MIR #221 — Mirage | Scryfall ID: 18e4551f-9f6c-4421-ad66-a270df6d3463 | Oracle ID: 3da9fecc-064a-44e6-b88e-4b10194e367e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_GEMSTONE,
    oracle_id = "3da9fecc-064a-44e6-b88e-4b10194e367e",
    scryfall_id = "18e4551f-9f6c-4421-ad66-a270df6d3463",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hall of Gemstone",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
