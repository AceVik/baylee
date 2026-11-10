//! Light of Day — {3}{W} — Enchantment
//! Oracle: Black creatures can't attack or block.
//! Set: 6ED #29 — Classic Sixth Edition | Scryfall ID: 3d5af047-7d98-4ba9-9ce1-c67563c19866 | Oracle ID: cd854e00-c9da-4593-8402-4bae637f5185
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static BLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::LIGHT_OF_DAY,
    oracle_id = "cd854e00-c9da-4593-8402-4bae637f5185",
    scryfall_id = "3d5af047-7d98-4ba9-9ce1-c67563c19866",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Light of Day",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(BLACK_CREATURE, Modifier::CantAttack),
        static_ability!(BLACK_CREATURE, Modifier::CantBlock(&Filter::Any)),
    ],
);
