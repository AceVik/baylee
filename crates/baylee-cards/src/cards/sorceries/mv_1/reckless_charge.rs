//! Reckless Charge — {R} — Sorcery
//! Oracle: Target creature gets +3/+0 and gains haste until end of turn.
//! Oracle: Flashback {2}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: MH1 #144 — Modern Horizons | Scryfall ID: 1754a8db-060e-470f-94c0-37f12d82978a | Oracle ID: 45bee121-0beb-4901-9473-20e4704ba6dc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECKLESS_CHARGE,
    oracle_id = "45bee121-0beb-4901-9473-20e4704ba6dc",
    scryfall_id = "1754a8db-060e-470f-94c0-37f12d82978a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Reckless Charge",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
