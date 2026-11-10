//! Rumbling Crescendo — {3}{R}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {R}, Sacrifice this enchantment: Destroy up to X target lands, where X is the number of verse counters on this enchantment.
//! Set: USG #210 — Urza's Saga | Scryfall ID: ddf267c2-c3d5-48ba-93af-dd0af133add3 | Oracle ID: 2e7e19c2-5399-433d-9653-d956f11838f5
// PARTIAL: targeting up to X lands where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to destroy up to one target land.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUMBLING_CRESCENDO,
    oracle_id = "2e7e19c2-5399-433d-9653-d956f11838f5",
    scryfall_id = "ddf267c2-c3d5-48ba-93af-dd0af133add3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "targeting up to X lands where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to destroy up to one target land",
    ),
    faces = &[face!(
        name = "Rumbling Crescendo",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: targeting up to X lands where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to destroy up to one target land.
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
            cost!("{R}", SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
            condition = Some(Condition::CountersOnSelf(counters::VERSE, 1)),
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::LAND))),
        ),
    ],
);
