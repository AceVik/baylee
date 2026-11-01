//! Black Market — {3}{B}{B} — Enchantment
//! Oracle: Whenever a creature dies, put a charge counter on this enchantment.
//! Oracle: At the beginning of your first main phase, add {B} for each charge counter on this enchantment.
//! Set: CLB #739 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 78c80c6d-f3ae-48d3-ba6a-b9b738e1affb | Oracle ID: 21338b71-37f5-4121-9b84-565025ebcd17
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLACK_MARKET,
    oracle_id = "21338b71-37f5-4121-9b84-565025ebcd17",
    scryfall_id = "78c80c6d-f3ae-48d3-ba6a-b9b738e1affb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Black Market",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::Dies(&Filter::CREATURE),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(1),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::FirstMain,
                whose: PlayerRel::You,
            },
            &[Effect::mana_dynamic(
                ManaColor::Black,
                Amount::CountersOnSource(CounterKind::Charge),
            )],
        ),
    ],
);
