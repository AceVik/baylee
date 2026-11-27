//! Timeless Witness — {2}{G}{G} — Creature — Human Shaman
//! Oracle: When this creature enters, return target card from your graveyard to your hand.
//! Oracle: Eternalize {5}{G}{G} ({5}{G}{G}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Human Shaman with no mana cost. Eternalize only as a sorcery.)
//! Set: TDC #274 — Tarkir: Dragonstorm Commander | Scryfall ID: 8b63ec86-670b-4acd-b14c-3d90acbde087 | Oracle ID: a52a06aa-56fc-4845-bbcd-a54f16369b34
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIMELESS_WITNESS,
    oracle_id = "a52a06aa-56fc-4845-bbcd-a54f16369b34",
    scryfall_id = "8b63ec86-670b-4acd-b14c-3d90acbde087",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Timeless Witness",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
