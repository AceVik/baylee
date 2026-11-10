//! Midsummer Revel — {3}{G}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a verse counter on this enchantment.
//! Oracle: {G}, Sacrifice this enchantment: Create X 3/3 green Beast creature tokens, where X is the number of verse counters on this enchantment.
//! Set: USG #268 — Urza's Saga | Scryfall ID: b40a7f65-10fe-4ce5-ad1c-be53a635d7a9 | Oracle ID: d3579282-5045-4b37-9998-9a282e9932b9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIDSUMMER_REVEL,
    oracle_id = "d3579282-5045-4b37-9998-9a282e9932b9",
    scryfall_id = "b40a7f65-10fe-4ce5-ad1c-be53a635d7a9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Midsummer Revel",
        mana_cost = mana!("{3}{G}{G}"),
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
            cost!("{G}", SacrificeSelf),
            &[Effect::CreateTokenN {
                token: &crate::generated_tokens::BEAST_3_3_GREEN,
                amount: Amount::CountersOnSource(counters::VERSE),
            }],
        ),
    ],
);
