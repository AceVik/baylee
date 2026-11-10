//! Balm of Restoration — {2} — Artifact
//! Oracle: {1}, {T}, Sacrifice this artifact: Choose one —
//! Oracle: • You gain 2 life.
//! Oracle: • Prevent the next 2 damage that would be dealt to any target this turn.
//! Set: FEM #82 — Fallen Empires | Scryfall ID: 7f95de4a-7fae-42bc-9660-39ea7685ca02 | Oracle ID: 45a40b4c-1d37-4d87-adf1-abf0ea4c86b4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BALM_OF_RESTORATION,
    oracle_id = "45a40b4c-1d37-4d87-adf1-abf0ea4c86b4",
    scryfall_id = "7f95de4a-7fae-42bc-9660-39ea7685ca02",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Balm of Restoration",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[modal_activated!(
        cost!("{1}", TapSelf, SacrificeSelf),
        &[
            mode!(&[Effect::gain_life(2)]),
            mode!(
                &[Effect::PreventNextDamage {
                    target: TargetSpec::AnyTarget,
                    amount: Amount::Fixed(2),
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
            ),
        ],
    ),],
);
