//! Kor Chant — {2}{W} — Instant
//! Oracle: All damage that would be dealt this turn to target creature you control by a source of your choice is dealt to another target creature instead.
//! Set: TPR #17 — Tempest Remastered | Scryfall ID: 27e20ee1-ab61-463b-b516-0a883d9c7c39 | Oracle ID: 5b3f6817-5d7a-4d83-ad1d-df75b4e1970b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KOR_CHANT,
    oracle_id = "5b3f6817-5d7a-4d83-ad1d-df75b4e1970b",
    scryfall_id = "27e20ee1-ab61-463b-b516-0a883d9c7c39",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Kor Chant",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
