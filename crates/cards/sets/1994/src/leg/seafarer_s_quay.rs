//! Seafarer's Quay — (no cost) — Land
//! Oracle: Blue legendary creatures you control have "bands with other legendary creatures." (Any legendary creatures can attack in a band as long as at least one has "bands with other legendary creatures." Bands are blocked as a group. If at least two legendary creatures you control, one of which has "bands with other legendary creatures," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: LEG #306 — Legends | Scryfall ID: 66641d88-b3f0-4bcd-8d2d-29aa2de69e30 | Oracle ID: f71f01bc-caae-426b-a25a-bfbfeb9feb67

use baylee_cards_dsl::prelude::*;

static BLUE_LEGENDARY_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

static LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

card!(
    index = index::SEAFARER_S_QUAY,
    oracle_id = "f71f01bc-caae-426b-a25a-bfbfeb9feb67",
    scryfall_id = "66641d88-b3f0-4bcd-8d2d-29aa2de69e30",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Seafarer's Quay", types = TypeSet::LAND,),],
    abilities = &[static_ability!(
        BLUE_LEGENDARY_CREATURE_YOU_CONTROL,
        Modifier::BandsWithOther(&LEGENDARY_CREATURE)
    ),],
);
