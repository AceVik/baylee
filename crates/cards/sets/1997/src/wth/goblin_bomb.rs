//! Goblin Bomb — {1}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, you may flip a coin. If you win the flip, put a fuse counter on this enchantment. If you lose the flip, remove a fuse counter from this enchantment.
//! Oracle: Remove five fuse counters from this enchantment and sacrifice it: It deals 20 damage to target player or planeswalker.
//! Set: WTH #103 — Weatherlight | Scryfall ID: 97e8a436-9fd0-409f-a020-0f9f41602d50 | Oracle ID: 061e2ef6-ce8d-4319-b525-96b9c2824001
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOBLIN_BOMB,
    oracle_id = "061e2ef6-ce8d-4319-b525-96b9c2824001",
    scryfall_id = "97e8a436-9fd0-409f-a020-0f9f41602d50",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Bomb",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::MayDo {
                effects: &[Effect::FlipCoin {
                    won: &[Effect::AddCounter {
                        kind: counters::FUSE,
                        amount: Amount::Fixed(1)
                    }],
                    lost: &[Effect::RemoveCounterSelf {
                        kind: counters::FUSE,
                        n: 1
                    }]
                }]
            }],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            )))
        ),
        activated!(
            cost!(
                RemoveCounterSelf {
                    kind: counters::FUSE,
                    n: 5
                },
                SacrificeSelf
            ),
            &[Effect::DealDamage {
                amount: Amount::Fixed(20),
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)
            }],
            target = Some(TargetSpec::PlayerOrObject(&Filter::PLANESWALKER))
        ),
    ],
);
