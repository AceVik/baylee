//! Krosan Reclamation — {1}{G} — Instant
//! Oracle: Target player shuffles up to two target cards from their graveyard into their library.
//! Oracle: Flashback {1}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: JUD #122 — Judgment | Scryfall ID: 5b3c5144-7e15-46c6-b819-d729ecb30bb1 | Oracle ID: e69252a6-8c27-4af9-8cbe-11eb416a6a46
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KROSAN_RECLAMATION,
    oracle_id = "e69252a6-8c27-4af9-8cbe-11eb416a6a46",
    scryfall_id = "5b3c5144-7e15-46c6-b819-d729ecb30bb1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Reclamation",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
