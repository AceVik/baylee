//! Giant Albatross — {1}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: When this creature dies, you may pay {1}{U}. If you do, for each creature that dealt damage to this creature this turn, destroy that creature unless its controller pays 2 life. A creature destroyed this way can't be regenerated.
//! Set: HML #27a — Homelands | Scryfall ID: bce05870-74d3-43f1-92d0-dc1744c0138d | Oracle ID: 8b59cacb-46ff-4400-a7f6-b258c81a9b14
// PARTIAL — plays as a 1/1 flier: the death trigger is left off. "You may pay {1}{U}. If you do, for each creature that dealt damage to this creature this turn, destroy that creature unless its controller pays 2 life" needs each damaging creature's controller asked for 2 life after the mana payment, and no piece says that.

use baylee_cards_dsl::prelude::*;

use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_ALBATROSS,
    oracle_id = "8b59cacb-46ff-4400-a7f6-b258c81a9b14",
    scryfall_id = "bce05870-74d3-43f1-92d0-dc1744c0138d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the death trigger is not played: \"you may pay {1}{U}; if you do, for each creature that dealt damage to this creature this turn, destroy that creature unless its controller pays 2 life\" needs each damaging creature's controller asked for the life after the mana payment, and no piece does that; only flying is played"
    ),
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Giant Albatross",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: "When this creature dies, you may pay {1}{U}. If you do, for each creature that dealt damage to this creature this turn, destroy that creature unless its controller pays 2 life. A creature destroyed this way can't be regenerated." Each damaging creature's controller is asked for 2 life, after the mana payment. `DestroyEachUnlessAnyPlayerPaysLife` lets any player pay, and a `ForEachMatching` whose life question sits inside `PlayerMayPayManaThen` loses the iterated creature when the question nests. A card that destroyed without the question would do more than it prints.
);
