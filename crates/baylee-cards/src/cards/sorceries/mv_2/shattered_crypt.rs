//! Shattered Crypt — {X}{B}{B} — Sorcery
//! Oracle: Return X target creature cards from your graveyard to your hand. You lose X life.
//! Set: WTH #80 — Weatherlight | Scryfall ID: 117df45d-4500-459b-96b5-ca41952580c1 | Oracle ID: 77bef290-7229-420a-a5c6-8c1b6fea63df
// IMPLEMENTED — Returns X target creature cards from your graveyard to your hand and you lose X life.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHATTERED_CRYPT,
    oracle_id = "77bef290-7229-420a-a5c6-8c1b6fea63df",
    scryfall_id = "117df45d-4500-459b-96b5-ca41952580c1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shattered Crypt",
        mana_cost = mana!("{X}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            },
            Effect::LoseLife {
                target: PlayerRel::You,
                amount: Amount::X,
            },
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    ),],
);
