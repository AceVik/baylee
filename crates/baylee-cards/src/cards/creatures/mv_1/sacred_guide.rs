//! Sacred Guide — {W} — Creature — Human Cleric
//! Oracle: {1}{W}, Sacrifice this creature: Reveal cards from the top of your library until you reveal a white card. Put that card into your hand and exile all other cards revealed this way.
//! Set: TMP #38 — Tempest | Scryfall ID: 7f10c37d-d25a-47d6-83b0-dbe0a9cfc938 | Oracle ID: de9e8b71-897d-4a9d-942b-af10f31e0425

// PARTIAL — Revealing until a white card is found with all other revealed cards exiled is not supported; only the 1/1 body is played.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SACRED_GUIDE,
    oracle_id = "de9e8b71-897d-4a9d-942b-af10f31e0425",
    scryfall_id = "7f10c37d-d25a-47d6-83b0-dbe0a9cfc938",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Revealing until a white card is found with all other revealed cards exiled is not supported; only the 1/1 body is played."
    ),
    faces = &[face!(
        name = "Sacred Guide",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: Revealing until a white card is found with all other revealed cards exiled is not supported; only the 1/1 body is played.
);
