//! Bellowing Fiend — {4}{B} — Creature — Spirit
//! Oracle: Flying
//! Oracle: Whenever this creature deals damage to a creature, this creature deals 3 damage to that creature's controller and 3 damage to you.
//! Set: 7ED #119 — Seventh Edition | Scryfall ID: 2b0962d7-d797-4f07-bd73-9cd7a11ffad8 | Oracle ID: 37331d0e-edda-4791-8bcc-fb457f984877
// PARTIAL — Triggers when this creature deals combat damage to a creature, dealing 3 damage to that creature's controller and 3 damage to you; Trigger::DealsCombatDamageToCreature covers combat damage only rather than any damage this creature deals to a creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BELLOWING_FIEND,
    oracle_id = "37331d0e-edda-4791-8bcc-fb457f984877",
    scryfall_id = "2b0962d7-d797-4f07-bd73-9cd7a11ffad8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Triggers when this creature deals combat damage to a creature, dealing 3 damage to that creature's controller and 3 damage to you; Trigger::DealsCombatDamageToCreature covers combat damage only rather than any damage this creature deals to a creature."
    ),
    faces = &[face!(
        name = "Bellowing Fiend",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToCreature(&Filter::This),
        &[
            Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
                amount: Amount::Fixed(3),
            },
            Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::You),
                amount: Amount::Fixed(3),
            },
        ]
    )],
);
