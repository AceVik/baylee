//! Dryad of the Ilysian Grove — {2}{G} — Enchantment Creature — Nymph Dryad
//! Oracle: You may play an additional land on each of your turns.
//! Oracle: Lands you control are every basic land type in addition to their other types.
//! Set: CMM #891 — Commander Masters | Scryfall ID: 43be1363-7e73-4862-b45f-07f490ab46be | Oracle ID: bdbde5d0-f5e4-44da-b27c-b4ad6f374cc9
// IMPLEMENTED — a second land drop for the controller (ExtraLandDrops) and
// every basic land type on the lands you control (AllBasicLandTypes).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRYAD_OF_THE_ILYSIAN_GROVE,
    oracle_id = "bdbde5d0-f5e4-44da-b27c-b4ad6f374cc9",
    scryfall_id = "43be1363-7e73-4862-b45f-07f490ab46be",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dryad of the Ilysian Grove",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::NYMPH, subtypes::creature::DRYAD],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[
        static_ability!(Filter::Any, Modifier::ExtraLandDrops(1)),
        static_ability!(Filter::YOUR_LAND, Modifier::AllBasicLandTypes),
    ],
);
