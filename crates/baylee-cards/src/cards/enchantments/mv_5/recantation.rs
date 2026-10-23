//! Recantation — {3}{U}{U} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {U}, Sacrifice this enchantment: Return up to X target permanents to their owners' hands, where X is the number of verse counters on this enchantment.
//! Set: USG #91 — Urza's Saga | Scryfall ID: 95479839-779e-4dbe-8989-ccf26cc488fe | Oracle ID: 02269353-2949-4599-b5a3-f20fdb501285
// PARTIAL: targeting up to X permanents where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to return up to one target permanent to its owner's hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECANTATION,
    oracle_id = "02269353-2949-4599-b5a3-f20fdb501285",
    scryfall_id = "95479839-779e-4dbe-8989-ccf26cc488fe",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "targeting up to X permanents where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to return up to one target permanent to its owner's hand",
    ),
    faces = &[face!(
        name = "Recantation",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: targeting up to X permanents where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to return up to one target permanent to its owner's hand.
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
            cost!("{U}", SacrificeSelf),
            &[Effect::ReturnToHand {
                target: TargetSpec::Object(&Filter::Any),
            }],
            condition = Some(Condition::CountersOnSelf(counters::VERSE, 1)),
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::Any))),
        ),
    ],
);
