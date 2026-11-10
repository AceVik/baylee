//! Unholy Citadel — (no cost) — Land
//! Oracle: Black legendary creatures you control have "bands with other legendary creatures." (Any legendary creatures can attack in a band as long as at least one has "bands with other legendary creatures." Bands are blocked as a group. If at least two legendary creatures you control, one of which has "bands with other legendary creatures," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: LEG #309 — Legends | Scryfall ID: 9de534ff-fb48-4692-bd0f-dd237ca28502 | Oracle ID: d410027b-1c22-461e-939d-06a2c851352e

use baylee_cards_dsl::prelude::*;

static BLACK_LEGENDARY_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

static LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
    Filter::HasType(TypeSet::CREATURE),
]);

card!(
    index = index::UNHOLY_CITADEL,
    oracle_id = "d410027b-1c22-461e-939d-06a2c851352e",
    scryfall_id = "9de534ff-fb48-4692-bd0f-dd237ca28502",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Unholy Citadel", types = TypeSet::LAND,),],
    abilities = &[static_ability!(
        BLACK_LEGENDARY_CREATURE_YOU_CONTROL,
        Modifier::BandsWithOther(&LEGENDARY_CREATURE)
    ),],
);
