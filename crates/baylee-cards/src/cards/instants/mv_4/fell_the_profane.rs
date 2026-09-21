//! Fell the Profane // Fell Mire — {2}{B}{B} — Instant // Land
//! Oracle: Destroy target creature or planeswalker.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: MH3 #244 — Modern Horizons 3 | Scryfall ID: a3cb782d-c459-468d-9779-9b5669abc337 | Oracle ID: 053a69d8-2b5e-4f14-8b02-ca405891dc4a
//! Face: Fell the Profane — {2}{B}{B} — Instant
//! Face: Fell Mire —  — Land
// IMPLEMENTED — the front face destroys a target creature or planeswalker;
// the back face enters tapped unless you pay 3 life, and taps for {B}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FELL_THE_PROFANE,
    oracle_id = "053a69d8-2b5e-4f14-8b02-ca405891dc4a",
    scryfall_id = "a3cb782d-c459-468d-9779-9b5669abc337",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Fell the Profane",
            mana_cost = mana!("{2}{B}{B}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Fell Mire",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(
            &Filter::CREATURE_OR_PLANESWALKER
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::CREATURE_OR_PLANESWALKER
        )))
    )],
);
