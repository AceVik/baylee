//! Master of the Hunt — {2}{G}{G} — Creature — Human
//! Oracle: {2}{G}{G}: Create a 1/1 green Wolf creature token named Wolves of the Hunt. It has "bands with other creatures named Wolves of the Hunt." (Any creatures named Wolves of the Hunt can attack in a band as long as at least one has "bands with other creatures named Wolves of the Hunt." Bands are blocked as a group. If at least two creatures named Wolves of the Hunt you control, one of which has "bands with other creatures named Wolves of the Hunt," are blocking or being blocked by the same creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: LEG #194 — Legends | Scryfall ID: 4e6bf56e-2d74-4e4d-a667-885853979377 | Oracle ID: cdf89ad2-ce08-4708-88c2-8acdc861526d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MASTER_OF_THE_HUNT,
    oracle_id = "cdf89ad2-ce08-4708-88c2-8acdc861526d",
    scryfall_id = "4e6bf56e-2d74-4e4d-a667-885853979377",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Master of the Hunt",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{2}{G}{G}"),
        &[Effect::CreateToken {
            token: &crate::tokens::WOLVES_OF_THE_HUNT_1_1_GREEN_WOLF,
        }],
    )],
);
