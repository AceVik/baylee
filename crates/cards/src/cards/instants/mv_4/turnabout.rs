//! Turnabout — {2}{U}{U} — Instant
//! Oracle: Choose artifact, creature, or land. Tap all untapped permanents of the chosen type target player controls, or untap all tapped permanents of that type that player controls.
//! Set: USG #105 — Urza's Saga | Scryfall ID: 52000066-a8cd-418f-98d2-30354b959b32 | Oracle ID: 715d0777-4d36-463b-9102-6bb6bb00b7e9
// PARTIAL: choosing artifact, creature, or land and choosing to tap or untap all permanents of that type target player controls is not in Effect; taps all creatures target player controls.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TURNABOUT,
    oracle_id = "715d0777-4d36-463b-9102-6bb6bb00b7e9",
    scryfall_id = "52000066-a8cd-418f-98d2-30354b959b32",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "choosing artifact, creature, or land and choosing to tap or untap all permanents of that type target player controls is not in Effect; taps all creatures target player controls",
    ),
    faces = &[face!(
        name = "Turnabout",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: choosing artifact, creature, or land and choosing to tap or untap all permanents of that type target player controls is not in Effect; taps all creatures target player controls.
    abilities = &[spell!(
        &[Effect::TapAllOf {
            who: PlayerRel::Chosen,
            filter: &Filter::CREATURE,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    ),],
);
