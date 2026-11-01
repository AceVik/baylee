//! Torrent of Lava — {X}{R}{R} — Sorcery
//! Oracle: Torrent of Lava deals X damage to each creature without flying.
//! Oracle: As long as Torrent of Lava is on the stack, each creature has "{T}: Prevent the next 1 damage that would be dealt to this creature by Torrent of Lava this turn."
//! Set: MIR #199 — Mirage | Scryfall ID: 19528a24-4968-4742-a2d1-06f94e60f290 | Oracle ID: f467ec85-ffe2-4963-82e0-509a7873fd63
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TORRENT_OF_LAVA,
    oracle_id = "f467ec85-ffe2-4963-82e0-509a7873fd63",
    scryfall_id = "19528a24-4968-4742-a2d1-06f94e60f290",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Torrent of Lava",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
