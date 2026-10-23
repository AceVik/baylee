//! Seeds of Innocence — {1}{G}{G} — Sorcery
//! Oracle: Destroy all artifacts. They can't be regenerated. The controller of each of those artifacts gains life equal to its mana value.
//! Set: MIR #241 — Mirage | Scryfall ID: f9c868f5-0f90-4f7e-bafb-c45d2372fe06 | Oracle ID: f5ebad5c-9675-4a37-b76c-760416083939
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEEDS_OF_INNOCENCE,
    oracle_id = "f5ebad5c-9675-4a37-b76c-760416083939",
    scryfall_id = "f9c868f5-0f90-4f7e-bafb-c45d2372fe06",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Seeds of Innocence",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
