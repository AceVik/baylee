//! Warning — {W} — Instant
//! Oracle: Prevent all combat damage that would be dealt by target attacking creature this turn.
//! Set: ME2 #38 — Masters Edition II | Scryfall ID: 5cfbcb49-a640-4c0d-800f-482b813369a3 | Oracle ID: c5ba0f0f-65c5-4ffa-987a-f320b401ec8f

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WARNING,
    oracle_id = "c5ba0f0f-65c5-4ffa-987a-f320b401ec8f",
    scryfall_id = "5cfbcb49-a640-4c0d-800f-482b813369a3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Warning",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::PreventDamageFromIt,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE
        )))
    )],
);
