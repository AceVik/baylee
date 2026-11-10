//! Archangel of Tithes — {1}{W}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: As long as this creature is untapped, creatures can't attack you or planeswalkers you control unless their controller pays {1} for each of those creatures.
//! Oracle: As long as this creature is attacking, creatures can't block unless their controller pays {1} for each of those creatures.
//! Set: OTJ #2 — Outlaws of Thunder Junction | Scryfall ID: c853d04c-864b-491c-8c6f-72d2d4874d2f | Oracle ID: ff5caed4-0276-476d-8fae-edae2536df7f
// IMPLEMENTED — flying and both conditionally active combat mana taxes.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCHANGEL_OF_TITHES,
    oracle_id = "ff5caed4-0276-476d-8fae-edae2536df7f",
    scryfall_id = "c853d04c-864b-491c-8c6f-72d2d4874d2f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Archangel of Tithes",
        mana_cost = mana!("{1}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(5),
    ),],
    abilities = &[
        static_ability!(
            Filter::CREATURE,
            Modifier::AttackTaxMana { mana: 1 },
            condition = Some(Condition::SourceMatches(&Filter::Untapped))
        ),
        static_ability!(
            Filter::CREATURE,
            Modifier::CantBlockUnlessPayMana(1),
            condition = Some(Condition::SourceMatches(&Filter::Attacking))
        ),
    ],
);
