//! Game of Chaos — {R}{R}{R} — Sorcery
//! Oracle: Flip a coin. If you win the flip, you gain 1 life and target opponent loses 1 life, and you decide whether to flip again. If you lose the flip, you lose 1 life and that opponent gains 1 life, and that player decides whether to flip again. Double the life stakes with each flip.
//! Set: 5ED #232 — Fifth Edition | Scryfall ID: 95b44933-6b0b-426a-97d4-7a7cf6ad1d65 | Oracle ID: b6756608-a618-4e26-a090-ea8fc992def1
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: ... and you decide whether to flip again. ... and that player decides whether to flip again. Double the life stakes with each flip.
card!(
    index = index::GAME_OF_CHAOS,
    oracle_id = "b6756608-a618-4e26-a090-ea8fc992def1",
    scryfall_id = "95b44933-6b0b-426a-97d4-7a7cf6ad1d65",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial("repeatable doubling-stakes coin flip loop not supported"),
    faces = &[face!(
        name = "Game of Chaos",
        mana_cost = mana!("{R}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::FlipCoin {
            won: &[
                Effect::gain_life(1),
                Effect::LoseLife {
                    amount: Amount::Fixed(1),
                    target: PlayerRel::Chosen,
                },
            ],
            lost: &[
                Effect::LoseLife {
                    amount: Amount::Fixed(1),
                    target: PlayerRel::You,
                },
                Effect::GainLifeFor {
                    amount: Amount::Fixed(1),
                    who: PlayerRel::Chosen,
                },
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
