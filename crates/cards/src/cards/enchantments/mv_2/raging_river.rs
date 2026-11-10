//! Raging River — {R}{R} — Enchantment
//! Oracle: Whenever one or more creatures you control attack, each defending player divides all creatures without flying they control into a "left" pile and a "right" pile. Then, for each attacking creature you control, choose "left" or "right." That creature can't be blocked this combat except by creatures with flying and creatures in a pile with the chosen label.
//! Set: 2ED #169 — Unlimited Edition | Scryfall ID: 7ee63877-056e-413d-932a-a393a4183686 | Oracle ID: a2310312-6e1e-4e34-a351-9aef499a810f
// IMPLEMENTED — once per attack: each defending player names a left pile, the
// controller labels each attacker, and only fliers and that pile may block it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAGING_RIVER,
    oracle_id = "a2310312-6e1e-4e34-a351-9aef499a810f",
    scryfall_id = "7ee63877-056e-413d-932a-a393a4183686",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Raging River",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::OneOrMoreAttack(&Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])),
        &[Effect::LeftRightPilesRestrictBlocks],
    )],
);
