//! Sorrow's Path — (no cost) — Land
//! Oracle: {T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking.
//! Oracle: Whenever this land becomes tapped, it deals 2 damage to you and each creature you control.
//! Set: ME3 #211 — Masters Edition III | Scryfall ID: 9347f02e-270b-44da-988c-d99a0fc5dfcb | Oracle ID: 5419eae0-3c12-4748-9cf8-d927752e527e

use baylee_cards_dsl::prelude::*;

static OPPONENT_BLOCKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Blocking,
    Filter::ControlledByOpponent,
]);

card!(
    index = index::SORROW_S_PATH,
    oracle_id = "5419eae0-3c12-4748-9cf8-d927752e527e",
    scryfall_id = "9347f02e-270b-44da-988c-d99a0fc5dfcb",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Sorrow's Path", types = TypeSet::LAND,)],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::SwapBlockers],
            targets = Some(TargetReq::exactly(
                TargetSpec::Object(&OPPONENT_BLOCKING_CREATURE),
                2
            ))
        ),
        triggered!(
            Trigger::BecomesTapped(&Filter::This),
            &[
                Effect::DealDamage {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::Player(PlayerRel::You),
                },
                Effect::DealDamageEach {
                    amount: Amount::Fixed(2),
                    filter: &Filter::YOUR_CREATURE,
                },
            ]
        ),
    ],
);
