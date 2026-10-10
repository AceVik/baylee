//! Diamond Valley — (no cost) — Land
//! Oracle: {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
//! Set: ME1 #175 — Masters Edition | Scryfall ID: 37ccc3ab-9875-4bd2-bdbd-e3af5e01d682 | Oracle ID: 84cef34a-c3e1-4059-b4cd-c481938a53a5
// IMPLEMENTED — gains life equal to the sacrificed creature's toughness
// (Amount::SacrificedToughness, as it last existed, CR 608.2h).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIAMOND_VALLEY,
    oracle_id = "84cef34a-c3e1-4059-b4cd-c481938a53a5",
    scryfall_id = "37ccc3ab-9875-4bd2-bdbd-e3af5e01d682",
    faces = &[face!(name = "Diamond Valley", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&Filter::CREATURE)),
        &[Effect::GainLife {
            amount: Amount::SacrificedToughness
        }]
    )],
);
