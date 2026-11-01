//! Booby Trap — {6} — Artifact
//! Oracle: As this artifact enters, choose an opponent and a card name other than a basic land card name.
//! Oracle: The chosen player reveals each card they draw.
//! Oracle: When the chosen player draws a card with the chosen name, sacrifice this artifact. If you do, this artifact deals 10 damage to that player.
//! Set: 9ED #289 — Ninth Edition | Scryfall ID: 1af0c9a0-0dfa-4245-8b29-7bd37982b7d2 | Oracle ID: 3e46381a-7850-4e50-9213-e1a180299977
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOOBY_TRAP,
    oracle_id = "3e46381a-7850-4e50-9213-e1a180299977",
    scryfall_id = "1af0c9a0-0dfa-4245-8b29-7bd37982b7d2",
    faces = &[face!(
        name = "Booby Trap",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
