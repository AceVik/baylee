//! Skirk Alarmist — {1}{R} — Creature — Human Wizard
//! Oracle: Haste
//! Oracle: {T}: Turn target face-down creature you control face up. At the beginning of the next end step, sacrifice it.
//! Set: LGN #111 — Legions | Scryfall ID: fd1c1d41-8666-4c1d-9498-0e259472958d | Oracle ID: a7d20fb0-005b-4bcc-9592-cdbd3fbdd43a
// IMPLEMENTED — haste and tap to turn target face-down creature you control face up with delayed end-step sacrifice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_FACE_DOWN_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::FaceDown, Filter::ControlledByYou]);

card!(
    index = index::SKIRK_ALARMIST,
    oracle_id = "a7d20fb0-005b-4bcc-9592-cdbd3fbdd43a",
    scryfall_id = "fd1c1d41-8666-4c1d-9498-0e259472958d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skirk Alarmist",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::TurnFaceUp {
                target: TargetSpec::Object(&YOUR_FACE_DOWN_CREATURE),
            },
            Effect::AtNextEndStep {
                effects: &[Effect::SacrificeEvent],
            },
        ],
        target = Some(TargetSpec::Object(&YOUR_FACE_DOWN_CREATURE)),
    )],
);
