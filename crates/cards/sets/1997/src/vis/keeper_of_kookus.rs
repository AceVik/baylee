//! Keeper of Kookus — {R} — Creature — Goblin
//! Oracle: {R}: This creature gains protection from red until end of turn.
//! Set: VIS #85 — Visions | Scryfall ID: d11b6df4-449f-44ea-a4fa-f079bcd26a54 | Oracle ID: 97417270-e525-4261-b7d9-0dde3360e666
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static RED_SOURCE: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

card!(
    index = index::KEEPER_OF_KOOKUS,
    oracle_id = "97417270-e525-4261-b7d9-0dde3360e666",
    scryfall_id = "d11b6df4-449f-44ea-a4fa-f079bcd26a54",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Keeper of Kookus",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::ModifySource {
            modifier: Modifier::ProtectionFrom(&RED_SOURCE),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
