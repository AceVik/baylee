//! Paroxysm — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player reveals the top card of their library. If that card is a land card, destroy that creature. Otherwise, it gets +3/+3 until end of turn.
//! Set: EXO #94 — Exodus | Scryfall ID: 53320321-4f02-40ee-8171-2375b1d4ed66 | Oracle ID: f00dd41d-4b11-491a-9e7b-12245092ab77
// PARTIAL: Enchanted creature's controller's upkeep trigger gives enchanted creature +3/+3 until end of turn; revealing the top card of that player's library to destroy the creature if it is a land card is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PAROXYSM,
    oracle_id = "f00dd41d-4b11-491a-9e7b-12245092ab77",
    scryfall_id = "53320321-4f02-40ee-8171-2375b1d4ed66",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Enchanted creature's controller's upkeep trigger gives enchanted creature +3/+3 until end of turn; revealing the top card of that player's library to destroy the creature if it is a land card is not supported.",
    ),
    faces = &[face!(
        name = "Paroxysm",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::ModifyAttached {
                modifier: Modifier::ModifyPT(3, 3),
                duration: Duration::UntilEndOfTurn,
            }]
        ),
    ],
);
