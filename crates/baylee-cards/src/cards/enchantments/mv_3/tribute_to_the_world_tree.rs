//! Tribute to the World Tree — {G}{G}{G} — Enchantment
//! Oracle: Whenever a creature you control enters, draw a card if its power is 3 or greater. Otherwise, put two +1/+1 counters on it.
//! Set: MOM #211 — March of the Machine | Scryfall ID: c0cdeaba-fc21-44e6-bf99-aa1ff379401b | Oracle ID: 72deedab-7c17-4505-aeca-4bc8596d80a5
// IMPLEMENTED — power-conditional ETB trigger (draw or two counters).

static YOUR_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);
static THEN_DRAW: &[Effect] = &[Effect::DrawCards {
    amount: Amount::Fixed(1),
}];
// The nested resolution targets the event object (the entering creature).
static ELSE_COUNTERS: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(2),
}];

use baylee_cards_dsl::prelude::*;

card! {
    index: 173,
    oracle_id: "72deedab-7c17-4505-aeca-4bc8596d80a5",
    scryfall_id: "c0cdeaba-fc21-44e6-bf99-aa1ff379401b",
    faces: &[face! {
        name: "Tribute to the World Tree",
        mana_cost: baylee_core::mana!("{G}{G}{G}"),
        types: TypeSet::ENCHANTMENT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&YOUR_CREATURE), &[Effect::IfEventPowerAtLeast {
            n: 3,
            then: THEN_DRAW,
            otherwise: ELSE_COUNTERS,
        }])],
}
