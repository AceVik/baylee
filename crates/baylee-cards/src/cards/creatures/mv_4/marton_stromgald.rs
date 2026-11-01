//! Márton Stromgald — {2}{R}{R} — Legendary Creature — Human Knight
//! Oracle: Whenever Márton Stromgald attacks, other attacking creatures get +1/+1 until end of turn for each attacking creature other than Márton Stromgald.
//! Oracle: Whenever Márton Stromgald blocks, other blocking creatures get +1/+1 until end of turn for each blocking creature other than Márton Stromgald.
//! Set: ME1 #104 — Masters Edition | Scryfall ID: 1e76a75a-7125-4957-ab7a-8e7ead21d002 | Oracle ID: 8364cf93-c323-46f4-854f-ae872e484438
// IMPLEMENTED — attack and block triggers pumping other attacking/blocking creatures by +1/+1 per other attacking/blocking creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_ATTACKING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Attacking, Filter::Another]);
static OTHER_BLOCKING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Blocking, Filter::Another]);

card!(
    index = index::MARTON_STROMGALD,
    oracle_id = "8364cf93-c323-46f4-854f-ae872e484438",
    scryfall_id = "1e76a75a-7125-4957-ab7a-8e7ead21d002",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Márton Stromgald",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::PumpFilter {
                filter: &OTHER_ATTACKING_CREATURE,
                controlled_by: None,
                power: Amount::CountOf {
                    zone: ZoneSel::Battlefield,
                    filter: &OTHER_ATTACKING_CREATURE,
                },
                toughness: Amount::CountOf {
                    zone: ZoneSel::Battlefield,
                    filter: &OTHER_ATTACKING_CREATURE,
                },
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::PumpFilter {
                filter: &OTHER_BLOCKING_CREATURE,
                controlled_by: None,
                power: Amount::CountOf {
                    zone: ZoneSel::Battlefield,
                    filter: &OTHER_BLOCKING_CREATURE,
                },
                toughness: Amount::CountOf {
                    zone: ZoneSel::Battlefield,
                    filter: &OTHER_BLOCKING_CREATURE,
                },
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
