//! Starlit Sanctum — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W}, {T}, Sacrifice a Cleric creature: You gain life equal to the sacrificed creature's toughness.
//! Oracle: {B}, {T}, Sacrifice a Cleric creature: Target player loses life equal to the sacrificed creature's power.
//! Set: CLB #917 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: f5774836-0140-420a-9a0f-8ba291cc5ca8 | Oracle ID: d16298ac-67bd-4f9d-9979-23c1b7e4b359
// IMPLEMENTED — {T}: Add {C}; the two Cleric sacrifices read the
// sacrificed creature's toughness and power (Amount::SacrificedToughness,
// Amount::SacrificedPower).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "A Cleric creature."
static CLERIC_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::CLERIC),
]);

card!(
    index = index::STARLIT_SANCTUM,
    oracle_id = "d16298ac-67bd-4f9d-9979-23c1b7e4b359",
    scryfall_id = "f5774836-0140-420a-9a0f-8ba291cc5ca8",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(name = "Starlit Sanctum", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{W}", TapSelf, Sacrifice(&CLERIC_CREATURE)),
            &[Effect::GainLife {
                amount: Amount::SacrificedToughness
            }]
        ),
        activated!(
            cost!("{B}", TapSelf, Sacrifice(&CLERIC_CREATURE)),
            &[Effect::LoseLife {
                amount: Amount::SacrificedPower,
                target: PlayerRel::Chosen
            }],
            target = Some(TargetSpec::AnyPlayer)
        ),
    ],
);
