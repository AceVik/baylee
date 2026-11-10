//! Lightning Surge — {3}{R}{R} — Sorcery
//! Oracle: Lightning Surge deals 4 damage to any target.
//! Oracle: Threshold — If there are seven or more cards in your graveyard, instead Lightning Surge deals 6 damage to that permanent or player and the damage can't be prevented.
//! Oracle: Flashback {5}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: JUD #96 — Judgment | Scryfall ID: 0452d78d-eafc-4ccb-a478-d1f46bcefffe | Oracle ID: dc55e69e-e1b8-4129-902c-c71bcb952418
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIGHTNING_SURGE,
    oracle_id = "dc55e69e-e1b8-4129-902c-c71bcb952418",
    scryfall_id = "0452d78d-eafc-4ccb-a478-d1f46bcefffe",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Lightning Surge",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
