//! Tainted Aether — {2}{B}{B} — Enchantment
//! Oracle: Whenever a creature enters, its controller sacrifices a creature or land of their choice.
//! Set: USG #161 — Urza's Saga | Scryfall ID: d1dd33a1-1603-4368-9c32-e4c9fd6ecd08 | Oracle ID: a61ceda1-5993-479e-945f-15753eeb7049
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static CREATURE_OR_LAND: Filter = Filter::Or(&[Filter::CREATURE, Filter::LAND]);

card!(
    index = index::TAINTED_AETHER,
    oracle_id = "a61ceda1-5993-479e-945f-15753eeb7049",
    scryfall_id = "d1dd33a1-1603-4368-9c32-e4c9fd6ecd08",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tainted Aether",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::CREATURE),
        &[Effect::SacrificeFilter {
            who: PlayerRel::ControllerOfEvent,
            filter: &CREATURE_OR_LAND,
        }],
    ),],
);
