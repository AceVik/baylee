//! Reign of Terror — {3}{B}{B} — Sorcery
//! Oracle: Destroy all green creatures or all white creatures. They can't be regenerated. You lose 2 life for each creature that died this way.
//! Set: MIR #137 — Mirage | Scryfall ID: 7bd83049-aec1-4911-bc70-39adba04b174 | Oracle ID: 67d12a5a-9e0c-4550-86ad-cd1e173fea8f
// PARTIAL: Effect::DestroyAll does not count creatures that died this way to make you lose 2 life for each; chooses between destroying all green creatures or all white creatures with no regeneration.

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
    coverage = Coverage::Partial(
        "Effect::DestroyAll does not count creatures that died this way to make you lose 2 life for each; chooses between destroying all green creatures or all white creatures with no regeneration",
    ),
    faces = &[face!(
        name = "Reign of Terror",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::ChooseOne { choices: CHOICES }])],
);
