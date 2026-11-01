//! Winter Sky — {R} — Sorcery
//! Oracle: Flip a coin. If you win the flip, Winter Sky deals 1 damage to each creature and each player. If you lose the flip, each player draws a card.
//! Set: HML #80 — Homelands | Scryfall ID: af1035f3-3027-4a41-834c-55222b13c2bc | Oracle ID: fb150d6a-5f81-4fd8-92b2-22a9615bfd4f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINTER_SKY,
    oracle_id = "fb150d6a-5f81-4fd8-92b2-22a9615bfd4f",
    scryfall_id = "af1035f3-3027-4a41-834c-55222b13c2bc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Winter Sky",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::FlipCoin {
        won: &[
            Effect::DealDamageEach {
                amount: Amount::Fixed(1),
                filter: &Filter::CREATURE,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::EachPlayer),
            },
        ],
        lost: &[Effect::DrawCardsFor {
            who: PlayerRel::EachPlayer,
            amount: Amount::Fixed(1),
        }],
    }])],
);
