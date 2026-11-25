//! Blood Tribute — {4}{B}{B} — Sorcery
//! Oracle: Kicker—Tap an untapped Vampire you control. (You may tap a Vampire you control in addition to any other costs as you cast this spell.)
//! Oracle: Target opponent loses half their life, rounded up. If this spell was kicked, you gain life equal to the life lost this way.
//! Set: C17 #100 — Commander 2017 | Scryfall ID: 361912f4-5205-44a5-b21e-17402526c1fd | Oracle ID: aa2da79c-b36c-4120-8d7d-f5571e87e45e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLOOD_TRIBUTE,
    oracle_id = "aa2da79c-b36c-4120-8d7d-f5571e87e45e",
    scryfall_id = "361912f4-5205-44a5-b21e-17402526c1fd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Blood Tribute",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
