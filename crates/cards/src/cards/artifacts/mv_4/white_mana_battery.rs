//! White Mana Battery — {4} — Artifact
//! Oracle: {2}, {T}: Put a charge counter on this artifact.
//! Oracle: {T}, Remove any number of charge counters from this artifact: Add {W}, then add an additional {W} for each charge counter removed this way.
//! Set: 4ED #357 — Fourth Edition | Scryfall ID: b622e694-858d-482f-a67d-0e52d268708c | Oracle ID: cf58682f-c305-4803-b54b-37f0841788e9
// IMPLEMENTED — {2}, {T}: a charge counter; {T} removing any number of
// charge counters adds {W} plus one more {W} per counter removed, the
// removal read back as `Amount::X`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHITE_MANA_BATTERY,
    oracle_id = "cf58682f-c305-4803-b54b-37f0841788e9",
    scryfall_id = "b622e694-858d-482f-a67d-0e52d268708c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "White Mana Battery",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(1),
            }],
        ),
        mana_ability!(
            cost!(
                TapSelf,
                RemoveCounterSelfX {
                    kind: CounterKind::Charge
                }
            ),
            &[Effect::mana_dynamic(
                ManaColor::White,
                Amount::Plus {
                    base: &Amount::X,
                    offset: 1,
                }
            )],
        ),
    ],
);
