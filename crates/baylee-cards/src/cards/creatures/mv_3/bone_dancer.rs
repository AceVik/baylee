//! Bone Dancer — {1}{B}{B} — Creature — Zombie
//! Oracle: Whenever this creature attacks and isn't blocked, you may put the top creature card of defending player's graveyard onto the battlefield under your control. If you do, this creature assigns no combat damage this turn.
//! Set: WTH #62 — Weatherlight | Scryfall ID: 207bb4cd-4525-47e0-b412-0d0e29717d44 | Oracle ID: 13b5c9ef-d0e3-453d-ae5e-8de4f310cc38
// NOT SUPPORTED: putting the top creature card of defending player's graveyard onto the battlefield under your control is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BONE_DANCER,
    oracle_id = "13b5c9ef-d0e3-453d-ae5e-8de4f310cc38",
    scryfall_id = "207bb4cd-4525-47e0-b412-0d0e29717d44",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "triggers when it attacks and isn't blocked so you may have it assign no combat damage this turn; putting the top creature card of defending player's graveyard onto the battlefield under your control is not yet modeled",
    ),
    faces = &[face!(
        name = "Bone Dancer",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::continuous(
                &Filter::This,
                Modifier::AssignsNoCombatDamage,
                Duration::UntilEndOfTurn,
            )],
        }],
    )],
);
