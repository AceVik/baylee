//! Ensnaring Bridge — {3} — Artifact
//! Oracle: Creatures with power greater than the number of cards in your hand can't attack.
//! Set: 2XM #253 — Double Masters | Scryfall ID: cf825a56-4870-463a-a2ef-eec86be891db | Oracle ID: dade5835-ccaf-45e3-b9a3-46017e0384b3
// PARTIAL: while you have no cards in hand, creatures with power 1 or greater can't attack; comparing creature power against arbitrary non-zero hand sizes is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENSNARING_BRIDGE,
    oracle_id = "dade5835-ccaf-45e3-b9a3-46017e0384b3",
    scryfall_id = "cf825a56-4870-463a-a2ef-eec86be891db",
    coverage = Coverage::Partial(
        "while you have no cards in hand, creatures with power 1 or greater can't attack; comparing creature power against arbitrary non-zero hand sizes is not supported"
    ),
    faces = &[face!(
        name = "Ensnaring Bridge",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::PowerAtLeast(1)]),
        Modifier::AddKeyword(KeywordSet::CANT_ATTACK),
        condition = Some(Condition::HandSizeAtMost(0)),
    )],
);
