//! Sawtooth Ogre — {2}{R}{R} — Creature — Ogre
//! Oracle: Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature at end of combat.
//! Set: WTH #114 — Weatherlight | Scryfall ID: 4a237580-f7f6-4d6b-a342-0d11fc0b5a59 | Oracle ID: 473c9f27-2091-4d87-b279-64b3662c9af8
// IMPLEMENTED — Whenever it blocks or becomes blocked by a creature, a delayed trigger deals 1 damage from it to that creature at end of combat (CR 603.7).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAWTOOTH_OGRE,
    oracle_id = "473c9f27-2091-4d87-b279-64b3662c9af8",
    scryfall_id = "4a237580-f7f6-4d6b-a342-0d11fc0b5a59",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sawtooth Ogre",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::EventObject,
            }],
        }],
    ),],
);
