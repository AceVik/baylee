//! Revered Unicorn — {1}{W} — Creature — Unicorn
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When this creature leaves the battlefield, you gain life equal to the number of age counters on it.
//! Set: WTH #23 — Weatherlight | Scryfall ID: 8c642dd2-1a3e-4b08-917e-6e8aed358b72 | Oracle ID: 18a8b50b-2328-459b-979d-04c4c769e569
// IMPLEMENTED — Cumulative upkeep {1} and gains life equal to the number of age counters on it when it leaves the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::REVERED_UNICORN,
    oracle_id = "18a8b50b-2328-459b-979d-04c4c769e569",
    scryfall_id = "8c642dd2-1a3e-4b08-917e-6e8aed358b72",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Revered Unicorn",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::UNICORN],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayOr {
                    player: PlayerRel::You,
                    mana: Amount::CountersOnSource(counters::AGE),
                    effect: &Effect::SacrificeSelf,
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::GainLife {
                amount: Amount::CountersOnSource(counters::AGE),
            }],
        ),
    ],
);
