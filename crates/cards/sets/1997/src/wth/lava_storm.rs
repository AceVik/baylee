//! Lava Storm — {3}{R}{R} — Instant
//! Oracle: Lava Storm deals 2 damage to each attacking creature or Lava Storm deals 2 damage to each blocking creature.
//! Set: WTH #110 — Weatherlight | Scryfall ID: 61fcd58e-e5e2-45f4-9edd-300a871ae5f5 | Oracle ID: 78d33345-a147-402f-9a40-1e8260d7d44e
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;

static BLOCKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Blocking]);

card!(
    index = index::LAVA_STORM,
    oracle_id = "78d33345-a147-402f-9a40-1e8260d7d44e",
    scryfall_id = "61fcd58e-e5e2-45f4-9edd-300a871ae5f5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lava Storm",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ChooseOne {
        choices: &[
            Effect::DealDamageEach {
                amount: Amount::Fixed(2),
                filter: &Filter::ATTACKING_CREATURE
            },
            Effect::DealDamageEach {
                amount: Amount::Fixed(2),
                filter: &BLOCKING_CREATURE
            }
        ]
    }]),],
);
