//! Brine Hag — {2}{U}{U} — Creature — Hag
//! Oracle: When this creature dies, the base power and toughness of each creature that dealt damage to it this turn become 0/2. (This effect lasts indefinitely.)
//! Set: LEG #49 — Legends | Scryfall ID: 2a1e7796-fbfb-4976-879f-bb748429d5c7 | Oracle ID: e0165326-f1a6-4fc0-94c1-4b33f24d36f5
// PARTIAL — the dies trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRINE_HAG,
    oracle_id = "e0165326-f1a6-4fc0-94c1-4b33f24d36f5",
    scryfall_id = "2a1e7796-fbfb-4976-879f-bb748429d5c7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no Filter names the creatures that dealt damage to the source this turn"
    ),
    faces = &[face!(
        name = "Brine Hag",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HAG],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: "When this creature dies, the base power and toughness
    // of each creature that dealt damage to it this turn become 0/2." — the
    // effect itself is sayable (`Effect::SetPTFilter` at 0/2 with
    // `Duration::Indefinitely`), but no `Filter` names the creatures that
    // dealt damage to this one this turn, and the engine keeps no such
    // per-object damage history for a filter to read;
    // `Trigger::DiesAfterDamageByThis` is the same relation in the other
    // direction, firing on the death of a creature this one damaged.
    abilities = &[],
);
