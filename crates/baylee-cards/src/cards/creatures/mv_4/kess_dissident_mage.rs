//! Kess, Dissident Mage — {1}{U}{B}{R} — Legendary Creature — Human Wizard
//! Oracle: Flying
//! Oracle: Once during each of your turns, you may cast an instant or sorcery spell from your graveyard. If a spell cast this way would be put into your graveyard, exile it instead.
//! Set: NCC #344 — New Capenna Commander | Scryfall ID: e83e6d7a-3af0-4955-8004-2310f051e306 | Oracle ID: f5092c14-eec4-472c-999c-ba96c36b2fbb
// PARTIAL — flying; once-per-turn graveyard casting permission for instants and sorceries is not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KESS_DISSIDENT_MAGE,
    oracle_id = "f5092c14-eec4-472c-999c-ba96c36b2fbb",
    scryfall_id = "e83e6d7a-3af0-4955-8004-2310f051e306",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "once-per-turn graveyard cast permission for instants and sorceries is not supported"
    ),
    faces = &[face!(
        name = "Kess, Dissident Mage",
        mana_cost = mana!("{1}{U}{B}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[
        // NOT SUPPORTED: Once during each of your turns, you may cast an instant or sorcery spell from your graveyard. If a spell cast this way would be put into your graveyard, exile it instead.
    ],
);
