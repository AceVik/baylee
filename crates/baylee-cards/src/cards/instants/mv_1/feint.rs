//! Feint — {R} — Instant
//! Oracle: Tap all creatures blocking target attacking creature. Prevent all combat damage that would be dealt this turn by that creature and each creature blocking it.
//! Set: LEG #146 — Legends | Scryfall ID: 95b265bc-a94d-403b-8232-8fdfa0f8d9d5 | Oracle ID: 1bb8fe05-abb3-40a8-9e80-5d99ed0e4284
// IMPLEMENTED — taps every creature blocking the target attacker, then
// prevents the combat damage that attacker and those blockers would deal
// this turn (a creature that blocks it later is not covered).

use baylee_cards_dsl::prelude::*;

/// "That creature and each creature blocking it", bound as the spell resolves.
static ATTACKER_AND_BLOCKERS: Filter = Filter::Or(&[Filter::Reference, Filter::BlockingReference]);

card!(
    index = index::FEINT,
    oracle_id = "1bb8fe05-abb3-40a8-9e80-5d99ed0e4284",
    scryfall_id = "95b265bc-a94d-403b-8232-8fdfa0f8d9d5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Feint",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::TapAll {
                filter: &Filter::BlockingReference
            },
            Effect::continuous(
                &ATTACKER_AND_BLOCKERS,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn
            ),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE
        )))
    )],
);
