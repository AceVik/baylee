//! Mizzix's Mastery — {3}{R} — Sorcery
//! Oracle: Exile target card that's an instant or sorcery from your graveyard. For each card exiled this way, copy it, and you may cast the copy without paying its mana cost. Exile Mizzix's Mastery.
//! Oracle: Overload {5}{R}{R}{R} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: OTC #175 — Outlaws of Thunder Junction Commander | Scryfall ID: 4fa2d7f2-05b3-468f-9f2c-a61b46bad88e | Oracle ID: 40362fe0-a1a9-4d76-8c35-eac474b91af5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIZZIX_S_MASTERY,
    oracle_id = "40362fe0-a1a9-4d76-8c35-eac474b91af5",
    scryfall_id = "4fa2d7f2-05b3-468f-9f2c-a61b46bad88e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mizzix's Mastery",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
