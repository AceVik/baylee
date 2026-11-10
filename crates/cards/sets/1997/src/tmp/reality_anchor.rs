//! Reality Anchor — {1}{G} — Instant
//! Oracle: Target creature loses shadow until end of turn.
//! Oracle: Draw a card.
//! Set: TPR #188 — Tempest Remastered | Scryfall ID: 3a1b2918-2adc-4e0a-9cd7-9d80b3c8ce5f | Oracle ID: a7b4a519-d85f-40ae-bb5f-08bc1d301418
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REALITY_ANCHOR,
    oracle_id = "a7b4a519-d85f-40ae-bb5f-08bc1d301418",
    scryfall_id = "3a1b2918-2adc-4e0a-9cd7-9d80b3c8ce5f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reality Anchor",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::RemoveKeyword(KeywordSet::SHADOW),
                Duration::UntilEndOfTurn,
            ),
            Effect::draw(1),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
