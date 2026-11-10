//! Interdict — {1}{U} — Instant
//! Oracle: Counter target activated ability from an artifact, creature, enchantment, or land. That permanent's activated abilities can't be activated this turn. (Mana abilities can't be targeted.)
//! Oracle: Draw a card.
//! Set: TMP #69 — Tempest | Scryfall ID: 3442c919-73b9-4d29-a014-87293f456325 | Oracle ID: 93771480-2672-4bc7-81a8-011d3b8878b0
// PARTIAL — Counters target activated ability from an artifact, creature, enchantment, or land and draws a card; preventing that permanent's activated abilities from being activated for the rest of the turn is not yet modeled.

use baylee_cards_dsl::prelude::*;

const PERMANENT_SOURCE: Filter = Filter::Or(&[
    Filter::ARTIFACT,
    Filter::CREATURE,
    Filter::ENCHANTMENT,
    Filter::LAND,
]);

card!(
    index = index::INTERDICT,
    oracle_id = "93771480-2672-4bc7-81a8-011d3b8878b0",
    scryfall_id = "3442c919-73b9-4d29-a014-87293f456325",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Counters target activated ability from an artifact, creature, enchantment, or land and draws a card; preventing that permanent's activated abilities from being activated for the rest of the turn is not yet modeled."
    ),
    faces = &[face!(
        name = "Interdict",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetAbility, Effect::draw(1)],
        targets = Some(TargetReq::one(TargetSpec::AbilityOnStack(
            &Filter::ActivatedAbilityFrom(&PERMANENT_SOURCE)
        ))),
    )],
);
