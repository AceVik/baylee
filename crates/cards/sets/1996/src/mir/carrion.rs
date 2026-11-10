//! Carrion — {1}{B}{B} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Create X 0/1 black Insect creature tokens, where X is the sacrificed creature's power.
//! Set: MIR #111 — Mirage | Scryfall ID: 9e6c7750-bc3b-4790-bc9d-88e2cf16881e | Oracle ID: 84ca8d74-95ec-4a9c-8793-7a195ad28be9
// IMPLEMENTED

use crate::generated_tokens::INSECT_0_1_BLACK;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::CARRION,
    oracle_id = "84ca8d74-95ec-4a9c-8793-7a195ad28be9",
    scryfall_id = "9e6c7750-bc3b-4790-bc9d-88e2cf16881e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Carrion",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    abilities = &[spell!(&[Effect::CreateTokenN {
        token: &INSECT_0_1_BLACK,
        amount: Amount::SacrificedPower,
    }])],
);
