//! Caverns of Despair — {2}{R}{R} — World Enchantment
//! Oracle: No more than two creatures can attack each combat.
//! Oracle: No more than two creatures can block each combat.
//! Set: LEG #136 — Legends | Scryfall ID: 209f7479-b3a0-4c27-9602-78babb8d2e99 | Oracle ID: a1034a02-36cf-4586-a001-9dc3fb76e904

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CAVERNS_OF_DESPAIR,
    oracle_id = "a1034a02-36cf-4586-a001-9dc3fb76e904",
    scryfall_id = "209f7479-b3a0-4c27-9602-78babb8d2e99",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Caverns of Despair",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[
        static_ability!(Filter::CREATURE, Modifier::MaxAttackersPerCombat(2)),
        static_ability!(Filter::CREATURE, Modifier::MaxBlockersPerCombat(2)),
    ],
);
