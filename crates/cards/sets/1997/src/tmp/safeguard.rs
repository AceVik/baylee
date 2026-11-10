//! Safeguard — {3}{W}{W} — Enchantment
//! Oracle: {2}{W}: Prevent all combat damage that would be dealt by target creature this turn.
//! Set: TMP #39 — Tempest | Scryfall ID: 2c8e174c-7abb-4a93-aa1d-8c2a2e815ba6 | Oracle ID: e310c3ab-a729-404d-944f-9b477258495c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAFEGUARD,
    oracle_id = "e310c3ab-a729-404d-944f-9b477258495c",
    scryfall_id = "2c8e174c-7abb-4a93-aa1d-8c2a2e815ba6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Safeguard",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}{W}"),
        &[Effect::continuous(
            &Filter::This,
            Modifier::PreventDamageFromIt,
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
