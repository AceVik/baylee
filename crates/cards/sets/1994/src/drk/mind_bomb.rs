//! Mind Bomb — {U} — Sorcery
//! Oracle: Each player may discard up to three cards. Mind Bomb deals damage to each player equal to 3 minus the number of cards they discarded this way.
//! Set: 5ED #105 — Fifth Edition | Scryfall ID: 3c46d040-58c2-4f7a-8b66-fea33fcb0e11 | Oracle ID: fad1562d-0ab9-4a3f-b80a-476d8132f66d

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_BOMB,
    oracle_id = "fad1562d-0ab9-4a3f-b80a-476d8132f66d",
    scryfall_id = "3c46d040-58c2-4f7a-8b66-fea33fcb0e11",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mind Bomb",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::DiscardUpToForPlayers {
            who: PlayerRel::EachPlayer,
            max: 3,
        },
        Effect::DealDamage {
            amount: Amount::ConstantMinus {
                constant: 3,
                subtract: &Amount::ThisWayBy {
                    what: ThisWay::Discarded,
                    who: PlayerRel::Recipient,
                },
            },
            target: TargetSpec::Player(PlayerRel::EachPlayer),
        },
    ])],
);
