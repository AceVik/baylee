//! Orgg — {3}{R}{R} — Creature — Orgg
//! Oracle: Trample
//! Oracle: This creature can't attack if defending player controls an untapped creature with power 3 or greater.
//! Oracle: This creature can't block creatures with power 3 or greater.
//! Set: TSB #67 — Time Spiral Timeshifted | Scryfall ID: e8961e50-dc20-40f9-86ca-968ba28ad0cc | Oracle ID: f6ea2b1b-1403-4ccd-a23c-48e0e81ceb91
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const POWER_3_OR_MORE: Filter = Filter::And(&[Filter::CREATURE, Filter::PowerAtLeast(3)]);
const UNTAPPED_POWER_3_OR_MORE: Filter = Filter::And(&[POWER_3_OR_MORE, Filter::Untapped]);

card!(
    index = index::ORGG,
    oracle_id = "f6ea2b1b-1403-4ccd-a23c-48e0e81ceb91",
    scryfall_id = "e8961e50-dc20-40f9-86ca-968ba28ad0cc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orgg",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORGG],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantAttackIfDefenderControls(&UNTAPPED_POWER_3_OR_MORE),
        ),
        static_ability!(Filter::This, Modifier::CantBlock(&POWER_3_OR_MORE)),
    ],
);
