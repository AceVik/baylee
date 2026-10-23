//! Gift of the Woods — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked, it gets +0/+3 until end of turn and you gain 1 life.
//! Set: ALL #92a — Alliances | Scryfall ID: da48976b-667d-4a1e-92de-9c3cb25dfd21 | Oracle ID: 491c6f69-9b75-4d5d-b41f-a14b2fbfdd07
// NOT SUPPORTED: "Whenever enchanted creature blocks or becomes blocked, ... you gain 1 life" as the Aura's own trigger (it would fire once per blocker); the trigger is granted to the enchanted creature instead, so its controller gains the life, which is the Aura's controller only while they stay the same player (the Aura may therefore enchant only a creature you control).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);

card!(
    index = index::GIFT_OF_THE_WOODS,
    oracle_id = "491c6f69-9b75-4d5d-b41f-a14b2fbfdd07",
    scryfall_id = "da48976b-667d-4a1e-92de-9c3cb25dfd21",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the trigger is granted to the enchanted creature instead of being the Aura's own, so the life is gained by the enchanted creature's controller (the Aura's controller until the creature changes control), a creature that loses its abilities loses the trigger, and the Aura may enchant only a creature you control",
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
                target: TargetSpec::Object(&YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&YOUR_CREATURE))),
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
