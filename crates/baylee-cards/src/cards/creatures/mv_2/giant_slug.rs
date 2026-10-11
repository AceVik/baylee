//! Giant Slug — {1}{B} — Creature — Slug
//! Oracle: {5}: At the beginning of your next upkeep, choose a basic land type. This creature gains landwalk of the chosen type until the end of that turn. (It can't be blocked as long as defending player controls a land of that type.)
//! Set: CHR #33 — Chronicles | Scryfall ID: d78999ab-2ccc-41ec-b808-18e40702d1c3 | Oracle ID: 1ce7f356-1f9b-44dc-9b05-f7b1ecc5d755

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_SLUG,
    oracle_id = "1ce7f356-1f9b-44dc-9b05-f7b1ecc5d755",
    scryfall_id = "d78999ab-2ccc-41ec-b808-18e40702d1c3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{5}"),
        &[Effect::AtNextUpkeep {
            effects: &[Effect::ChooseSourceLandwalk {
                duration: Duration::UntilEndOfTurn
            }],
        }]
    ),],
    faces = &[face!(
        name = "Giant Slug",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLUG],
        power = Some(1),
        toughness = Some(1),
    ),],
);
