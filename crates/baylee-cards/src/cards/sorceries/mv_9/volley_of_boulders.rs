//! Volley of Boulders — {8}{R} — Sorcery
//! Oracle: Volley of Boulders deals 6 damage divided as you choose among any number of targets.
//! Oracle: Flashback {R}{R}{R}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: DDI #73 — Duel Decks: Venser vs. Koth | Scryfall ID: aecab75c-25e5-4465-9c0a-28c2f6dc2ce5 | Oracle ID: 47860e92-e86a-49f2-80e3-fd144de08b4d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOLLEY_OF_BOULDERS,
    oracle_id = "47860e92-e86a-49f2-80e3-fd144de08b4d",
    scryfall_id = "aecab75c-25e5-4465-9c0a-28c2f6dc2ce5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Volley of Boulders",
        mana_cost = mana!("{8}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
