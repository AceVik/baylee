//! Farrel's Mantle — {2}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature attacks and isn't blocked, its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn.
//! Set: ME2 #13 — Masters Edition II | Scryfall ID: cd5182d9-fc52-4309-a622-41c0b5ae58b8 | Oracle ID: ffae3ff4-67f7-4e41-b166-c63f88c7aa08
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ANOTHER_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::AttachedToBySource)]);

card!(
    index = index::FARREL_S_MANTLE,
    oracle_id = "ffae3ff4-67f7-4e41-b166-c63f88c7aa08",
    scryfall_id = "cd5182d9-fc52-4309-a622-41c0b5ae58b8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Farrel's Mantle",
        mana_cost = mana!("{2}{W}"),
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
            Trigger::AttacksAndIsntBlocked(&Filter::AttachedToBySource),
            &[Effect::MayDoFor {
                player: PlayerRel::ControllerOfEvent,
                effects: &[
                    Effect::EventObjectDealsDamage {
                        amount: Amount::Plus {
                            base: &Amount::EventPower,
                            offset: 2,
                        },
                        target: TargetSpec::Object(&ANOTHER_CREATURE),
                    },
                    Effect::ModifyAttached {
                        modifier: Modifier::AssignsNoCombatDamage,
                        duration: Duration::UntilEndOfTurn,
                    },
                ],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_CREATURE))),
        ),
    ],
);
