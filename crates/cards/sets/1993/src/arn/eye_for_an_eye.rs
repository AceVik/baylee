//! Eye for an Eye — {W}{W} — Instant
//! Oracle: The next time a source of your choice would deal damage to you this turn, instead that source deals that much damage to you and Eye for an Eye deals that much damage to that source's controller.
//! Set: ME4 #12 — Masters Edition IV | Scryfall ID: 7b4b8de1-a548-4398-887e-f95b4c15590c | Oracle ID: 22647b1a-5a7c-41b5-b820-b2e9f49c7aad
// IMPLEMENTED — the next chosen-source damage event still deals its damage
// to you and creates equal damage from this spell to that source's controller.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EYE_FOR_AN_EYE,
    oracle_id = "22647b1a-5a7c-41b5-b820-b2e9f49c7aad",
    scryfall_id = "7b4b8de1-a548-4398-887e-f95b4c15590c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eye for an Eye",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::ReflectNextChosenSourceDamage])],
);
