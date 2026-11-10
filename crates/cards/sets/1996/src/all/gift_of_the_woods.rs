//! Gift of the Woods — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked, it gets +0/+3 until end of turn and you gain 1 life.
//! Set: ALL #92a — Alliances | Scryfall ID: da48976b-667d-4a1e-92de-9c3cb25dfd21 | Oracle ID: 491c6f69-9b75-4d5d-b41f-a14b2fbfdd07
// IMPLEMENTED — the Aura's own trigger pumps its attached creature and gives its controller 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIFT_OF_THE_WOODS,
    oracle_id = "491c6f69-9b75-4d5d-b41f-a14b2fbfdd07",
    scryfall_id = "da48976b-667d-4a1e-92de-9c3cb25dfd21",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
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
        triggered!(
            Trigger::AttachedBlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
            &[
                Effect::PumpFilter {
                    filter: &Filter::AttachedToBySource,
                    controlled_by: None,
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(3),
                    keywords: KeywordSet::EMPTY,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::gain_life(1),
            ],
        ),
    ],
);
