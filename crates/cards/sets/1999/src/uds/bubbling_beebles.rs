//! Bubbling Beebles — {4}{U} — Creature — Beeble
//! Oracle: This creature can't be blocked as long as defending player controls an enchantment.
//! Set: UDS #29 — Urza's Destiny | Scryfall ID: 002cf7d8-3fc2-48eb-a727-a1ce5a049665 | Oracle ID: 75903c3b-2dbe-4662-b6a8-39599e49adde
// IMPLEMENTED — checks the actual defending player enchantment count.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const DEFENDER_ENCHANTMENT: Filter =
    Filter::And(&[Filter::ENCHANTMENT, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::BUBBLING_BEEBLES,
    oracle_id = "75903c3b-2dbe-4662-b6a8-39599e49adde",
    scryfall_id = "002cf7d8-3fc2-48eb-a727-a1ce5a049665",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bubbling Beebles",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEEBLE],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
        condition = Some(Condition::BattlefieldCount(&DEFENDER_ENCHANTMENT, 1)),
    )],
);
