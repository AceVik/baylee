//! Smuggler's Copter — {2} — Artifact — Vehicle
//! Oracle: Flying
//! Oracle: Whenever this Vehicle attacks or blocks, you may draw a card. If you do, discard a card.
//! Oracle: Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)
//! Set: NEC #160 — Neon Dynasty Commander | Scryfall ID: 2680ed41-da35-475a-9d80-ae2f4686feed | Oracle ID: 49136bdc-bc50-49a2-999a-1ef9c16ea130

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SMUGGLER_S_COPTER,
    oracle_id = "49136bdc-bc50-49a2-999a-1ef9c16ea130",
    scryfall_id = "2680ed41-da35-475a-9d80-ae2f4686feed",
    faces = &[face!(
        name = "Smuggler's Copter",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::VEHICLE],
        power = Some(3),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::MayDo {
                effects: &[
                    Effect::draw(1),
                    Effect::DiscardForPlayers {
                        who: PlayerRel::You,
                        count: 1,
                    },
                ],
            }],
        ),
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::MayDo {
                effects: &[
                    Effect::draw(1),
                    Effect::DiscardForPlayers {
                        who: PlayerRel::You,
                        count: 1
                    },
                ]
            }]
        ),
        crew!(1),
    ],
);
