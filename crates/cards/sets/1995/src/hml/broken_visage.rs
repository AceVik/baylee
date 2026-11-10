//! Broken Visage — {4}{B} — Instant
//! Oracle: Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.
//! Set: ME2 #81 — Masters Edition II | Scryfall ID: 9e934eed-d37c-4ede-b867-87bd17269b78 | Oracle ID: 8e1d750e-44a7-47b1-8e0a-5f8b15d48b41
// PARTIAL — dynamic P/T Spirit token matching the destroyed creature's power and toughness with delayed end-step sacrifice is not in Effect; destroys target nonartifact attacking creature without regeneration.

use baylee_cards_dsl::prelude::*;

static NONARTIFACT_ATTACKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::ARTIFACT),
    Filter::Attacking,
]);

card!(
    index = index::BROKEN_VISAGE,
    oracle_id = "8e1d750e-44a7-47b1-8e0a-5f8b15d48b41",
    scryfall_id = "9e934eed-d37c-4ede-b867-87bd17269b78",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "dynamic P/T Spirit token matching destroyed creature's power/toughness with delayed end-step sacrifice not in Effect; destroys target nonartifact attacking creature without regeneration"
    ),
    faces = &[face!(
        name = "Broken Visage",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy_no_regen(TargetSpec::Object(
            &NONARTIFACT_ATTACKING_CREATURE
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &NONARTIFACT_ATTACKING_CREATURE
        )))
    )],
);
