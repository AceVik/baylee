//! Ring of Immortals — {5} — Artifact
//! Oracle: {3}, {T}: Counter target instant or Aura spell that targets a permanent you control.
//! Set: LEG #293 — Legends | Scryfall ID: 61706102-67fd-4167-bd7d-ec6da41db362 | Oracle ID: 468a8c2e-0cc2-4108-809d-42ca1eb25ff2
// PARTIAL — nothing is built: the spell's own targets cannot be asked.
// NOT SUPPORTED: "{3}, {T}: Counter target instant or Aura spell that
// targets a permanent you control." — no `Filter` predicate reads a spell's
// targets (`Filter::WithSingleTarget` counts them and nothing matches what
// they are), so the restriction cannot be stated; dropping it would counter
// any instant or Aura spell.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RING_OF_IMMORTALS,
    oracle_id = "468a8c2e-0cc2-4108-809d-42ca1eb25ff2",
    scryfall_id = "61706102-67fd-4167-bd7d-ec6da41db362",
    faces = &[face!(
        name = "Ring of Immortals",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Filter predicate reads a spell's targets, so \"targets a permanent \
         you control\" cannot be stated",
    ),
);
