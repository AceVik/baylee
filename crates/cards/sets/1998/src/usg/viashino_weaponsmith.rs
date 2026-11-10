//! Viashino Weaponsmith — {3}{R} — Creature — Lizard
//! Oracle: Whenever this creature becomes blocked by a creature, this creature gets +2/+2 until end of turn.
//! Set: USG #226 — Urza's Saga | Scryfall ID: def316ed-b080-4d1d-b946-d7a86ebb8ad9 | Oracle ID: f4d17f41-efd6-4173-ab6e-28b6fbcee10d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_WEAPONSMITH,
    oracle_id = "f4d17f41-efd6-4173-ab6e-28b6fbcee10d",
    scryfall_id = "def316ed-b080-4d1d-b946-d7a86ebb8ad9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viashino Weaponsmith",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&Filter::CREATURE),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
