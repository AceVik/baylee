//! Brine Hag — {2}{U}{U} — Creature — Hag
//! Oracle: When this creature dies, the base power and toughness of each creature that dealt damage to it this turn become 0/2. (This effect lasts indefinitely.)
//! Set: LEG #49 — Legends | Scryfall ID: 2a1e7796-fbfb-4976-879f-bb748429d5c7 | Oracle ID: e0165326-f1a6-4fc0-94c1-4b33f24d36f5
// IMPLEMENTED — the dies trigger makes each creature that dealt damage to
// this one this turn a base 0/2, indefinitely.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Each creature that dealt damage to it this turn", as the creature that
/// dealt it.
static DEALT_DAMAGE_TO_IT: Filter =
    Filter::And(&[Filter::CREATURE, Filter::DealtDamageToSourceThisTurn]);

card!(
    index = index::BRINE_HAG,
    oracle_id = "e0165326-f1a6-4fc0-94c1-4b33f24d36f5",
    scryfall_id = "2a1e7796-fbfb-4976-879f-bb748429d5c7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brine Hag",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HAG],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::SetPTFilter {
            filter: &DEALT_DAMAGE_TO_IT,
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(2),
            duration: Duration::Indefinitely,
        }]
    )],
);
