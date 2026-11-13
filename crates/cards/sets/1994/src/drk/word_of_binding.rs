//! Word of Binding — {X}{B}{B} — Sorcery
//! Oracle: Tap X target creatures.
//! Set: 4ED #172 — Fourth Edition | Scryfall ID: 715472e1-585b-4379-b335-8f3b5e572d83 | Oracle ID: 8bbc9f21-2b4d-4d03-aef1-f5bf0b79257c

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORD_OF_BINDING,
    oracle_id = "8bbc9f21-2b4d-4d03-aef1-f5bf0b79257c",
    scryfall_id = "715472e1-585b-4379-b335-8f3b5e572d83",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Word of Binding",
        mana_cost = mana!("{X}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::TapTarget],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
