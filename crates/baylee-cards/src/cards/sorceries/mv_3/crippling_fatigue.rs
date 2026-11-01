//! Crippling Fatigue — {1}{B}{B} — Sorcery
//! Oracle: Target creature gets -2/-2 until end of turn.
//! Oracle: Flashback—{1}{B}, Pay 3 life. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: TOR #58 — Torment | Scryfall ID: 0bb53a87-ba48-4c77-b284-3be321c8836e | Oracle ID: d87190d0-bda3-4ad9-84b2-019f751999ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRIPPLING_FATIGUE,
    oracle_id = "d87190d0-bda3-4ad9-84b2-019f751999ce",
    scryfall_id = "0bb53a87-ba48-4c77-b284-3be321c8836e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Crippling Fatigue",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
