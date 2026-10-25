//! Cloak of Confusion — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature you control
//! Oracle: Whenever enchanted creature attacks and isn't blocked, you may have it assign no combat damage this turn. If you do, defending player discards a card at random.
//! Set: ME2 #82 — Masters Edition II | Scryfall ID: f5fd387e-d77f-4502-8d2e-752a53f577fc | Oracle ID: 5d258409-28d3-4eac-9f1a-3e494db818bf

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLOAK_OF_CONFUSION,
    oracle_id = "5d258409-28d3-4eac-9f1a-3e494db818bf",
    scryfall_id = "f5fd387e-d77f-4502-8d2e-752a53f577fc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cloak of Confusion",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        triggered!(
            Trigger::AttacksAndIsntBlocked(&Filter::AttachedToBySource),
            &[Effect::MayDo {
                effects: &[
                    Effect::ModifyAttached {
                        modifier: Modifier::AssignsNoCombatDamage,
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::DiscardRandom {
                        who: PlayerRel::DefendingPlayer,
                        count: Amount::Fixed(1),
                    },
                ],
            }],
        ),
    ],
);
