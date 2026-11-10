//! Hazduhr the Abbot — {3}{W}{W} — Legendary Creature — Human Cleric
//! Oracle: {X}, {T}: The next X damage that would be dealt this turn to target white creature you control is dealt to Hazduhr instead.
//! Set: HML #8 — Homelands | Scryfall ID: adfd416a-dddf-40e4-acf0-84057edb7a58 | Oracle ID: 35844d8b-8f68-4248-b921-e766757a9e26
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAZDUHR_THE_ABBOT,
    oracle_id = "35844d8b-8f68-4248-b921-e766757a9e26",
    scryfall_id = "adfd416a-dddf-40e4-acf0-84057edb7a58",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Hazduhr the Abbot",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
