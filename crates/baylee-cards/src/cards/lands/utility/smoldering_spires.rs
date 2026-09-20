//! Smoldering Spires — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, target creature can't block this turn.
//! Oracle: {T}: Add {R}.
//! Set: E01 #96 — Archenemy: Nicol Bolas | Scryfall ID: e6741f53-f02e-4f1b-9620-e49ad00e7e1d | Oracle ID: cfa3288d-e521-4a13-bcb3-7950a94e1746
// PARTIAL — enters tapped and {T}: Add {R} are written; the enters trigger
// is dropped, see the `// NOT SUPPORTED:` line below.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "When this land enters, target creature can't block this
// turn." No `Modifier` says a creature can't block, and no keyword bit means
// it either — the engine's `unblockable` is the opposite sentence, "this
// creature can't be blocked". The whole triggered ability comes off the card
// rather than shipping an offer that resolves into nothing.

card!(
    index = index::SMOLDERING_SPIRES,
    oracle_id = "cfa3288d-e521-4a13-bcb3-7950a94e1746",
    scryfall_id = "e6741f53-f02e-4f1b-9620-e49ad00e7e1d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Smoldering Spires",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the enters trigger's \"target creature can't block this turn\" has no DSL variant",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);
