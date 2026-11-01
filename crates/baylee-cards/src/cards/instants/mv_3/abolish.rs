//! Abolish — {1}{W}{W} — Instant
//! Oracle: You may discard a Plains card rather than pay this spell's mana cost.
//! Oracle: Destroy target artifact or enchantment.
//! Set: DDF #29 — Duel Decks: Elspeth vs. Tezzeret | Scryfall ID: 842e7e6a-c89a-463d-a6e8-743a953c827c | Oracle ID: d6d7faa8-ecc9-408b-9c46-a0f62c74f567
// PARTIAL — casts for {1}{W}{W} to destroy target artifact or enchantment.
// NOT SUPPORTED: alternative cost discarding a Plains card (`CardDef` has no general alternative-cost field).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABOLISH,
    oracle_id = "d6d7faa8-ecc9-408b-9c46-a0f62c74f567",
    scryfall_id = "842e7e6a-c89a-463d-a6e8-743a953c827c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage =
        Coverage::Partial("CardDef has no alternative-cost field for discarding a Plains card",),
    faces = &[face!(
        name = "Abolish",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        )))
    ),],
);
