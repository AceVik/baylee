//! Soldier of Fortune — {R} — Creature — Human Mercenary
//! Oracle: {R}, {T}: Target player shuffles their library.
//! Set: ALL #80 — Alliances | Scryfall ID: 37c05f46-2081-4ebb-a758-894ac040ea2a | Oracle ID: 8b2367b3-3a8d-442f-8d58-6e4c03fa5a46
// IMPLEMENTED — {R}, {T}: the target player shuffles their library.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLDIER_OF_FORTUNE,
    oracle_id = "8b2367b3-3a8d-442f-8d58-6e4c03fa5a46",
    scryfall_id = "37c05f46-2081-4ebb-a758-894ac040ea2a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Soldier of Fortune",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MERCENARY],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{R}", TapSelf),
        &[Effect::ShuffleLibrary {
            who: PlayerRel::Chosen,
        }],
        target = Some(TargetSpec::AnyPlayer)
    )],
);
