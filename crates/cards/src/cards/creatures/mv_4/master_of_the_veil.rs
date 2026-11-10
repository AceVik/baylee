//! Master of the Veil — {2}{U}{U} — Creature — Human Wizard
//! Oracle: Morph {2}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, you may turn target creature with a morph ability face down.
//! Set: LGN #43 — Legions | Scryfall ID: d7ce1755-9f4a-4741-b6e5-288595ec494d | Oracle ID: ee9104d5-3583-4549-8dd0-5b69274f4b4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MASTER_OF_THE_VEIL,
    oracle_id = "ee9104d5-3583-4549-8dd0-5b69274f4b4f",
    scryfall_id = "d7ce1755-9f4a-4741-b6e5-288595ec494d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Master of the Veil",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
