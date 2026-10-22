//! Forbidden Ritual — {2}{B}{B} — Sorcery
//! Oracle: Sacrifice a nontoken permanent. If you do, target opponent loses 2 life unless that player sacrifices a permanent of their choice or discards a card. You may repeat this process any number of times.
//! Set: VIS #60 — Visions | Scryfall ID: f5327e6d-db4e-4b44-a00e-b764e80b8946 | Oracle ID: 4b5d9c76-8be8-4e25-b978-9750df3d8c15
// PARTIAL — Repeating the sacrifice process any number of times and offering the targeted opponent a three-way punisher choice (lose 2 life, sacrifice a permanent, or discard a card) is not yet supported in Effect; in play, Forbidden Ritual sacrifices a nontoken permanent you control and causes target opponent to lose 2 life.

use baylee_cards_dsl::prelude::*;

static YOUR_NONTOKEN_PERMANENT: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::Not(&Filter::IsToken)]);

card!(
    index = index::FORBIDDEN_RITUAL,
    oracle_id = "4b5d9c76-8be8-4e25-b978-9750df3d8c15",
    scryfall_id = "f5327e6d-db4e-4b44-a00e-b764e80b8946",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Repeating the sacrifice process any number of times and offering the targeted opponent a three-way punisher choice (lose 2 life, sacrifice a permanent, or discard a card) is not yet supported in Effect; in play, Forbidden Ritual sacrifices a nontoken permanent you control and causes target opponent to lose 2 life.",
    ),
    faces = &[face!(
        name = "Forbidden Ritual",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::SacrificeOneThen {
            filter: &YOUR_NONTOKEN_PERMANENT,
            if_it_was: &Filter::Any,
            then: &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::Chosen,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
