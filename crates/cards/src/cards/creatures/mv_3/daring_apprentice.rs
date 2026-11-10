//! Daring Apprentice — {1}{U}{U} — Creature — Human Wizard
//! Oracle: {T}, Sacrifice this creature: Counter target spell.
//! Set: 9ED #72 — Ninth Edition | Scryfall ID: 6c5becbf-77ac-4bd2-912d-7f7c944c2a0b | Oracle ID: 863ad4b8-094b-46b3-b6cd-0640e1f01d6d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARING_APPRENTICE,
    oracle_id = "863ad4b8-094b-46b3-b6cd-0640e1f01d6d",
    scryfall_id = "6c5becbf-77ac-4bd2-912d-7f7c944c2a0b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Daring Apprentice",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::CounterTargetSpell],
        target = Some(TargetSpec::Spell(&Filter::Any)),
    )],
);
