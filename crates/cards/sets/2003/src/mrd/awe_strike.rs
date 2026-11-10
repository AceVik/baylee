//! Awe Strike — {W} — Instant
//! Oracle: The next time target creature would deal damage this turn, prevent that damage. You gain life equal to the damage prevented this way.
//! Set: MRD #6 — Mirrodin | Scryfall ID: a5cd231b-7891-42b5-b5de-5112d1230c37 | Oracle ID: 994bcde3-4a76-403d-a6e1-88609a13a99c
use baylee_cards_dsl::prelude::*;

card!(
    index = index::AWE_STRIKE,
    oracle_id = "994bcde3-4a76-403d-a6e1-88609a13a99c",
    scryfall_id = "a5cd231b-7891-42b5-b5de-5112d1230c37",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Awe Strike",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PreventNextDamageFromTarget { gain_life: true }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
