//! Mountain Stronghold — (no cost) — Land
//! Oracle: Red legendary creatures you control have "bands with other legendary creatures." (Any legendary creatures can attack in a band as long as at least one has "bands with other legendary creatures." Bands are blocked as a group. If at least two legendary creatures you control, one of which has "bands with other legendary creatures," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: LEG #304 — Legends | Scryfall ID: 314fd1d7-4bd8-4d95-b7c2-1aa6660ab88a | Oracle ID: af147bb6-aa46-4fc9-af96-de3740c19fd5

use baylee_cards_dsl::prelude::*;

static RED_LEGENDARY_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

static LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

card!(
    index = index::MOUNTAIN_STRONGHOLD,
    oracle_id = "af147bb6-aa46-4fc9-af96-de3740c19fd5",
    scryfall_id = "314fd1d7-4bd8-4d95-b7c2-1aa6660ab88a",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Mountain Stronghold", types = TypeSet::LAND,),],
    abilities = &[static_ability!(
        RED_LEGENDARY_CREATURE_YOU_CONTROL,
        Modifier::BandsWithOther(&LEGENDARY_CREATURE)
    ),],
);
