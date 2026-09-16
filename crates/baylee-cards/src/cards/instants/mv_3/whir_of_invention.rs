//! Whir of Invention — {X}{U}{U}{U} — Instant
//! Oracle: Improvise (Your artifacts can help cast this spell. Each artifact you tap after you're done activating mana abilities pays for {1}.)
//! Oracle: Search your library for an artifact card with mana value X or less, put it onto the battlefield, then shuffle.
//! Set: AER #49 — Aether Revolt | Scryfall ID: 0279fd3c-9252-4958-9d7a-5f33aa25907e | Oracle ID: 152b91c9-cc07-4ca8-944f-9bc2242a2283
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHIR_OF_INVENTION,
    oracle_id = "152b91c9-cc07-4ca8-944f-9bc2242a2283",
    scryfall_id = "0279fd3c-9252-4958-9d7a-5f33aa25907e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Whir of Invention",
        mana_cost = mana!("{X}{U}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
