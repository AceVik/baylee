//! City of Solitude — {2}{G} — Enchantment
//! Oracle: Players can cast spells and activate abilities only during their own turns.
//! Set: VIS #102 — Visions | Scryfall ID: be499b81-bb2d-4f1d-9deb-c8bfcdca8e13 | Oracle ID: 671458ac-7441-4332-9e59-a42ec8b62f31
// PARTIAL — resolves as an enchantment; no static restriction allowing players to cast spells and activate abilities only during their own turns exists yet.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITY_OF_SOLITUDE,
    oracle_id = "671458ac-7441-4332-9e59-a42ec8b62f31",
    scryfall_id = "be499b81-bb2d-4f1d-9deb-c8bfcdca8e13",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "resolves as an enchantment; no static restriction allowing players to cast spells and activate abilities only during their own turns exists yet",
    ),
    faces = &[face!(
        name = "City of Solitude",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
