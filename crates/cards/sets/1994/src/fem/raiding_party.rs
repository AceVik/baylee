//! Raiding Party — {2}{R} — Enchantment
//! Oracle: This enchantment can't be the target of white spells or abilities from white sources.
//! Oracle: Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player.
//! Set: FEM #64 — Fallen Empires | Scryfall ID: 907a3396-706b-4ca2-9973-bca758986032 | Oracle ID: 1d9b8859-14e3-4bbe-ad92-916b25a37b31
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const WHITE_SOURCE: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));
const ORC: Filter = Filter::HasSubtype(subtypes::creature::ORC);

card!(
    index = index::RAIDING_PARTY,
    oracle_id = "1d9b8859-14e3-4bbe-ad92-916b25a37b31",
    scryfall_id = "907a3396-706b-4ca2-9973-bca758986032",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Raiding Party",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::CantBeTargetedBy(&WHITE_SOURCE)),
        activated!(cost!(Sacrifice(&ORC)), &[Effect::RaidingParty]),
    ],
);
