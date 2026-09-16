//! Past in Flames — {3}{R} — Sorcery
//! Oracle: Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
//! Oracle: Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: MM3 #105 — Modern Masters 2017 | Scryfall ID: 2b7472f4-37b0-439f-b4ac-80706d40d191 | Oracle ID: 37a18736-5fe2-4897-809b-013497bdd890
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PAST_IN_FLAMES,
    oracle_id = "37a18736-5fe2-4897-809b-013497bdd890",
    scryfall_id = "2b7472f4-37b0-439f-b4ac-80706d40d191",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Past in Flames",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
