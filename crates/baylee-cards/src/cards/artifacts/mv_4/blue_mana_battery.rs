//! Blue Mana Battery — {4} — Artifact
//! Oracle: {2}, {T}: Put a charge counter on this artifact.
//! Oracle: {T}, Remove any number of charge counters from this artifact: Add {U}, then add an additional {U} for each charge counter removed this way.
//! Set: 4ED #300 — Fourth Edition | Scryfall ID: cd2cb84e-c079-486e-87ad-d188fe38bc76 | Oracle ID: 79e91e8a-cee7-411b-b0a3-d3383c0d4a43
// IMPLEMENTED — {2}, {T}: a charge counter; {T} removing any number of
// charge counters adds {U} plus one more {U} per counter removed, the
// removal read back as `Amount::X`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLUE_MANA_BATTERY,
    oracle_id = "79e91e8a-cee7-411b-b0a3-d3383c0d4a43",
    scryfall_id = "cd2cb84e-c079-486e-87ad-d188fe38bc76",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blue Mana Battery",
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
                ManaColor::Blue,
                Amount::Plus {
                    base: &Amount::X,
                    offset: 1,
                }
            )],
        ),
    ],
);
