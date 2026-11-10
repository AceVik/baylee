//! Life // Death — {G} — Sorcery // Sorcery
//! Oracle: All lands you control become 1/1 creatures until end of turn. They're still lands.
//! Oracle: Return target creature card from your graveyard to the battlefield. You lose life equal to its mana value.
//! Set: DMR #216 — Dominaria Remastered | Scryfall ID: e16d52ca-f8de-4852-9bff-9d208e5f678f | Oracle ID: d476f4b3-db63-4756-ab76-4b35f63c2825
//! Face: Life — {G} — Sorcery
//! Face: Death — {1}{B} — Sorcery
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFE,
    oracle_id = "d476f4b3-db63-4756-ab76-4b35f63c2825",
    scryfall_id = "e16d52ca-f8de-4852-9bff-9d208e5f678f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[
        face!(
            name = "Life",
            mana_cost = mana!("{G}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Death",
            mana_cost = mana!("{1}{B}"),
            types = TypeSet::SORCERY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
