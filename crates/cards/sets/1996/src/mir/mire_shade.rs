//! Mire Shade — {1}{B} — Creature — Shade
//! Oracle: {B}, Sacrifice a Swamp: Put a +1/+1 counter on this creature. Activate only as a sorcery.
//! Set: MIR #131 — Mirage | Scryfall ID: 120822b8-02ae-411f-bda5-a774c21db66c | Oracle ID: b77a7f4a-1b3a-440e-977c-1ba39e450581
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const SWAMP: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::SWAMP)]);

card!(
    index = index::MIRE_SHADE,
    oracle_id = "b77a7f4a-1b3a-440e-977c-1ba39e450581",
    scryfall_id = "120822b8-02ae-411f-bda5-a774c21db66c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mire Shade",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHADE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}", Sacrifice(&SWAMP)),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        timing = ActivationTiming::SorcerySpeed,
    )],
);
