//! Wishclaw Talisman — {1}{B} — Artifact
//! Oracle: This artifact enters with three wish counters on it.
//! Oracle: {1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.
//! Set: FDN #617 — Foundations | Scryfall ID: 69d0f5bd-ccea-49b2-bd79-ad5e4d850cf5 | Oracle ID: 81c70ae7-3c18-4c9b-8505-e4db9e0e6518
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WISHCLAW_TALISMAN,
    oracle_id = "81c70ae7-3c18-4c9b-8505-e4db9e0e6518",
    scryfall_id = "69d0f5bd-ccea-49b2-bd79-ad5e4d850cf5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Wishclaw Talisman",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
