//! Skirk Alarmist — {1}{R} — Creature — Human Wizard
//! Oracle: Haste
//! Oracle: {T}: Turn target face-down creature you control face up. At the beginning of the next end step, sacrifice it.
//! Set: LGN #111 — Legions | Scryfall ID: fd1c1d41-8666-4c1d-9498-0e259472958d | Oracle ID: a7d20fb0-005b-4bcc-9592-cdbd3fbdd43a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKIRK_ALARMIST,
    oracle_id = "a7d20fb0-005b-4bcc-9592-cdbd3fbdd43a",
    scryfall_id = "fd1c1d41-8666-4c1d-9498-0e259472958d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skirk Alarmist",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
