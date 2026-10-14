//! Leovold's Operative — {2}{G} — Creature — Elf Rogue
//! Oracle: Draft this card face up.
//! Oracle: As you draft a card, you may draft an additional card from that booster pack. If you do, turn this card face down, then pass the next booster pack without drafting a card from it. (You may look at that booster pack.)
//! Set: CN2 #66 — Conspiracy: Take the Crown | Scryfall ID: d41deffe-766f-4a46-8552-73483adc0cfb | Oracle ID: 8fedb2c2-fb13-4af1-b85e-714832562da7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LEOVOLD_S_OPERATIVE,
    oracle_id = "8fedb2c2-fb13-4af1-b85e-714832562da7",
    scryfall_id = "d41deffe-766f-4a46-8552-73483adc0cfb",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Leovold's Operative",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::ROGUE],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
