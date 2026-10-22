//! `cards/creatures/mv_1/keeper_of_kookus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn keeper_of_kookus() -> CardIndex {
    card_index("97417270-e525-4261-b7d9-0dde3360e666")
}

fn goblin_swine_rider() -> CardIndex {
    card_index("9ebe32fc-3b6e-4cb9-9af0-1a6c3c6ad885")
}

/// `Keeper of Kookus` ({R}, 1/1 Goblin, `Coverage::Implemented`):
/// `{R}: This creature gains protection from red until end of turn.`
#[test]
fn keeper_of_kookus_gains_protection_from_red_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), keeper_of_kookus()])
        .battlefield(1, &[goblin_swine_rider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let keeper = on_battlefield(&engine, p0, keeper_of_kookus()).unwrap();
    let red_blocker = on_battlefield(&engine, p1, goblin_swine_rider()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, keeper_of_kookus(), 0);
    pass_until(&mut engine, stack_is_empty);

    // Protection from red prevents a red creature from blocking Keeper of Kookus.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(keeper, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(red_blocker, keeper)],
                },
            )
            .is_err(),
        "red creature cannot block Keeper of Kookus while it has protection from red"
    );
}
