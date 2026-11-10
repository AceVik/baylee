//! Tolarian Emissary — {2}{U} — Creature — Human Wizard
//! Oracle: Kicker {1}{W} (You may pay an additional {1}{W} as you cast this spell.)
//! Oracle: Flying
//! Oracle: When this creature enters, if it was kicked, destroy target enchantment.
//! Set: INV #81 — Invasion | Scryfall ID: 1cbc55e5-b84c-4449-a288-ec26cdd3997c | Oracle ID: 8042ade2-f589-4c3e-a461-a4c284a67ee4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TOLARIAN_EMISSARY,
    oracle_id = "8042ade2-f589-4c3e-a461-a4c284a67ee4",
    scryfall_id = "1cbc55e5-b84c-4449-a288-ec26cdd3997c",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Tolarian Emissary",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
