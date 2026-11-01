//! Academy Researchers — {1}{U}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, you may put an Aura card from your hand onto the battlefield attached to this creature.
//! Set: 10E #63 — Tenth Edition | Scryfall ID: b62ca800-ec24-476e-b56f-834eef9622a9 | Oracle ID: df2d2c52-475b-4057-9cf8-7886700f0fae
// PARTIAL: Enters-the-battlefield trigger putting an Aura card from your hand onto the battlefield attached to Academy Researchers is not supported by the DSL; enters as a 2/2 Human Wizard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ACADEMY_RESEARCHERS,
    oracle_id = "df2d2c52-475b-4057-9cf8-7886700f0fae",
    scryfall_id = "b62ca800-ec24-476e-b56f-834eef9622a9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Enters-the-battlefield trigger putting an Aura card from your hand onto the battlefield attached to Academy Researchers is not supported by the DSL; enters as a 2/2 Human Wizard.",
    ),
    faces = &[face!(
        name = "Academy Researchers",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);
