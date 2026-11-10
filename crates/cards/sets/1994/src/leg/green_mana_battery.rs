//! Green Mana Battery — {4} — Artifact
//! Oracle: {2}, {T}: Put a charge counter on this artifact.
//! Oracle: {T}, Remove any number of charge counters from this artifact: Add {G}, then add an additional {G} for each charge counter removed this way.
//! Set: 4ED #323 — Fourth Edition | Scryfall ID: d0a6e224-72df-4f1f-93d1-5114779aed2c | Oracle ID: b953d19f-6ba7-4aee-b353-c60dc9572610
// IMPLEMENTED — {2}, {T}: a charge counter; {T} removing any number of
// charge counters adds {G} plus one more {G} per counter removed, the
// removal read back as `Amount::X`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREEN_MANA_BATTERY,
    oracle_id = "b953d19f-6ba7-4aee-b353-c60dc9572610",
    scryfall_id = "d0a6e224-72df-4f1f-93d1-5114779aed2c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Green Mana Battery",
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
                ManaColor::Green,
                Amount::Plus {
                    base: &Amount::X,
                    offset: 1,
                }
            )],
        ),
    ],
);
