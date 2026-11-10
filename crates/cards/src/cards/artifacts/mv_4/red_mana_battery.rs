//! Red Mana Battery — {4} — Artifact
//! Oracle: {2}, {T}: Put a charge counter on this artifact.
//! Oracle: {T}, Remove any number of charge counters from this artifact: Add {R}, then add an additional {R} for each charge counter removed this way.
//! Set: 4ED #343 — Fourth Edition | Scryfall ID: e4e507bc-441d-4f44-85d4-cf93ca199d2e | Oracle ID: 8ceb295c-a611-4ac7-940b-557bf60f8a8b
// IMPLEMENTED — {2}, {T}: a charge counter; {T} removing any number of
// charge counters adds {R} plus one more {R} per counter removed, the
// removal read back as `Amount::X`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RED_MANA_BATTERY,
    oracle_id = "8ceb295c-a611-4ac7-940b-557bf60f8a8b",
    scryfall_id = "e4e507bc-441d-4f44-85d4-cf93ca199d2e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Red Mana Battery",
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
                ManaColor::Red,
                Amount::Plus {
                    base: &Amount::X,
                    offset: 1,
                }
            )],
        ),
    ],
);
