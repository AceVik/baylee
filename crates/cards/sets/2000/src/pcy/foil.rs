//! Foil — {2}{U}{U} — Instant
//! Oracle: You may discard an Island card and another card rather than pay this spell's mana cost.
//! Oracle: Counter target spell.
//! Set: UMA #55 — Ultimate Masters | Scryfall ID: e8b39fd6-9240-4f76-b12c-e7d9aa88f061 | Oracle ID: 7dde4eb6-e9d7-4259-abc2-3af738e0f00f
// IMPLEMENTED — discard-an-Island-and-another-card alternative cost; counter target spell.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOIL,
    oracle_id = "7dde4eb6-e9d7-4259-abc2-3af738e0f00f",
    scryfall_id = "e8b39fd6-9240-4f76-b12c-e7d9aa88f061",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Foil",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(
                Discard(&Filter::HasSubtype(subtypes::land::ISLAND)),
                Discard(&Filter::Any),
            ),
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    ),],
);
