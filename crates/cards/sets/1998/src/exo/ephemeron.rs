//! Ephemeron — {4}{U}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: Discard a card: Return this creature to its owner's hand.
//! Set: TPR #48 — Tempest Remastered | Scryfall ID: 82ebfe9c-a584-41d2-a33e-7aefbe55de0e | Oracle ID: e4f28144-71c8-4393-b383-047529fe8878
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EPHEMERON,
    oracle_id = "e4f28144-71c8-4393-b383-047529fe8878",
    scryfall_id = "82ebfe9c-a584-41d2-a33e-7aefbe55de0e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ephemeron",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(Discard(&Filter::Any)),
        &[Effect::bounce(TargetSpec::ThisObject)],
    )],
);
