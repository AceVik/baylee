//! Transmutation — {1}{B} — Instant
//! Oracle: Switch target creature's power and toughness until end of turn.
//! Set: CHR #40 — Chronicles | Scryfall ID: e2d90519-68f8-43ab-902a-0fed0f526488 | Oracle ID: 72a75113-4ddf-460d-8d9b-03931c2788da
// IMPLEMENTED — the target creature's power and toughness are switched until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRANSMUTATION,
    oracle_id = "72a75113-4ddf-460d-8d9b-03931c2788da",
    scryfall_id = "e2d90519-68f8-43ab-902a-0fed0f526488",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Transmutation",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::continuous(
            &Filter::This,
            Modifier::SwitchPT,
            Duration::UntilEndOfTurn
        )],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
