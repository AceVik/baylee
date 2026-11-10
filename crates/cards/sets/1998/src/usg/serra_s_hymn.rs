//! Serra's Hymn — {W} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: Sacrifice this enchantment: Prevent the next X damage that would be dealt this turn to any number of targets, divided as you choose, where X is the number of verse counters on this enchantment.
//! Set: USG #48 — Urza's Saga | Scryfall ID: 74b8205a-608e-4274-a32c-802a7ce52d9c | Oracle ID: 66a3ffde-4c81-4798-b99b-a6cc3a93c3e3
// PARTIAL: dividing X damage prevention among any number of targets is not in Effect; puts a verse counter on itself on your upkeep and only activates with at least one verse counter and sacrifices itself to prevent the next X damage to any single target this turn, where X is the number of verse counters on it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SERRA_S_HYMN,
    oracle_id = "66a3ffde-4c81-4798-b99b-a6cc3a93c3e3",
    scryfall_id = "74b8205a-608e-4274-a32c-802a7ce52d9c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "dividing X damage prevention among any number of targets is not in Effect; puts a verse counter on itself on your upkeep and only activates with at least one verse counter and sacrifices itself to prevent the next X damage to any single target this turn, where X is the number of verse counters on it",
    ),
    faces = &[face!(
        name = "Serra's Hymn",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: dividing X damage prevention among any number of targets is not in Effect; puts a verse counter on itself on your upkeep and only activates with at least one verse counter and sacrifices itself to prevent the next X damage to any single target this turn, where X is the number of verse counters on it.
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
            cost!(SacrificeSelf),
            &[Effect::PreventNextDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::CountersOnSource(counters::VERSE),
            }],
            target = Some(TargetSpec::AnyTarget),
            condition = Some(Condition::CountersOnSelf(counters::VERSE, 1)),
        ),
    ],
);
