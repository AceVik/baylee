//! Spirit of the Labyrinth — {1}{W} — Enchantment Creature — Spirit
//! Oracle: Each player can't draw more than one card each turn.
//! Set: BNG #27 — Born of the Gods | Scryfall ID: f44e5128-e146-4e46-b313-a40d82719d1d | Oracle ID: 1463795b-ec0c-44d6-ae1a-55f78d9843ec
// IMPLEMENTED — one static: every player's draws are capped at one a turn
// (CR 121.2b), the controller's included, which is what "each player" says.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPIRIT_OF_THE_LABYRINTH,
    oracle_id = "1463795b-ec0c-44d6-ae1a-55f78d9843ec",
    scryfall_id = "f44e5128-e146-4e46-b313-a40d82719d1d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spirit of the Labyrinth",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::DrawLimitPerTurn {
            who: PlayerRel::EachPlayer,
            limit: 1,
        }
    )],
);
