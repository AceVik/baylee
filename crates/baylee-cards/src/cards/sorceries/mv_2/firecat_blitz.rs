//! Firecat Blitz — {X}{R}{R} — Sorcery
//! Oracle: Create X 1/1 red Elemental Cat creature tokens with haste. Exile them at the beginning of the next end step.
//! Oracle: Flashback—{R}{R}, Sacrifice X Mountains. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: JUD #88 — Judgment | Scryfall ID: d4e1d485-02d5-4a07-bcc6-d2a8d95763e8 | Oracle ID: 8f262a8b-2553-45c9-b37f-218a5547fcff
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRECAT_BLITZ,
    oracle_id = "8f262a8b-2553-45c9-b37f-218a5547fcff",
    scryfall_id = "d4e1d485-02d5-4a07-bcc6-d2a8d95763e8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Firecat Blitz",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
