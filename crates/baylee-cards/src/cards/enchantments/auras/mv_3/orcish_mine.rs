//! Orcish Mine — {1}{R}{R} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: This Aura enters with three ore counters on it.
//! Oracle: At the beginning of your upkeep and whenever enchanted land becomes tapped, remove an ore counter from this Aura.
//! Oracle: When the last ore counter is removed from this Aura, destroy enchanted land and this Aura deals 2 damage to that land's controller.
//! Set: HML #78 — Homelands | Scryfall ID: 3a630875-b43d-4591-992c-117e1212fa34 | Oracle ID: a6bfa03e-6317-4550-960b-fbd2a30e521f
// PARTIAL — ore counter type is not yet in counters.rs; uses counters::MINING for the three counters, upkeep/tap removal, and last-counter land destruction and 2 damage to its controller.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORCISH_MINE,
    oracle_id = "a6bfa03e-6317-4550-960b-fbd2a30e521f",
    scryfall_id = "3a630875-b43d-4591-992c-117e1212fa34",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "ore counter type is not yet in counters.rs; uses counters::MINING for the three counters, upkeep/tap removal, and last-counter land destruction and 2 damage to its controller"
    ),
    faces = &[face!(
        name = "Orcish Mine",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::MINING,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::RemoveCounterSelf {
                kind: counters::MINING,
                n: 1,
            }],
        ),
        triggered!(
            Trigger::BecomesTapped(&Filter::AttachedToBySource),
            &[Effect::RemoveCounterSelf {
                kind: counters::MINING,
                n: 1,
            }],
        ),
        triggered!(
            Trigger::LastCounterRemoved {
                kind: counters::MINING,
            },
            &[
                Effect::DealDamage {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::Player(PlayerRel::ControllerOfAttached),
                },
                Effect::DestroyAll {
                    filter: &Filter::AttachedToBySource,
                    no_regen: false,
                },
            ],
        ),
    ],
);
