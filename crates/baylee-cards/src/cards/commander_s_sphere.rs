//! Commander's Sphere — {3} — Artifact
//! Oracle: {T}: Add one mana of any color in your commander's color identity.
//! Oracle: Sacrifice this artifact: Draw a card.
//! Set: ECC #139 — Lorwyn Eclipsed Commander | Scryfall ID: a0d50603-e31f-4cd7-b4b6-49847d99eb2e | Oracle ID: 0b67c4e2-f88b-4e01-85a1-9d5f5b8db13b
// IMPLEMENTED — the same identity mana as Arcane Signet, plus a sacrifice
// that draws. The sacrifice costs no mana and does not tap: "Sacrifice this
// artifact" is the whole cost, so a tapped Sphere still cashes itself in.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1347,
    oracle_id: "0b67c4e2-f88b-4e01-85a1-9d5f5b8db13b",
    scryfall_id: "a0d50603-e31f-4cd7-b4b6-49847d99eb2e",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Commander's Sphere",
        mana_cost: baylee_core::mana!("{3}"),
        types: TypeSet::ARTIFACT,
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana_commander_identity()]),
        activated!(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::SacrificeSelf],
            },
            &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }]
        ),
    ],
}
