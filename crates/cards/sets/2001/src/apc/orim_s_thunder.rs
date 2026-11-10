//! Orim's Thunder — {2}{W} — Instant
//! Oracle: Kicker {R} (You may pay an additional {R} as you cast this spell.)
//! Oracle: Destroy target artifact or enchantment. If this spell was kicked, it deals damage equal to that permanent's mana value to target creature.
//! Set: DMR #18 — Dominaria Remastered | Scryfall ID: 5f86bba5-e203-4a86-a415-ce748f6d1f6f | Oracle ID: 380429d5-82db-449c-b9b9-3e82ab987972
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ORIM_S_THUNDER,
    oracle_id = "380429d5-82db-449c-b9b9-3e82ab987972",
    scryfall_id = "5f86bba5-e203-4a86-a415-ce748f6d1f6f",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Orim's Thunder",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
