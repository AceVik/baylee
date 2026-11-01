//! Fog Elemental — {2}{U} — Creature — Elemental
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: When this creature attacks or blocks, sacrifice it at end of combat.
//! Set: 10E #85 — Tenth Edition | Scryfall ID: 95534175-0bf8-4460-8182-117dc1487dcb | Oracle ID: b304fcc0-899e-433f-b6df-5cea504dc4ba
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOG_ELEMENTAL,
    oracle_id = "b304fcc0-899e-433f-b6df-5cea504dc4ba",
    scryfall_id = "95534175-0bf8-4460-8182-117dc1487dcb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fog Elemental",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(4),
        toughness = Some(4),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::SacrificeSelf],
            }],
        ),
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::SacrificeSelf],
            }],
        ),
    ],
);
