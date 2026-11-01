//! Lilting Refrain — {1}{U} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: Sacrifice this enchantment: Counter target spell unless its controller pays {X}, where X is the number of verse counters on this enchantment.
//! Set: USG #83 — Urza's Saga | Scryfall ID: ef319154-c5fc-4432-a860-a05508c132d4 | Oracle ID: f91a71b7-9bc0-4609-a312-ffede52dc0ad
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LILTING_REFRAIN,
    oracle_id = "f91a71b7-9bc0-4609-a312-ffede52dc0ad",
    scryfall_id = "ef319154-c5fc-4432-a860-a05508c132d4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lilting Refrain",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: counters::VERSE,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::TargetPlayerOrController,
                mana: Amount::CountersOnSource(counters::VERSE),
                effect: &Effect::CounterTargetSpell,
            }],
            target = Some(TargetSpec::Spell(&Filter::Any)),
        ),
    ],
);
