//! Branchsnap Lorian — {1}{G}{G} — Creature — Beast
//! Oracle: Trample
//! Oracle: Morph {G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: LGN #118 — Legions | Scryfall ID: 52118ff1-ad76-4b97-9fdc-6adfe80140f8 | Oracle ID: e09b24a0-675b-4c64-ac44-74ddf109598a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRANCHSNAP_LORIAN,
    oracle_id = "e09b24a0-675b-4c64-ac44-74ddf109598a",
    scryfall_id = "52118ff1-ad76-4b97-9fdc-6adfe80140f8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Branchsnap Lorian",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
