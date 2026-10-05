//! Kismet — {3}{W} — Enchantment
//! Oracle: Artifacts, creatures, and lands your opponents control enter tapped.
//! Set: ME4 #17 — Masters Edition IV | Scryfall ID: f38cdfb1-0437-4afe-a777-60228b8eba69 | Oracle ID: 81fdd1c4-d43b-4f8b-8712-7c2bf45a3e0b
// PARTIAL — the entire sentence is off the card; nothing makes another
// player's permanents enter tapped (see the NOT SUPPORTED line).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KISMET,
    oracle_id = "81fdd1c4-d43b-4f8b-8712-7c2bf45a3e0b",
    scryfall_id = "f38cdfb1-0437-4afe-a777-60228b8eba69",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "no vocabulary makes another player's permanents enter tapped: \
         `EnterModifier` is the entering card's own arrival, and \
         `ReplacementRule` has no enter-tapped variant"
    ),
    faces = &[face!(
        name = "Kismet",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: "Artifacts, creatures, and lands your opponents control
    // enter tapped." — `FaceDef::enter_modifiers` only modifies the arrival of
    // the permanent that carries it, and no `ReplacementRule` (or `Modifier`)
    // reaches another player's entering permanents to tap them, so the
    // sentence cannot be registered anywhere.
    abilities = &[],
);
