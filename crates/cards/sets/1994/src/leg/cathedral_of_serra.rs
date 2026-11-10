//! Cathedral of Serra — (no cost) — Land
//! Oracle: White legendary creatures you control have "bands with other legendary creatures." (Any legendary creatures can attack in a band as long as at least one has "bands with other legendary creatures." Bands are blocked as a group. If at least two legendary creatures you control, one of which has "bands with other legendary creatures," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: LEG #301 — Legends | Scryfall ID: e65356e6-0ead-49fd-b069-be1ea9b1c105 | Oracle ID: 4b9922b9-c5b5-467e-8a5b-1e45e862194d

use baylee_cards_dsl::prelude::*;

static WHITE_LEGENDARY_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

static LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

card!(
    index = index::CATHEDRAL_OF_SERRA,
    oracle_id = "4b9922b9-c5b5-467e-8a5b-1e45e862194d",
    scryfall_id = "e65356e6-0ead-49fd-b069-be1ea9b1c105",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Cathedral of Serra", types = TypeSet::LAND,),],
    abilities = &[static_ability!(
        WHITE_LEGENDARY_CREATURE_YOU_CONTROL,
        Modifier::BandsWithOther(&LEGENDARY_CREATURE)
    ),],
);
