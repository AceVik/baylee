//! Ignoble Soldier — {2}{W} — Creature — Human Soldier
//! Oracle: Whenever this creature becomes blocked, prevent all combat damage that would be dealt by it this turn.
//! Set: MMQ #22 — Mercadian Masques | Scryfall ID: 676a2506-17f8-4b8e-be0c-eacc0fe972f6 | Oracle ID: 52a61983-53e2-440f-af03-cebe57b102f2
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: preventing combat damage dealt by the source after assignment; approximated with `Modifier::AssignsNoCombatDamage` until end of turn.
card!(
    index = index::IGNOBLE_SOLDIER,
    oracle_id = "52a61983-53e2-440f-af03-cebe57b102f2",
    scryfall_id = "676a2506-17f8-4b8e-be0c-eacc0fe972f6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Modifier::AssignsNoCombatDamage makes Ignoble Soldier assign no combat damage this turn rather than preventing its combat damage after assignment (CR 510.1a)",
    ),
    faces = &[face!(
        name = "Ignoble Soldier",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlocked,
        &[Effect::ModifySource {
            modifier: Modifier::AssignsNoCombatDamage,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
