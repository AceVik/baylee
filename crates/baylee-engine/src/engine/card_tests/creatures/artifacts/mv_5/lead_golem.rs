//! `cards/creatures/artifacts/mv_5/lead_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5810c1e2-5a36-4a87-be4e-890ba1627b46"
fn lead_golem() -> CardIndex {
    card_index("5810c1e2-5a36-4a87-be4e-890ba1627b46")
}

#[test]
fn lead_golem_attacking_does_not_untap_during_controllers_next_untap_step() {
    let mut engine = lk::game(SEED, &[lead_golem()], &[], &[], &[]);
    let golem = lk::obj(&engine, P0, lead_golem());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(golem, Defender::Player(P1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(is_tapped(&engine, golem));
    assert_eq!(engine.state().players[1].life, 17);

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    assert!(
        is_tapped(&engine, golem),
        "Lead Golem remains tapped during its controller's next untap step"
    );

    // "Next untap step" is one untap step: it untaps in the one after.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        !is_tapped(&engine, golem),
        "Lead Golem untaps again at the untap step after the skipped one"
    );
}
