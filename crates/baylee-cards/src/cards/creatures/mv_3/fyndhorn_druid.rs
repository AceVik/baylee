//! Fyndhorn Druid — {2}{G} — Creature — Elf Druid
//! Oracle: When this creature dies, if it was blocked this turn, you gain 4 life.
//! Set: ALL #90a — Alliances | Scryfall ID: 778b028f-fa4e-4638-82b4-fb287223ea20 | Oracle ID: 98e51f1a-ed6e-4286-a41b-e4656d08c934
// NOT SUPPORTED: checking whether this creature was blocked this turn on its dies trigger is not supported; gains 4 life whenever this creature dies regardless of whether it was blocked this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FYNDHORN_DRUID,
    oracle_id = "98e51f1a-ed6e-4286-a41b-e4656d08c934",
    scryfall_id = "778b028f-fa4e-4638-82b4-fb287223ea20",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "checking whether this creature was blocked this turn on its dies trigger is not supported; gains 4 life whenever this creature dies regardless of whether it was blocked this turn",
    ),
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
    )],
);
