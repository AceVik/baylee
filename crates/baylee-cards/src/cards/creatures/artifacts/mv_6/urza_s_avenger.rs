//! Urza's Avenger — {6} — Artifact Creature — Shapeshifter
//! Oracle: {0}: This creature gets -1/-1 and gains your choice of banding, flying, first strike, or trample until end of turn. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: 5ED #405 — Fifth Edition | Scryfall ID: 60bd9559-1a8f-47d0-af6b-d0681cae4060 | Oracle ID: 1cfeb5c4-a2d0-4872-a907-a6d484be85a5

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_AVENGER,
    oracle_id = "1cfeb5c4-a2d0-4872-a907-a6d484be85a5",
    scryfall_id = "60bd9559-1a8f-47d0-af6b-d0681cae4060",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urza's Avenger",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        Cost::FREE,
        &[
            Effect::ModifySource {
                modifier: Modifier::ModifyPT(-1, -1),
                duration: Duration::UntilEndOfTurn
            },
            Effect::ChooseOne {
                choices: &[
                    Effect::ModifySource {
                        modifier: Modifier::AddKeyword(KeywordSet::BANDING),
                        duration: Duration::UntilEndOfTurn
                    },
                    Effect::ModifySource {
                        modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                        duration: Duration::UntilEndOfTurn
                    },
                    Effect::ModifySource {
                        modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                        duration: Duration::UntilEndOfTurn
                    },
                    Effect::ModifySource {
                        modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE),
                        duration: Duration::UntilEndOfTurn
                    }
                ]
            },
        ]
    )],
);
