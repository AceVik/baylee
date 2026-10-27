//! Decree of Justice — {X}{X}{2}{W}{W} — Sorcery
//! Oracle: Create X 4/4 white Angel creature tokens with flying.
//! Oracle: Cycling {2}{W} ({2}{W}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may pay {X}. If you do, create X 1/1 white Soldier creature tokens.
//! Set: MH3 #263 — Modern Horizons 3 | Scryfall ID: 21637fbe-d9d3-4d5b-a361-0687385d4738 | Oracle ID: 67a98359-6f02-4556-ad47-c01b643686d6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECREE_OF_JUSTICE,
    oracle_id = "67a98359-6f02-4556-ad47-c01b643686d6",
    scryfall_id = "21637fbe-d9d3-4d5b-a361-0687385d4738",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Decree of Justice",
        mana_cost = mana!("{X}{X}{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
