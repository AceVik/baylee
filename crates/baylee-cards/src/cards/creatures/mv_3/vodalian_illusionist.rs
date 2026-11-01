//! Vodalian Illusionist — {2}{U} — Creature — Merfolk Wizard
//! Oracle: {U}{U}, {T}: Target creature phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)
//! Set: WTH #58 — Weatherlight | Scryfall ID: 9ce0e28b-9fd6-4763-8d6b-952b530358ab | Oracle ID: ac935639-ba30-4b04-86f6-363527e85a8b
// IMPLEMENTED — {U}{U}, {T}: Target creature phases out.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VODALIAN_ILLUSIONIST,
    oracle_id = "ac935639-ba30-4b04-86f6-363527e85a8b",
    scryfall_id = "9ce0e28b-9fd6-4763-8d6b-952b530358ab",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vodalian Illusionist",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{U}{U}", TapSelf),
        &[Effect::PhaseOut {
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
