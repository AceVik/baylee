//! Reign of Terror — {3}{B}{B} — Sorcery
//! Oracle: Destroy all green creatures or all white creatures. They can't be regenerated. You lose 2 life for each creature that died this way.
//! Set: MIR #137 — Mirage | Scryfall ID: 7bd83049-aec1-4911-bc70-39adba04b174 | Oracle ID: 67d12a5a-9e0c-4550-86ad-cd1e173fea8f
// IMPLEMENTED — the controller chooses green or white as this resolves, every creature of that color is destroyed with no regeneration, and the controller loses 2 life for each creature that died this way (a creature that regenerated or was exiled instead is not counted).

use baylee_cards_dsl::prelude::*;

static GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);
static WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

static CHOICES: &[Effect] = &[
    Effect::DestroyAll {
        filter: &GREEN_CREATURE,
        no_regen: true,
    },
    Effect::DestroyAll {
        filter: &WHITE_CREATURE,
        no_regen: true,
    },
];

card!(
    index = index::REIGN_OF_TERROR,
    oracle_id = "67d12a5a-9e0c-4550-86ad-cd1e173fea8f",
    scryfall_id = "7bd83049-aec1-4911-bc70-39adba04b174",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reign of Terror",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::ChooseOne { choices: CHOICES },
        // "You lose 2 life for each creature that died this way."
        Effect::LoseLife {
            amount: Amount::Times {
                base: &Amount::ThisWay(ThisWay::Died(&Filter::CREATURE)),
                factor: 2,
            },
            target: PlayerRel::You,
        },
    ])],
);
