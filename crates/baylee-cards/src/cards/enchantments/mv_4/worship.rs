//! Worship — {3}{W} — Enchantment
//! Oracle: If you control a creature, damage that would reduce your life total to less than 1 reduces it to 1 instead.
//! Set: USG #57 — Urza's Saga | Scryfall ID: 908781a0-1ba4-4027-bd9d-13f9faf08686 | Oracle ID: 99f9a4c8-b59f-4a41-8577-6c1e4684b240
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORSHIP,
    oracle_id = "99f9a4c8-b59f-4a41-8577-6c1e4684b240",
    scryfall_id = "908781a0-1ba4-4027-bd9d-13f9faf08686",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Worship",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::DamageLifeLossFloor {
            players: PlayerRel::You,
            minimum: 1,
        },
        condition = Some(Condition::ControlCount(&Filter::YOUR_CREATURE, 1)),
    )],
);
