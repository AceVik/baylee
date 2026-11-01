//! Brush with Death — {2}{B} — Sorcery
//! Oracle: Buyback {2}{B}{B} (You may pay an additional {2}{B}{B} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Target opponent loses 2 life. You gain 2 life.
//! Set: STH #52 — Stronghold | Scryfall ID: 04013bc6-7774-400b-a6af-de755b4c324d | Oracle ID: d87e580f-eca1-4f91-962b-82d684cba5e7
// PARTIAL: target opponent loses 2 life and you gain 2 life; buyback {2}{B}{B} is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRUSH_WITH_DEATH,
    oracle_id = "d87e580f-eca1-4f91-962b-82d684cba5e7",
    scryfall_id = "04013bc6-7774-400b-a6af-de755b4c324d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "target opponent loses 2 life and you gain 2 life; buyback {2}{B}{B} is not supported"
    ),
    faces = &[face!(
        name = "Brush with Death",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::Chosen,
            },
            Effect::gain_life(2),
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
