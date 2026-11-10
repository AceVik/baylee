//! Mana Matrix — {6} — Artifact
//! Oracle: Instant and enchantment spells you cast cost {2} less to cast.
//! Set: ME4 #213 — Masters Edition IV | Scryfall ID: 0c69b20a-34f8-4c61-babe-1d3221c28a07 | Oracle ID: 256cea34-4691-4367-a789-17d15460f664

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_MATRIX,
    oracle_id = "256cea34-4691-4367-a789-17d15460f664",
    scryfall_id = "0c69b20a-34f8-4c61-babe-1d3221c28a07",
    faces = &[face!(
        name = "Mana Matrix",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::ControlledByYou,
            Filter::Or(&[Filter::HasType(TypeSet::INSTANT), Filter::ENCHANTMENT])
        ]),
        Modifier::SpellsCostLess(2)
    )],
);
