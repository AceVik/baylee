//! Serra's Liturgy — {2}{W}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {W}, Sacrifice this enchantment: Destroy up to X target artifacts and/or enchantments, where X is the number of verse counters on this enchantment.
//! Set: USG #49 — Urza's Saga | Scryfall ID: b84be3ef-a820-41e3-b66a-09303dad32dd | Oracle ID: 17b7b7e4-4ae8-495d-bf0c-d18697f250ea
// PARTIAL: targeting up to X artifacts and/or enchantments where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to destroy up to one target artifact or enchantment.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SERRA_S_LITURGY,
    oracle_id = "17b7b7e4-4ae8-495d-bf0c-d18697f250ea",
    scryfall_id = "b84be3ef-a820-41e3-b66a-09303dad32dd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "targeting up to X artifacts and/or enchantments where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to destroy up to one target artifact or enchantment",
    ),
    faces = &[face!(
        name = "Serra's Liturgy",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: targeting up to X artifacts and/or enchantments where X is the number of verse counters on this enchantment is not in TargetReq; puts a verse counter on itself on your upkeep and sacrifices itself only with at least one verse counter to destroy up to one target artifact or enchantment.
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
            cost!("{W}", SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(
                &Filter::ARTIFACT_OR_ENCHANTMENT,
            ))],
            condition = Some(Condition::CountersOnSelf(counters::VERSE, 1)),
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(
                &Filter::ARTIFACT_OR_ENCHANTMENT,
            ))),
        ),
    ],
);
