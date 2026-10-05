//! Wood Elemental — {3}{G} — Creature — Elemental
//! Oracle: As this creature enters, sacrifice any number of untapped Forests.
//! Oracle: Wood Elemental's power and toughness are each equal to the number of Forests sacrificed as it entered.
//! Set: ME4 #175 — Masters Edition IV | Scryfall ID: 71ced69c-921c-4a31-a213-0faf927134ef | Oracle ID: 2b6da458-e075-4696-a74f-c8846e0cc370
// PARTIAL — both abilities are off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WOOD_ELEMENTAL,
    oracle_id = "2b6da458-e075-4696-a74f-c8846e0cc370",
    scryfall_id = "71ced69c-921c-4a31-a213-0faf927134ef",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no EnterModifier sacrifices a chosen number as the creature enters \
         and no PtCount reads what was sacrificed that way"
    ),
    faces = &[face!(
        name = "Wood Elemental",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(0),
    ),],
    // NOT SUPPORTED: "As this creature enters, sacrifice any number of
    // untapped Forests." — `FaceDef::enter_modifiers` has no optional
    // sacrifice choice: its `EnterModifier` list carries entry choices and
    // counters, and nothing in it moves a permanent.
    // NOT SUPPORTED: "Wood Elemental's power and toughness are each equal to
    // the number of Forests sacrificed as it entered." — `PtCount` counts
    // permanents on the battlefield, card types in graveyards or cards exiled
    // with the source, and nothing remembers the lands sacrificed to its own
    // entry.
    abilities = &[],
);
