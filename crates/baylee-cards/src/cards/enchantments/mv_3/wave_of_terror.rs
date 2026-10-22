//! Wave of Terror — {2}{B} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your draw step, destroy each creature with mana value equal to the number of age counters on this enchantment. They can't be regenerated.
//! Set: WTH #86 — Weatherlight | Scryfall ID: d40ab3e7-9abb-4acc-9932-de03b533722f | Oracle ID: 2346be19-7344-4f11-be2c-a22750cb7e1d
// PARTIAL — Cumulative upkeep {1} and draw-step trigger destroying each creature with mana value equal to the number of age counters on this enchantment work; Effect::DestroyAllWithManaValue does not suppress regeneration ("They can't be regenerated").

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAVE_OF_TERROR,
    oracle_id = "2346be19-7344-4f11-be2c-a22750cb7e1d",
    scryfall_id = "d40ab3e7-9abb-4acc-9932-de03b533722f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Cumulative upkeep {1} and draw-step trigger destroying each creature with mana value equal to the number of age counters on this enchantment work; Effect::DestroyAllWithManaValue does not suppress regeneration (\"They can't be regenerated\")."
    ),
    faces = &[face!(
        name = "Wave of Terror",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
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
                    mana: Amount::CountersOnSource(counters::AGE),
                    effect: &Effect::SacrificeSelf
                }
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            )))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Draw,
                whose: PlayerRel::You
            },
            &[Effect::DestroyAllWithManaValue {
                filter: &Filter::CREATURE,
                value: Amount::CountersOnSource(counters::AGE),
            }]
        ),
    ],
);
