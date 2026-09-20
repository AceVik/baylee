//! Exploration — {G} — Enchantment
//! Oracle: You may play an additional land on each of your turns.
//! Set: DMR #159 — Dominaria Remastered | Scryfall ID: 5b372045-a4a0-44c8-96ec-1e201d61ed26 | Oracle ID: 0c2841bb-038c-4fbf-8360-bc0a1522b58d
// IMPLEMENTED — one extra land drop each turn (Modifier::ExtraLandDrops(1)).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXPLORATION,
    oracle_id = "0c2841bb-038c-4fbf-8360-bc0a1522b58d",
    scryfall_id = "5b372045-a4a0-44c8-96ec-1e201d61ed26",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Exploration",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(Filter::Any, Modifier::ExtraLandDrops(1))],
);
