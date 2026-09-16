//! Twinflame — {1}{R} — Sorcery
//! Oracle: Strive — This spell costs {2}{R} more to cast for each target beyond the first.
//! Oracle: Choose any number of target creatures you control. For each of them, create a token that's a copy of that creature, except it has haste. Exile those tokens at the beginning of the next end step.
//! Set: SOC #258 — Secrets of Strixhaven Commander | Scryfall ID: 85c863ef-b266-410d-87b3-ced791f99966 | Oracle ID: 83cf1169-5853-4332-b897-7b17d72d76ab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TWINFLAME,
    oracle_id = "83cf1169-5853-4332-b897-7b17d72d76ab",
    scryfall_id = "85c863ef-b266-410d-87b3-ced791f99966",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Twinflame",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
