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
    abilities = &[mana_ability!(&[Effect::AddMana {
        source: ManaSource::CircledColors,
        amount: Amount::Fixed(1),
        distribution: ManaDistribution::OneColor,
        restriction: None,
    }])],
    coverage = Coverage::Implemented,
);
