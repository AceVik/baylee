//! Mistfolk — {U}{U} — Creature — Illusion
//! Oracle: {U}: Counter target spell that targets this creature.
//! Set: ICE #84 — Ice Age | Scryfall ID: 4f3f4d4e-ca4a-4fba-b9fd-cd1d9457cfa1 | Oracle ID: d57409dc-22fa-40c1-a043-cab94d5f3b5f
// IMPLEMENTED — {U} activated ability countering target spell that targets Mistfolk.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFOLK,
    oracle_id = "d57409dc-22fa-40c1-a043-cab94d5f3b5f",
    scryfall_id = "4f3f4d4e-ca4a-4fba-b9fd-cd1d9457cfa1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mistfolk",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{U}"),
        &[Effect::CounterTargetSpell],
        target = Some(TargetSpec::Spell(&Filter::SpellTargetingAffected)),
    )],
);
