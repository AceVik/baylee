//! Chill — {1}{U} — Enchantment
//! Oracle: Red spells cost {2} more to cast.
//! Set: 6ED #60 — Classic Sixth Edition | Scryfall ID: 24846eba-e085-4d1d-8c7a-d4faf11034a6 | Oracle ID: 70a93d0f-9a02-4e60-a2b8-827fd2410604
// IMPLEMENTED — Red spells cost {2} more to cast.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHILL,
    oracle_id = "70a93d0f-9a02-4e60-a2b8-827fd2410604",
    scryfall_id = "24846eba-e085-4d1d-8c7a-d4faf11034a6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chill",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::HasColor(ColorSet::of(Color::Red)),
        Modifier::SpellsCostMore(2)
    )],
);
