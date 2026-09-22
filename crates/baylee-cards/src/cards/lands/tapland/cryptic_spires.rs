//! Cryptic Spires — (no cost) — Land
//! Oracle: As you create your deck, circle two of the colors below.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of either of the circled colors.
//! Set: 2X2 #332 — Double Masters 2022 | Scryfall ID: 309a6684-ecb3-491c-899a-3aa15a51130b | Oracle ID: 6d6a25fb-0432-4c7d-b0e6-e787ddc71218
// PARTIAL — the enters-tapped clause is built; the circled-colour choice is a
// deck-construction instruction no DSL shape can record, so the mana line is
// left off rather than guessed at.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRYPTIC_SPIRES,
    oracle_id = "6d6a25fb-0432-4c7d-b0e6-e787ddc71218",
    scryfall_id = "309a6684-ecb3-491c-899a-3aa15a51130b",
    faces = &[face!(
        name = "Cryptic Spires",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the two colours are circled while creating the deck, and no enter modifier or \
         mana source names a deck-construction choice",
    ),
);

// NOT SUPPORTED: {T}: Add one mana of either of the circled colors —
// `ManaSource::Chosen`/`ChosenOr` name a colour chosen as the *permanent*
// enters, and nothing in the vocabulary carries two colours fixed while the
// deck was built. `Effect::mana_of_any_color()` would be a strictly better
// card, and the deckbuilder would deal it as playable.
//
// NOT SUPPORTED: As you create your deck, circle two of the colors below —
// deck-construction instructions have no representation at all (CR 103.1).
