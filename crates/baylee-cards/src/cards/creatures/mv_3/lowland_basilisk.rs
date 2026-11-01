//! Lowland Basilisk — {2}{G} — Creature — Basilisk
//! Oracle: Whenever this creature deals damage to a creature, destroy that creature at end of combat.
//! Set: TPR #178 — Tempest Remastered | Scryfall ID: 2c3cce7a-637f-4c68-af30-6f89f6c9f096 | Oracle ID: effe239e-ade0-4ee2-99d7-f93b2f5ef585
// PARTIAL: when this creature deals combat damage to a creature, destroys that creature at end of combat; triggering on noncombat damage dealt to a creature is not supported

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LOWLAND_BASILISK,
    oracle_id = "effe239e-ade0-4ee2-99d7-f93b2f5ef585",
    scryfall_id = "2c3cce7a-637f-4c68-af30-6f89f6c9f096",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "when this creature deals combat damage to a creature, destroys that creature at end of combat; triggering on noncombat damage dealt to a creature is not supported"
    ),
    faces = &[face!(
        name = "Lowland Basilisk",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BASILISK],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToCreature(&Filter::This),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[Effect::destroy(TargetSpec::EventObject)],
        }],
    )],
);
