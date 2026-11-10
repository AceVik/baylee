//! Spinning Darkness — {4}{B}{B} — Instant
//! Oracle: You may exile the top three black cards of your graveyard rather than pay this spell's mana cost.
//! Oracle: Spinning Darkness deals 3 damage to target nonblack creature. You gain 3 life.
//! Set: WTH #81 — Weatherlight | Scryfall ID: 58e64a8e-84b1-416c-9fa7-8b10130dc9e9 | Oracle ID: 69eeee95-cc69-41d1-833b-8895ef72df82
// PARTIAL — Deals 3 damage to target nonblack creature and you gain 3 life; the alternative cost exiling the top three black cards of your graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;

static NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::SPINNING_DARKNESS,
    oracle_id = "69eeee95-cc69-41d1-833b-8895ef72df82",
    scryfall_id = "58e64a8e-84b1-416c-9fa7-8b10130dc9e9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Deals 3 damage to target nonblack creature and you gain 3 life; the alternative cost exiling the top three black cards of your graveyard is not yet modeled.",
    ),
    faces = &[face!(
        name = "Spinning Darkness",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Object(&NONBLACK_CREATURE),
            },
            Effect::GainLife {
                amount: Amount::Fixed(3),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&NONBLACK_CREATURE))),
    ),],
);
