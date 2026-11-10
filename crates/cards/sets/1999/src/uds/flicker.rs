//! Flicker — {1}{W} — Sorcery
//! Oracle: Exile target nontoken permanent, then return it to the battlefield under its owner's control.
//! Set: UDS #9 — Urza's Destiny | Scryfall ID: f55e7ec5-6488-483f-8020-b48e1a951f09 | Oracle ID: 19ea326f-45ff-4bc0-a957-7d3f22dd8c6f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NONTOKEN_PERMANENT: Filter = Filter::Not(&Filter::IsToken);

card!(
    index = index::FLICKER,
    oracle_id = "19ea326f-45ff-4bc0-a957-7d3f22dd8c6f",
    scryfall_id = "f55e7ec5-6488-483f-8020-b48e1a951f09",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flicker",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::blink_to_owner(TargetSpec::Object(
            &NONTOKEN_PERMANENT,
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(&NONTOKEN_PERMANENT))),
    )],
);
