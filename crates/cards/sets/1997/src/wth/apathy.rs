//! Apathy — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature doesn't untap during its controller's untap step.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player may discard a card at random. If the player does, untap that creature.
//! Set: WTH #33 — Weatherlight | Scryfall ID: adf3a6fe-e234-4c3f-96fc-3eb5eb22c0b8 | Oracle ID: 09ac5adf-f722-417e-b0fc-7ed0d8c9abd6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::APATHY,
    oracle_id = "09ac5adf-f722-417e-b0fc-7ed0d8c9abd6",
    scryfall_id = "adf3a6fe-e234-4c3f-96fc-3eb5eb22c0b8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Apathy",
        mana_cost = mana!("{U}"),
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
        static_ability!(ENCHANTED_CREATURE, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[
                // The enchanted creature's controller decides (not the Aura's), and the discard
                // is theirs; the creature untaps only if a card was discarded this way (an empty
                // hand cannot pay it).
                Effect::MayDoFor {
                    player: PlayerRel::ControllerOfAttached,
                    effects: &[Effect::DiscardRandom {
                        who: PlayerRel::ControllerOfAttached,
                        count: Amount::Fixed(1),
                    }],
                },
                Effect::IfCondition {
                    condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::Discarded), 1),
                    then: &[Effect::UntapAll {
                        filter: &ENCHANTED_CREATURE,
                    }],
                    otherwise: &[],
                },
            ],
        ),
    ],
);
