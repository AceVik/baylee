//! Arctic Foxes — {1}{W} — Creature — Fox
//! Oracle: This creature can't be blocked by creatures with power 2 or greater as long as defending player controls a snow land.
//! Set: ICE #2 — Ice Age | Scryfall ID: 98f99c3e-dddc-492f-aab6-1d899346a385 | Oracle ID: 898f0824-cf4c-4e34-8407-9a32fdae6dcb

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

// A creature with power 2 or greater whose controller controls a snow land.
// A blocker's controller is the defending player it blocks for (CR 509.1a,
// 802.4a), so asking it of each blocker is asking it of the defending player
// as the block is declared.
static BLOCKER_WHILE_DEFENDER_HAS_SNOW: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::PowerAtLeast(2),
    Filter::ControllerControls(&SNOW_LAND),
]);

card!(
    index = index::ARCTIC_FOXES,
    oracle_id = "898f0824-cf4c-4e34-8407-9a32fdae6dcb",
    scryfall_id = "98f99c3e-dddc-492f-aab6-1d899346a385",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arctic Foxes",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FOX],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantBeBlockedBy(&BLOCKER_WHILE_DEFENDER_HAS_SNOW)
    )],
);
