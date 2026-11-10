//! Pit Spawn — {4}{B}{B}{B} — Creature — Demon
//! Oracle: First strike
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {B}{B}.
//! Oracle: Whenever this creature deals damage to a creature, exile that creature.
//! Set: EXO #70 — Exodus | Scryfall ID: 669ad60b-4053-4f07-9072-52e6ff65b4e3 | Oracle ID: f536acf3-ff8e-48da-8509-00e7ce4a1678
// PARTIAL — First strike, upkeep sacrifice-unless-pay {B}{B}, and exiling creatures dealt combat damage are implemented; exiling creatures dealt noncombat damage by Pit Spawn is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: First strike, upkeep sacrifice-unless-pay {B}{B}, and exiling creatures dealt combat damage are implemented; exiling creatures dealt noncombat damage by Pit Spawn is not supported.
card!(
    index = index::PIT_SPAWN,
    oracle_id = "f536acf3-ff8e-48da-8509-00e7ce4a1678",
    scryfall_id = "669ad60b-4053-4f07-9072-52e6ff65b4e3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "First strike, upkeep sacrifice-unless-pay {B}{B}, and exiling creatures dealt combat damage are implemented; exiling creatures dealt noncombat damage by Pit Spawn is not supported.",
    ),
    faces = &[face!(
        name = "Pit Spawn",
        mana_cost = mana!("{4}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        keywords = KeywordSet::FIRST_STRIKE,
        power = Some(6),
        toughness = Some(4),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{B}{B}"),
                effect: &Effect::SacrificeSelf,
            }]
        ),
        triggered!(
            Trigger::DealsCombatDamageToCreature(&Filter::This),
            &[Effect::Exile {
                target: TargetSpec::EventObject,
            }]
        ),
    ],
);
