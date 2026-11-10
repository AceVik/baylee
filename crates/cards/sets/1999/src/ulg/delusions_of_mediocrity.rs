//! Delusions of Mediocrity — {3}{U} — Enchantment
//! Oracle: When this enchantment enters, you gain 10 life.
//! Oracle: When this enchantment leaves the battlefield, you lose 10 life.
//! Set: 7ED #70 — Seventh Edition | Scryfall ID: ab43fba7-699e-4bb5-ab3e-b5b1c290340c | Oracle ID: a1fb6dfc-1dab-4337-8311-b4ab2ef336c0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DELUSIONS_OF_MEDIOCRITY,
    oracle_id = "a1fb6dfc-1dab-4337-8311-b4ab2ef336c0",
    scryfall_id = "ab43fba7-699e-4bb5-ab3e-b5b1c290340c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Delusions of Mediocrity",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::gain_life(10)]),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::LoseLife {
                target: PlayerRel::You,
                amount: Amount::Fixed(10),
            }],
        ),
    ],
);
