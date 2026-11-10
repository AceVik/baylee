//! Adventurers' Guildhouse — (no cost) — Land
//! Oracle: Green legendary creatures you control have "bands with other legendary creatures." (Any legendary creatures can attack in a band as long as at least one has "bands with other legendary creatures." Bands are blocked as a group. If at least two legendary creatures you control, one of which has "bands with other legendary creatures," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: LEG #300 — Legends | Scryfall ID: 32865e68-5842-4f17-b2ea-4ffa743b511f | Oracle ID: bf29b215-d4f5-4641-97ef-b65d4c463e72

use baylee_cards_dsl::prelude::*;

static GREEN_LEGENDARY_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

static LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

card!(
    index = index::ADVENTURERS_GUILDHOUSE,
    oracle_id = "bf29b215-d4f5-4641-97ef-b65d4c463e72",
    scryfall_id = "32865e68-5842-4f17-b2ea-4ffa743b511f",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Adventurers' Guildhouse",
        types = TypeSet::LAND,
    ),],
    abilities = &[static_ability!(
        GREEN_LEGENDARY_CREATURE_YOU_CONTROL,
        Modifier::BandsWithOther(&LEGENDARY_CREATURE)
    ),],
);
