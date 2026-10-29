//! Holistic Wisdom — {1}{G}{G} — Enchantment
//! Oracle: {2}, Exile a card from your hand: Return target card from your graveyard to your hand if it shares a card type with the card exiled this way. (Artifact, battle, creature, enchantment, kindred, instant, land, planeswalker, and sorcery are card types.)
//! Set: ODY #243 — Odyssey | Scryfall ID: 2134a564-b089-4049-bd9c-a3f10e072263 | Oracle ID: 7e108285-52da-473c-accd-d48e646a49c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HOLISTIC_WISDOM,
    oracle_id = "7e108285-52da-473c-accd-d48e646a49c0",
    scryfall_id = "2134a564-b089-4049-bd9c-a3f10e072263",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Holistic Wisdom",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
