//! Stromgald Spy — {3}{B} — Creature — Human Rogue
//! Oracle: Whenever this creature attacks and isn't blocked, you may have defending player play with their hand revealed for as long as this creature remains on the battlefield. If you do, this creature assigns no combat damage this turn.
//! Set: ALL #62 — Alliances | Scryfall ID: 0cf8cecc-449f-4cc6-ac4d-440722df0ab9 | Oracle ID: b7aa0625-c691-4219-b099-3798b349d09c

// PARTIAL — "you may have defending player play with their hand revealed ... If you do, this creature assigns no combat damage this turn": the hand is revealed once, and the combat damage is suppressed.
// NOT SUPPORTED: "for as long as this creature remains on the battlefield" (a hand that stays revealed to you); nothing keeps one player's hand open to another player for a duration.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STROMGALD_SPY,
    oracle_id = "b7aa0625-c691-4219-b099-3798b349d09c",
    scryfall_id = "0cf8cecc-449f-4cc6-ac4d-440722df0ab9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Stromgald Spy",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(2),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "The defending player's hand is revealed once when this creature attacks and isn't blocked, not for as long as this creature remains on the battlefield; the \"may\" and \"this creature assigns no combat damage this turn\" are played."
    ),
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::RevealHand {
                    who: PlayerRel::DefendingPlayer,
                },
                // "If you do, this creature assigns no combat damage this turn."
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        }]
    )],
);
