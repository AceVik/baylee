//! Soltari Guerrillas — {2}{R}{W} — Creature — Soltari Soldier
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: {0}: The next time this creature would deal combat damage to an opponent this turn, it deals that damage to target creature instead.
//! Set: TPR #212 — Tempest Remastered | Scryfall ID: 2fabd462-15d3-4a08-bad2-edab2a142420 | Oracle ID: 37da03c2-c03e-453c-8fbf-e1039faceb8c
// PARTIAL: 3/2 with shadow; the {0} activated ability redirecting the next combat damage this creature would deal to an opponent this turn to target creature instead is not in Effect.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOLTARI_GUERRILLAS,
    oracle_id = "37da03c2-c03e-453c-8fbf-e1039faceb8c",
    scryfall_id = "2fabd462-15d3-4a08-bad2-edab2a142420",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Partial(
        "3/2 with shadow; the {0} activated ability redirecting the next combat damage this creature would deal to an opponent this turn to target creature instead is not in Effect",
    ),
    faces = &[face!(
        name = "Soltari Guerrillas",
        mana_cost = mana!("{2}{R}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SOLTARI, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(2),
    ),],
);
