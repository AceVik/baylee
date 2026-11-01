//! Shimmer — {2}{U}{U} — Enchantment
//! Oracle: As this enchantment enters, choose a land type.
//! Oracle: Each land of the chosen type has phasing. (It phases in or out before its controller untaps during each of their untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: MIR #92 — Mirage | Scryfall ID: 5c892daa-57fe-4712-b64a-6099d531bb26 | Oracle ID: f557fb2c-581b-4a8d-8766-1e46e92224fa

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::SubtypeId;

static ALL_LAND_SUBTYPES: &[SubtypeId] = &[
    land::CAVE,
    land::CLOUD,
    land::DESERT,
    land::FOREST,
    land::GATE,
    land::ISLAND,
    land::LAIR,
    land::LOCUS,
    land::MINE,
    land::MOUNTAIN,
    land::PLAINS,
    land::PLANET,
    land::POWER_PLANT,
    land::SPHERE,
    land::SWAMP,
    land::TOWER,
    land::TOWN,
    land::URZA_S,
];

static CHOSEN_LAND: Filter = Filter::And(&[Filter::LAND, Filter::MatchesChosenTypeOfSource]);

card!(
    index = index::SHIMMER,
    oracle_id = "f557fb2c-581b-4a8d-8766-1e46e92224fa",
    scryfall_id = "5c892daa-57fe-4712-b64a-6099d531bb26",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shimmer",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseSubtypeFrom(ALL_LAND_SUBTYPES)],
    ),],
    abilities = &[static_ability!(
        CHOSEN_LAND,
        Modifier::AddKeyword(KeywordSet::PHASING),
    )],
);
