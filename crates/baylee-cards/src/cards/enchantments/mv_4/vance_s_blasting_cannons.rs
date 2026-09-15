//! Vance's Blasting Cannons // Spitfire Bastion — {3}{R} — Legendary Enchantment // Legendary Land
//! Oracle: At the beginning of your upkeep, exile the top card of your library. If it's a nonland card, you may cast that card this turn.
//! Oracle: Whenever you cast your third spell in a turn, you may transform Vance's Blasting Cannons.
//! Oracle: (Transforms from Vance's Blasting Cannons.)
//! Oracle: {T}: Add {R}.
//! Oracle: {2}{R}, {T}: Spitfire Bastion deals 3 damage to any target.
//! Set: XLN #173 — Ixalan | Scryfall ID: 9e8c0009-787f-480b-84b6-bf297f1fb466 | Oracle ID: 5e7eca9c-a7b8-4b7b-a0a0-e8937530145a
//! Face: Vance's Blasting Cannons — {3}{R} — Legendary Enchantment
//! Face: Spitfire Bastion —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 17334,
    oracle_id = "5e7eca9c-a7b8-4b7b-a0a0-e8937530145a",
    scryfall_id = "9e8c0009-787f-480b-84b6-bf297f1fb466",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Vance's Blasting Cannons",
            mana_cost = mana!("{3}{R}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Spitfire Bastion",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
