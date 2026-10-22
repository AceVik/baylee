//! Gift of the Woods — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked, it gets +0/+3 until end of turn and you gain 1 life.
//! Set: ALL #92a — Alliances | Scryfall ID: da48976b-667d-4a1e-92de-9c3cb25dfd21 | Oracle ID: 491c6f69-9b75-4d5d-b41f-a14b2fbfdd07
// NOT SUPPORTED: triggering from the Aura itself when enchanted creature blocks or becomes blocked is approximated by granting the triggered ability to enchanted creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIFT_OF_THE_WOODS,
    oracle_id = "491c6f69-9b75-4d5d-b41f-a14b2fbfdd07",
    scryfall_id = "da48976b-667d-4a1e-92de-9c3cb25dfd21",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "attaches to target creature and grants enchanted creature a trigger that gives it +0/+3 until end of turn and gains 1 life whenever it blocks or becomes blocked; triggering from the Aura itself rather than granting the triggered ability to the enchanted creature is not supported",
    ),
    faces = &[face!(
        name = "Gift of the Woods",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
                effects: &[
                    Effect::PumpFilter {
                        filter: &Filter::This,
                        controlled_by: None,
                        power: Amount::Fixed(0),
                        toughness: Amount::Fixed(3),
                        keywords: KeywordSet::EMPTY,
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::gain_life(1),
                ],
                target: None,
            },
        ),
    ],
);
