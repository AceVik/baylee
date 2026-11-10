//! Vampire Hexmage — {B}{B} — Creature — Vampire Shaman
//! Oracle: First strike
//! Oracle: Sacrifice this creature: Remove all counters from target permanent.
//! Set: 2XM #112 — Double Masters | Scryfall ID: 97ec2e55-fa60-4ee1-b9c0-e6b84939697b | Oracle ID: fcb2a95b-cef0-4b29-8b2b-538f380aff2b
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VAMPIRE_HEXMAGE,
    oracle_id = "fcb2a95b-cef0-4b29-8b2b-538f380aff2b",
    scryfall_id = "97ec2e55-fa60-4ee1-b9c0-e6b84939697b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vampire Hexmage",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::RemoveAllCountersTarget],
        target = Some(TargetSpec::Object(&Filter::Any)),
    )],
);
