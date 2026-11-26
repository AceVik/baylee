//! Rooting Kavu — {2}{G}{G} — Creature — Kavu
//! Oracle: When this creature dies, you may exile it. If you do, shuffle all creature cards from your graveyard into your library.
//! Set: INV #207 — Invasion | Scryfall ID: 12c25a4c-d93a-402b-999f-0b9919123cc5 | Oracle ID: e7438616-cc42-4a6d-a7e3-5856cb0782fa
// IMPLEMENTED — dies trigger that may exile itself from the graveyard to shuffle all creature cards from your graveyard into your library.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOTING_KAVU,
    oracle_id = "e7438616-cc42-4a6d-a7e3-5856cb0782fa",
    scryfall_id = "12c25a4c-d93a-402b-999f-0b9919123cc5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rooting Kavu",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::IfCondition {
            condition: Condition::SourceMatches(&Filter::InZone(ZoneRef::Graveyard)),
            then: &[Effect::MayDo {
                effects: &[
                    Effect::ExileSource,
                    Effect::ShuffleGraveyardMatchingIntoLibrary {
                        who: PlayerRel::You,
                        filter: &Filter::CREATURE,
                    },
                ],
            }],
            otherwise: &[],
        }]
    ),],
);
