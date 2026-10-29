//! Fervent Denial — {3}{U}{U} — Instant
//! Oracle: Counter target spell.
//! Oracle: Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: C19 #86 — Commander 2019 | Scryfall ID: 7b15428e-946e-490d-93bb-9888bfd3a1df | Oracle ID: 2e56bc34-1e35-4b95-9525-cb58ac8ba060
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FERVENT_DENIAL,
    oracle_id = "2e56bc34-1e35-4b95-9525-cb58ac8ba060",
    scryfall_id = "7b15428e-946e-490d-93bb-9888bfd3a1df",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Fervent Denial",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
