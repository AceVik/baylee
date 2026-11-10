//! Derelor — {3}{B} — Creature — Thrull
//! Oracle: Black spells you cast cost {B} more to cast.
//! Set: ME1 #66 — Masters Edition | Scryfall ID: 7793c8e6-8f5f-473c-99ea-a74dff8cecd2 | Oracle ID: 43f2969e-46eb-4fd4-8b2d-ff81aafcc24a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLACK_SPELL_YOU_CAST: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::DERELOR,
    oracle_id = "43f2969e-46eb-4fd4-8b2d-ff81aafcc24a",
    scryfall_id = "7793c8e6-8f5f-473c-99ea-a74dff8cecd2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Derelor",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THRULL],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[static_ability!(
        BLACK_SPELL_YOU_CAST,
        Modifier::SpellsCostMoreMana(mana!("{B}")),
    ),],
);
