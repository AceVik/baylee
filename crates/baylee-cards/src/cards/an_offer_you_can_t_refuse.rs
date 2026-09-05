//! An Offer You Can't Refuse — {U} — Instant
//! Oracle: Counter target noncreature spell. Its controller creates two Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: FDN #160 — Foundations | Scryfall ID: a829747f-cf9b-4d81-ba66-9f0630ed4565 | Oracle ID: 234a734b-ba28-4f1b-9d01-3c3e7d516590
// IMPLEMENTED — counter, then two Treasures for the *countered spell's*
// controller, not for you. Two effects rather than one with an amount,
// because `CreateTokenForTargetController` makes one token; the pair reads
// the target of the same resolution, which survives the counter above it as
// a card in its owner's graveyard.

use baylee_cards_dsl::prelude::*;

use crate::tokens::TREASURE as TREASURE_TOKEN;

/// "target noncreature spell" — the same filter Negate targets with.
static NONCREATURE_SPELL: Filter = Filter::NONCREATURE;

card! {
    index: 1344,
    oracle_id: "234a734b-ba28-4f1b-9d01-3c3e7d516590",
    scryfall_id: "a829747f-cf9b-4d81-ba66-9f0630ed4565",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "An Offer You Can't Refuse",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::INSTANT,
    },
    ],
    abilities: &[spell!(
        &[
            Effect::CounterTargetSpell,
            Effect::CreateTokenForTargetController {
                token: &TREASURE_TOKEN,
            },
            Effect::CreateTokenForTargetController {
                token: &TREASURE_TOKEN,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Spell(&NONCREATURE_SPELL)))
    )],
}
