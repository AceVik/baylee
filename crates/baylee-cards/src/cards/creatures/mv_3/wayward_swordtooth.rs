//! Wayward Swordtooth — {2}{G} — Creature — Dinosaur
//! Oracle: Ascend (If you control ten or more permanents, you get the city's blessing for the rest of the game.)
//! Oracle: You may play an additional land on each of your turns.
//! Oracle: This creature can't attack or block unless you have the city's blessing.
//! Set: LCC #263 — The Lost Caverns of Ixalan Commander | Scryfall ID: 95a5d742-187a-4eba-82e4-7a4cc5c4e6f3 | Oracle ID: 3875aef0-3102-4fbf-be90-e4139f7a2348
// PARTIAL — the extra land drop is built (Modifier::ExtraLandDrops); ascend and
// the city's blessing it gates on have no variant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WAYWARD_SWORDTOOTH,
    oracle_id = "3875aef0-3102-4fbf-be90-e4139f7a2348",
    scryfall_id = "95a5d742-187a-4eba-82e4-7a4cc5c4e6f3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "ascend grants no city's blessing the DSL can record, so the land-drop clause is all that is built"
    ),
    faces = &[face!(
        name = "Wayward Swordtooth",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DINOSAUR],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        // NOT SUPPORTED: "Ascend (If you control ten or more permanents, you
        // get the city's blessing for the rest of the game.)" — there is no
        // ascend keyword, no city's-blessing marker and no Condition that
        // reads one.
        // NOT SUPPORTED: "This creature can't attack or block unless you have
        // the city's blessing." — no Modifier says a creature can't attack or
        // block, and the clause it is gated on is the one above.
        static_ability!(Filter::Any, Modifier::ExtraLandDrops(1)),
    ],
);
