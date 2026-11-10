//! Touch of Death — {2}{B} — Sorcery
//! Oracle: Touch of Death deals 1 damage to target player or planeswalker. You gain 1 life.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: 5ED #200 — Fifth Edition | Scryfall ID: 15e529fb-b951-4f64-85c9-1c2e3b9581a2 | Oracle ID: 52a768eb-16eb-4643-b8be-7a4b5b32f28b

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOUCH_OF_DEATH,
    oracle_id = "52a768eb-16eb-4643-b8be-7a4b5b32f28b",
    scryfall_id = "15e529fb-b951-4f64-85c9-1c2e3b9581a2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Touch of Death",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
            },
            Effect::gain_life(1),
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
            &Filter::PLANESWALKER
        )))
    )],
);
