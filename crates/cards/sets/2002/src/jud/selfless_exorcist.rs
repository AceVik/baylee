//! Selfless Exorcist — {3}{W}{W} — Creature — Human Cleric
//! Oracle: {T}: Exile target creature card from a graveyard. That card deals damage equal to its power to this creature.
//! Set: JUD #21 — Judgment | Scryfall ID: c9b1c300-aec3-4512-9902-309615e86c73 | Oracle ID: 78e875b1-e621-4bf0-ad81-24ed9e2b43f8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SELFLESS_EXORCIST,
    oracle_id = "78e875b1-e621-4bf0-ad81-24ed9e2b43f8",
    scryfall_id = "c9b1c300-aec3-4512-9902-309615e86c73",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Selfless Exorcist",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
