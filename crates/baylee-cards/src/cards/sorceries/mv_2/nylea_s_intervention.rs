//! Nylea's Intervention — {X}{G}{G} — Sorcery
//! Oracle: Choose one —
//! Oracle: • Search your library for up to X land cards, reveal them, put them into your hand, then shuffle.
//! Oracle: • Nylea's Intervention deals twice X damage to each creature with flying.
//! Set: THB #188 — Theros Beyond Death | Scryfall ID: daa2f963-9d16-4224-b24e-b6a79f2b9d75 | Oracle ID: acf388b2-c4e3-4f1b-a16c-88f991d5c17b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NYLEA_S_INTERVENTION,
    oracle_id = "acf388b2-c4e3-4f1b-a16c-88f991d5c17b",
    scryfall_id = "daa2f963-9d16-4224-b24e-b6a79f2b9d75",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nylea's Intervention",
        mana_cost = mana!("{X}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
