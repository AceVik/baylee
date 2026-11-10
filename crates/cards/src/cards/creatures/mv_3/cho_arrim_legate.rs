//! Cho-Arrim Legate — {2}{W} — Creature — Human Soldier
//! Oracle: If an opponent controls a Swamp and you control a Plains, you may cast this spell without paying its mana cost.
//! Oracle: Protection from black
//! Set: MMQ #10 — Mercadian Masques | Scryfall ID: 1427a3a1-24e1-4697-b5eb-1c0a24f89e75 | Oracle ID: 780c9cb2-9d98-4789-87e0-cb81db477204
// PARTIAL — 1/2 Human Soldier with protection from black.
// NOT SUPPORTED: "If an opponent controls a Swamp and you control a Plains, you may cast this spell without paying its mana cost" — `AltCondition` has no condition checking both an opponent's Swamp and your Plains.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));

card!(
    index = index::CHO_ARRIM_LEGATE,
    oracle_id = "780c9cb2-9d98-4789-87e0-cb81db477204",
    scryfall_id = "1427a3a1-24e1-4697-b5eb-1c0a24f89e75",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "conditional free cast (opponent controls Swamp and you control Plains) not supported"
    ),
    faces = &[face!(
        name = "Cho-Arrim Legate",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&BLACK)
    ),],
);
