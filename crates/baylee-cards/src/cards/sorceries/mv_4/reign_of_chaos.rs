//! Reign of Chaos — {2}{R}{R} — Sorcery
//! Oracle: Choose one —
//! Oracle: • Destroy target Plains and target white creature.
//! Oracle: • Destroy target Island and target blue creature.
//! Set: MIR #190 — Mirage | Scryfall ID: 9285b14a-fc8e-457a-b803-202e05be41e5 | Oracle ID: 436b5989-9b8c-4e03-a82e-046837c891ca
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REIGN_OF_CHAOS,
    oracle_id = "436b5989-9b8c-4e03-a82e-046837c891ca",
    scryfall_id = "9285b14a-fc8e-457a-b803-202e05be41e5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Reign of Chaos",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
