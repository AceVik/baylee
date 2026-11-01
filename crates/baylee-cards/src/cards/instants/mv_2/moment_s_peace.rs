//! Moment's Peace — {1}{G} — Instant
//! Oracle: Prevent all combat damage that would be dealt this turn.
//! Oracle: Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #251 — Odyssey | Scryfall ID: 40ebe935-ccf9-435e-8fe8-53bcbf3526e7 | Oracle ID: b6d22228-a45e-4296-9ae2-1649e04b1c53
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOMENT_S_PEACE,
    oracle_id = "b6d22228-a45e-4296-9ae2-1649e04b1c53",
    scryfall_id = "40ebe935-ccf9-435e-8fe8-53bcbf3526e7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Moment's Peace",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
