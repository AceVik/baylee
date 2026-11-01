//! Arrogant Wurm — {3}{G}{G} — Creature — Wurm
//! Oracle: Trample
//! Oracle: Madness {2}{G} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: VMA #195 — Vintage Masters | Scryfall ID: 2dd6977e-269a-4cd1-99ad-37846a0647a7 | Oracle ID: dc7f0c33-80ea-453e-a237-473f077f038e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARROGANT_WURM,
    oracle_id = "dc7f0c33-80ea-453e-a237-473f077f038e",
    scryfall_id = "2dd6977e-269a-4cd1-99ad-37846a0647a7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Arrogant Wurm",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
