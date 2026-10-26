//! Blood Hound — {2}{R} — Creature — Dog
//! Oracle: Whenever you're dealt damage, you may put that many +1/+1 counters on this creature.
//! Oracle: At the beginning of your end step, remove all +1/+1 counters from this creature.
//! Set: MMQ #176 — Mercadian Masques | Scryfall ID: baa1e796-809c-49af-a84e-ec088f7f48f8 | Oracle ID: ace532a3-fa6e-479d-b169-1fb24c3d0f8c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLOOD_HOUND,
    oracle_id = "ace532a3-fa6e-479d-b169-1fb24c3d0f8c",
    scryfall_id = "baa1e796-809c-49af-a84e-ec088f7f48f8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blood Hound",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::PlayerDealtDamage(PlayerRel::You),
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::EventAmount,
                }],
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::RemoveAllCountersSelf {
                kind: CounterKind::P1P1,
            }],
        ),
    ],
);
