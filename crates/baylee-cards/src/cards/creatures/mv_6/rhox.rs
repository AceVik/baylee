//! Rhox — {4}{G}{G} — Creature — Rhino Beast
//! Oracle: You may have this creature assign its combat damage as though it weren't blocked.
//! Oracle: {2}{G}: Regenerate this creature. (The next time this creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: 10E #291 — Tenth Edition | Scryfall ID: 6aa50126-f6ec-4917-ac45-4b1f8f23ca09 | Oracle ID: f6f4c684-2915-4a58-9c4b-487387f7cf73
// NOT SUPPORTED: You may have this creature assign its combat damage as though it weren't blocked — optional unblocked combat-damage assignment is not in the DSL
// PARTIAL — {4}{G}{G} 5/5 Rhino Beast with "{2}{G}: Regenerate this creature" implemented; optional unblocked combat-damage assignment is not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RHOX,
    oracle_id = "f6f4c684-2915-4a58-9c4b-487387f7cf73",
    scryfall_id = "6aa50126-f6ec-4917-ac45-4b1f8f23ca09",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "You may have this creature assign its combat damage as though it weren't blocked — optional unblocked combat-damage assignment is not in the DSL",
    ),
    faces = &[face!(
        name = "Rhox",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RHINO, subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[activated!(
        cost!("{2}{G}"),
        &[Effect::regenerate(TargetSpec::ThisObject)],
    ),],
);
