//! Goblin Grenadiers — {3}{R} — Creature — Goblin
//! Oracle: Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, destroy target creature and target land.
//! Set: WTH #104 — Weatherlight | Scryfall ID: 5a73db23-727f-4d63-97d7-2ca542276722 | Oracle ID: be83a518-cfab-46b1-9fe9-7e3d54311c37
// PARTIAL — The optional sacrifice/destroy trigger is omitted because its mandatory creature and land targets cannot both be destroyed; the 2/2 body remains.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_GRENADIERS,
    oracle_id = "be83a518-cfab-46b1-9fe9-7e3d54311c37",
    scryfall_id = "5a73db23-727f-4d63-97d7-2ca542276722",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "The optional sacrifice/destroy trigger is omitted because its mandatory creature and land targets cannot both be destroyed; the 2/2 body remains."
    ),
    faces = &[face!(
        name = "Goblin Grenadiers",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: You may sacrifice it to destroy target creature AND target land.
    // Keeping creature destruction alone removes the mandatory land target and its consequences.
);
