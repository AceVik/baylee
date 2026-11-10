//! Breya, Etherium Shaper — {W}{U}{B}{R} — Legendary Artifact Creature — Human
//! Oracle: When Breya enters, create two 1/1 blue Thopter artifact creature tokens with flying.
//! Oracle: {2}, Sacrifice two artifacts: Choose one —
//! Oracle: • Breya deals 3 damage to target player or planeswalker.
//! Oracle: • Target creature gets -4/-4 until end of turn.
//! Oracle: • You gain 5 life.
//! Set: MH3 #289 — Modern Horizons 3 | Scryfall ID: 8bf3929c-957f-4ea2-a27d-7d53979844af | Oracle ID: d460a9e2-5a7d-4562-880e-45174be19a9d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BREYA_ETHERIUM_SHAPER,
    oracle_id = "d460a9e2-5a7d-4562-880e-45174be19a9d",
    scryfall_id = "8bf3929c-957f-4ea2-a27d-7d53979844af",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Breya, Etherium Shaper",
        mana_cost = mana!("{W}{U}{B}{R}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
