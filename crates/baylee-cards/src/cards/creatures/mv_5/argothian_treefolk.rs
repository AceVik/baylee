//! Argothian Treefolk — {3}{G}{G} — Creature — Treefolk
//! Oracle: Prevent all damage that would be dealt to this creature by artifact sources.
//! Set: ME4 #143 — Masters Edition IV | Scryfall ID: 36b0d5b3-84d7-4888-90e9-2d0eb16c11d6 | Oracle ID: f3aaef18-dc32-40d6-b48c-f957aa31247f
// IMPLEMENTED — Modifier::PreventDamageFrom(artifacts), all damage, not
// protection.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARGOTHIAN_TREEFOLK,
    oracle_id = "f3aaef18-dc32-40d6-b48c-f957aa31247f",
    scryfall_id = "36b0d5b3-84d7-4888-90e9-2d0eb16c11d6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Argothian Treefolk",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(3),
        toughness = Some(5),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::PreventDamageFrom(&Filter::ARTIFACT)
    )],
);
