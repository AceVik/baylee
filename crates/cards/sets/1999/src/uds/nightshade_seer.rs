//! Nightshade Seer — {3}{B} — Creature — Human Wizard
//! Oracle: {2}{B}, {T}: Reveal any number of black cards in your hand. Target creature gets -X/-X until end of turn, where X is the number of cards revealed this way.
//! Set: UDS #63 — Urza's Destiny | Scryfall ID: e2262467-1354-4aec-84a2-21916c44b9ef | Oracle ID: 9bcf66d0-8a4d-4c57-b4de-4cc35399465e
// NOT SUPPORTED: variable-count hand reveal (reveal any number of black cards from your hand and scale -X/-X by the number revealed) is unsupported; plays as a 1/1 Human Wizard body
// PARTIAL: {3}{B} 1/1 Human Wizard creature body implemented; missing "{2}{B}, {T}: Reveal any number of black cards in your hand. Target creature gets -X/-X until end of turn, where X is the number of cards revealed this way" because the DSL has no cost or effect to reveal any number of matching cards from your hand and count them.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NIGHTSHADE_SEER,
    oracle_id = "9bcf66d0-8a4d-4c57-b4de-4cc35399465e",
    scryfall_id = "e2262467-1354-4aec-84a2-21916c44b9ef",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "variable-count hand reveal (reveal any number of black cards from your hand and scale -X/-X by the number revealed) is unsupported; plays as a 1/1 Human Wizard body"
    ),
    faces = &[face!(
        name = "Nightshade Seer",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);
