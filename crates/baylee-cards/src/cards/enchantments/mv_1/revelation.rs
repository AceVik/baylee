//! Revelation — {G} — World Enchantment
//! Oracle: Players play with their hands revealed.
//! Set: CHR #68 — Chronicles | Scryfall ID: d467517a-1e6f-4c1f-adb5-bf60df1284e2 | Oracle ID: 251ff4e4-3be4-424e-8dd7-eb4ede7b415c
// PARTIAL — no Modifier reveals a hand, so the printed static is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVELATION,
    oracle_id = "251ff4e4-3be4-424e-8dd7-eb4ede7b415c",
    scryfall_id = "d467517a-1e6f-4c1f-adb5-bf60df1284e2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no Modifier reveals a player's hand: Modifier::RevealLibraryTop is \
         the DSL's only reveal and it covers a library's top card"
    ),
    faces = &[face!(
        name = "Revelation",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// NOT SUPPORTED: "Players play with their hands revealed." — the DSL has no
// Modifier that makes a hand public; `Modifier::RevealLibraryTop` reveals a
// library's top card and nothing else.
