//! Rogue Skycaptain — {2}{R} — Creature — Human Rogue Mercenary
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, put a wage counter on this creature. You may pay {2} for each wage counter on it. If you don't, remove all wage counters from this creature and an opponent gains control of it.
//! Set: ME2 #149 — Masters Edition II | Scryfall ID: 2d449a05-c0b9-4532-a67d-21d3d3baea59 | Oracle ID: e600136a-8b4f-4725-a29e-021e0cfc7d0a
// IMPLEMENTED — flying and upkeep wage counter trigger with {2}-per-counter payment or removing all wage counters and an opponent gaining control of Rogue Skycaptain.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROGUE_SKYCAPTAIN,
    oracle_id = "e600136a-8b4f-4725-a29e-021e0cfc7d0a",
    scryfall_id = "2d449a05-c0b9-4532-a67d-21d3d3baea59",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rogue Skycaptain",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::ROGUE,
            subtypes::creature::MERCENARY
        ],
        power = Some(3),
        toughness = Some(4),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[
            Effect::AddCounter {
                kind: counters::WAGE,
                amount: Amount::Fixed(1),
            },
            Effect::PlayerMayPayManaEachThenElse {
                player: PlayerRel::You,
                each: mana!("{2}"),
                times: Amount::CountersOnSource(counters::WAGE),
                paid: &[],
                unpaid: &[
                    Effect::RemoveAllCountersSelf {
                        kind: counters::WAGE,
                    },
                    Effect::ChangeController {
                        new_controller: PlayerRel::Opponent,
                    },
                ],
            },
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        ))),
    )],
);
