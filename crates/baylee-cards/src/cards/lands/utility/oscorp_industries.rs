//! Oscorp Industries — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters from a graveyard, you lose 2 life.
//! Oracle: {T}: Add {U}, {B}, or {R}.
//! Oracle: Mayhem (You may play this card from your graveyard if you discarded it this turn. Timing rules still apply.)
//! Set: SPM #182 — Marvel's Spider-Man | Scryfall ID: 1e609d6e-9e37-45d2-87de-8c76675f7cec | Oracle ID: f432eb6a-f1bf-4ce7-b915-1488dccb4bb9
// PARTIAL — enters tapped (`EnterModifier::Tapped`) and taps for {U}, {B} or
// {R}; the enters-from-a-graveyard trigger and Mayhem have no DSL shape.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OSCORP_INDUSTRIES,
    oracle_id = "f432eb6a-f1bf-4ce7-b915-1488dccb4bb9",
    scryfall_id = "1e609d6e-9e37-45d2-87de-8c76675f7cec",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Oscorp Industries",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "no trigger carries the zone an entering permanent came from, so \
         \"when this land enters from a graveyard, you lose 2 life\" is \
         dropped; Mayhem (play this card from your graveyard if you \
         discarded it this turn) is not expressible"
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
    ])])],
);

// NOT SUPPORTED: "When this land enters from a graveyard, you lose 2 life." —
// `Trigger` has no variant carrying where an entering permanent came from,
// and `Condition::SourceMatches` reads the source's *current* zone (the
// battlefield), not its origin. A plain `Trigger::ETB` would be a different
// card: it would drain 2 life on every entry, from the hand or the graveyard.
// NOT SUPPORTED: Mayhem — a graveyard-play permission plus a
// discarded-this-turn marker. `Modifier::PlayLandsFromGraveyard` is the
// permission half, but nothing records the discard, and mayhem has no
// `KeywordSet` bit any rule reads.
