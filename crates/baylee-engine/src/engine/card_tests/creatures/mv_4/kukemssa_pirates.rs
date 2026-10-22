//! `cards/creatures/mv_4/kukemssa_pirates.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "280148c5-3a2f-48f7-a147-4578f2068073"
fn kukemssa_pirates() -> CardIndex {
    card_index("280148c5-3a2f-48f7-a147-4578f2068073")
}

#[test]
fn kukemssa_pirates_unblocked_steals_defenders_artifact_and_assigns_no_combat_damage() {
    let mut engine = lk::game(
        SEED,
        &[kukemssa_pirates()],
        &[index::JAYEMDAE_TOME],
        &[],
        &[],
    );
    let pirates = lk::obj(&engine, P0, kukemssa_pirates());
    let tome = lk::obj(&engine, P1, index::JAYEMDAE_TOME);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(pirates, Defender::Player(P1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    lk::aim(&mut engine, &[tome], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(
        engine.state().object(tome).unwrap().controller,
        P0,
        "Kukemssa Pirates steals P1's artifact"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "Kukemssa Pirates assigns no combat damage when it steals the artifact"
    );
}
