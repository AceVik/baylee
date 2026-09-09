//! Banishing Stroke — {5}{W} — Instant
//! Oracle: Put target artifact, creature, or enchantment on the bottom of its owner's library.
//! Oracle: Miracle {W} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: C18 #63 — Commander 2018 | Scryfall ID: aad93570-b50a-405a-ad73-03f97594061f | Oracle ID: a6898364-c29e-4b97-a500-344efa3ec24a
// IMPLEMENTED — bottom-of-library removal + miracle cast.

static ARTIFACT_CREATURE_ENCHANTMENT: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::ENCHANTMENT]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 11,
    oracle_id: "a6898364-c29e-4b97-a500-344efa3ec24a",
    scryfall_id: "aad93570-b50a-405a-ad73-03f97594061f",
    faces: &[face! {
        name: "Banishing Stroke",
        mana_cost: baylee_core::mana!("{5}{W}"),
        types: TypeSet::INSTANT,
        miracle: Some(baylee_core::mana!("{W}")),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::PutTargetOnBottomOfLibrary], targets: Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_CREATURE_ENCHANTMENT,
        ))))],
}
