//! `cards/creatures/mv_1/gustcloak_runner.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Gustcloak Runner (`0f485ea1-af00-4f6c-8e70-103ba172aa18`) — {W} Creature — Human Soldier 1/1:
/// "Whenever this creature becomes blocked, you may untap it and remove it from combat."
#[test]
fn gustcloak_runner_untaps_and_leaves_combat_when_blocked() {
    let runner_card = card_index("0f485ea1-af00-4f6c-8e70-103ba172aa18");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[runner_card])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let runner = on_battlefield(&engine, p0, runner_card).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(runner, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(
        engine
            .state()
            .object(runner)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, runner)],
            },
        )
        .unwrap();

    // `pass_until` answers `Yes` to `MayDo`, untapping Gustcloak Runner and removing it from combat.
    reach_their_main_phase(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, runner_card).is_some(),
        "Gustcloak Runner was removed from combat and survived blocking Hill Giant"
    );
    assert!(
        !engine
            .state()
            .object(runner)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Gustcloak Runner was untapped by its trigger"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        0,
        "Gustcloak Runner dealt no combat damage after leaving combat"
    );
}
