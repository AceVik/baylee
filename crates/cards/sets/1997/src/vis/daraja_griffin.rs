//! Daraja Griffin — {3}{W} — Creature — Griffin
//! Oracle: Flying
//! Oracle: Sacrifice this creature: Destroy target black creature.
//! Set: 6ED #14 — Classic Sixth Edition | Scryfall ID: 0795a1d6-9caf-472a-a349-fca97bccf8e2 | Oracle ID: daa0d720-cac1-487a-93e2-33e18f91f19b
// IMPLEMENTED — flying and sacrifice ability destroying target black creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::DARAJA_GRIFFIN,
    oracle_id = "daa0d720-cac1-487a-93e2-33e18f91f19b",
    scryfall_id = "0795a1d6-9caf-472a-a349-fca97bccf8e2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Daraja Griffin",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GRIFFIN],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::destroy(TargetSpec::Object(&BLACK_CREATURE))],
        target = Some(TargetSpec::Object(&BLACK_CREATURE)),
    )],
);
