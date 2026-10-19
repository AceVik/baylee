//! Witch Hunter — {2}{W}{W} — Creature — Human Cleric
//! Oracle: {T}: This creature deals 1 damage to target player or planeswalker.
//! Oracle: {1}{W}{W}, {T}: Return target creature an opponent controls to its owner's hand.
//! Set: TSB #17 — Time Spiral Timeshifted | Scryfall ID: 980ada04-1fb2-4aa8-b342-8934767587e0 | Oracle ID: 5fc1f04a-2c39-4b3a-b62e-6e30c3d8a583

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WITCH_HUNTER,
    oracle_id = "5fc1f04a-2c39-4b3a-b62e-6e30c3d8a583",
    scryfall_id = "980ada04-1fb2-4aa8-b342-8934767587e0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Witch Hunter",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
        abilities = &[
            activated!(
                Cost::TAP,
                &[Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
                }],
                target = Some(TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)),
            ),
            activated!(
                cost!("{1}{W}{W}", TapSelf),
                &[Effect::ReturnToHand {
                    target: TargetSpec::Object(&Filter::OPPONENT_CREATURE),
                }],
                target = Some(TargetSpec::Object(&Filter::OPPONENT_CREATURE)),
            ),
        ],
    ),],
);
