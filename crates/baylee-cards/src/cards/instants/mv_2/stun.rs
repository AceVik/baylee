//! Stun — {1}{R} — Instant
//! Oracle: Target creature can't block this turn.
//! Oracle: Draw a card.
//! Set: TPR #163 — Tempest Remastered | Scryfall ID: 4b3231af-f1cc-4f3c-8f97-60e5d639d1ad | Oracle ID: d3491972-44c5-4962-a680-42b79357a189

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STUN,
    oracle_id = "d3491972-44c5-4962-a680-42b79357a189",
    scryfall_id = "4b3231af-f1cc-4f3c-8f97-60e5d639d1ad",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Stun",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::CANT_BLOCK),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::draw(1),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
