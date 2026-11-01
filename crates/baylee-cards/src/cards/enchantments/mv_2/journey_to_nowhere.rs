//! Journey to Nowhere — {1}{W} — Enchantment
//! Oracle: When this enchantment enters, exile target creature.
//! Oracle: When this enchantment leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: CMD #17 — Commander 2011 | Scryfall ID: 4686b51c-e02b-48c1-bafe-e8d08a5407b9 | Oracle ID: 2bfec482-be0d-4768-84e7-1c2c74ea1373
// IMPLEMENTED — linked exile and return when this enchantment leaves.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JOURNEY_TO_NOWHERE,
    oracle_id = "2bfec482-be0d-4768-84e7-1c2c74ea1373",
    scryfall_id = "4686b51c-e02b-48c1-bafe-e8d08a5407b9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Journey to Nowhere",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedToBattlefield],
        ),
    ],
);
