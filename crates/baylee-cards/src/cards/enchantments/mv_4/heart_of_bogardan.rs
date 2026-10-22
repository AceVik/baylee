//! Heart of Bogardan — {2}{R}{R} — Enchantment
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When a player doesn't pay this enchantment's cumulative upkeep, this enchantment deals X damage to target player or planeswalker and each creature that player or that planeswalker's controller controls, where X is twice the number of age counters on this enchantment minus 2.
//! Set: WTH #106 — Weatherlight | Scryfall ID: 4e30d025-1df9-4a08-b686-037e9cbf23a6 | Oracle ID: 1e1d8c64-4cea-4c2c-8933-be19d13bf07c
// PARTIAL — Has cumulative upkeep {2}; triggering when a player doesn't pay its cumulative upkeep to deal X damage to target player or planeswalker and each creature they control is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEART_OF_BOGARDAN,
    oracle_id = "1e1d8c64-4cea-4c2c-8933-be19d13bf07c",
    scryfall_id = "4e30d025-1df9-4a08-b686-037e9cbf23a6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Has cumulative upkeep {2}; triggering when a player doesn't pay its cumulative upkeep to deal X damage to target player or planeswalker and each creature they control is not yet modeled."
    ),
    faces = &[face!(
        name = "Heart of Bogardan",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[
            Effect::AddCounter {
                kind: counters::AGE,
                amount: Amount::Fixed(1)
            },
            Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: Amount::Times {
                    base: &Amount::CountersOnSource(counters::AGE),
                    factor: 2
                },
                effect: &Effect::SacrificeSelf
            }
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        )))
    ),],
);
