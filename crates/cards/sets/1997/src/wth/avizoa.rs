//! Avizoa — {3}{U} — Creature — Jellyfish
//! Oracle: Flying
//! Oracle: {0}: This creature gets +2/+2 until end of turn. You skip your next untap step. Activate only once each turn.
//! Set: WTH #35 — Weatherlight | Scryfall ID: a993986c-e8f1-41b1-86e6-c72021c53b87 | Oracle ID: 91520db8-2d50-4c17-9f0e-2f92f63285f7
// PARTIAL — flying body only; the optional pump needs separately consumed
// skipped-untap-step debts, including activations during successive turns.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVIZOA,
    oracle_id = "91520db8-2d50-4c17-9f0e-2f92f63285f7",
    scryfall_id = "a993986c-e8f1-41b1-86e6-c72021c53b87",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "flying body only; the optional +2/+2 activation is omitted because multiple skip-your-next-untap effects must consume separate future untap steps",
    ),
    faces = &[face!(
        name = "Avizoa",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::JELLYFISH],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
    // NOT SUPPORTED: the optional pump and its skip-your-next-untap obligation
    // are omitted together. A continuous skip expiring at the next upkeep
    // merges obligations from activations on different turns into one skip.
);
