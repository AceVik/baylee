//! Scornful Egotist — {7}{U} — Creature — Human Wizard
//! Oracle: Morph {U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: SCG #50 — Scourge | Scryfall ID: fec6b189-97e7-4627-9785-a9ce2f1ad89f | Oracle ID: 455cbf35-f340-4f1f-a331-5b082c9034ec
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCORNFUL_EGOTIST,
    oracle_id = "455cbf35-f340-4f1f-a331-5b082c9034ec",
    scryfall_id = "fec6b189-97e7-4627-9785-a9ce2f1ad89f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Scornful Egotist",
        mana_cost = mana!("{7}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
