//! Black Mana Battery — {4} — Artifact
//! Oracle: {2}, {T}: Put a charge counter on this artifact.
//! Oracle: {T}, Remove any number of charge counters from this artifact: Add {B}, then add an additional {B} for each charge counter removed this way.
//! Set: 4ED #298 — Fourth Edition | Scryfall ID: a81f7a0f-183f-438b-b252-738d8d30c245 | Oracle ID: dc3b2bab-755d-4b2f-97c0-70a74d721a79
// IMPLEMENTED — charge counter storage; the second ability adds {B} plus one
// more {B} per counter removed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLACK_MANA_BATTERY,
    oracle_id = "dc3b2bab-755d-4b2f-97c0-70a74d721a79",
    scryfall_id = "a81f7a0f-183f-438b-b252-738d8d30c245",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Black Mana Battery",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(1)
            }]
        ),
        mana_ability!(
            cost!(
                TapSelf,
                RemoveCounterSelfX {
                    kind: CounterKind::Charge
                }
            ),
            &[Effect::mana_dynamic(
                ManaColor::Black,
                Amount::Plus {
                    base: &Amount::X,
                    offset: 1
                }
            )]
        ),
    ],
);
