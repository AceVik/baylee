//! Torch Song — {2}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {2}{R}, Sacrifice this enchantment: It deals X damage to any target, where X is the number of verse counters on this enchantment.
//! Set: USG #222 — Urza's Saga | Scryfall ID: 6ba8cdf0-492a-4e64-9143-4ccb29ba1d56 | Oracle ID: 69826a0d-3ec9-47e7-b129-2fc13b41a07e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TORCH_SONG,
    oracle_id = "69826a0d-3ec9-47e7-b129-2fc13b41a07e",
    scryfall_id = "6ba8cdf0-492a-4e64-9143-4ccb29ba1d56",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Torch Song",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: counters::VERSE,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        activated!(
            cost!("{2}{R}", SacrificeSelf),
            &[Effect::DealDamage {
                amount: Amount::CountersOnSource(counters::VERSE),
                target: TargetSpec::AnyTarget,
            }],
            target = Some(TargetSpec::AnyTarget),
        ),
    ],
);
