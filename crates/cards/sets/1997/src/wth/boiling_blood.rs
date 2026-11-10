//! Boiling Blood — {2}{R} — Instant
//! Oracle: Target creature attacks this turn if able.
//! Oracle: Draw a card.
//! Set: WTH #92 — Weatherlight | Scryfall ID: 3fcb85b6-ab5a-40db-aaae-555315f32877 | Oracle ID: cd55a003-da0d-4522-b08a-d0731b1050c6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOILING_BLOOD,
    oracle_id = "cd55a003-da0d-4522-b08a-d0731b1050c6",
    scryfall_id = "3fcb85b6-ab5a-40db-aaae-555315f32877",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Boiling Blood",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AttacksEachCombat,
                Duration::UntilEndOfTurn,
            ),
            Effect::draw(1),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
