//! Fyndhorn Druid — {2}{G} — Creature — Elf Druid
//! Oracle: When this creature dies, if it was blocked this turn, you gain 4 life.
//! Set: ALL #90a — Alliances | Scryfall ID: 778b028f-fa4e-4638-82b4-fb287223ea20 | Oracle ID: 98e51f1a-ed6e-4286-a41b-e4656d08c934
// IMPLEMENTED — gains 4 life on dying after being blocked earlier that turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FYNDHORN_DRUID,
    oracle_id = "98e51f1a-ed6e-4286-a41b-e4656d08c934",
    scryfall_id = "778b028f-fa4e-4638-82b4-fb287223ea20",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fyndhorn Druid",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::gain_life(4)],
        condition = Some(Condition::SourceWasBlockedThisTurn)
    )],
);
