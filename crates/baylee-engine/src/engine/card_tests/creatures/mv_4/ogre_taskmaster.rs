//! `cards/creatures/mv_4/ogre_taskmaster.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "5604f04e-fd72-41bf-906f-eb89d4a3475d"
fn ogre_taskmaster() -> CardIndex {
    card_index("5604f04e-fd72-41bf-906f-eb89d4a3475d")
}

/// `Ogre Taskmaster` (`Coverage::Implemented`):
/// "This creature can't block."
#[test]
fn ogre_taskmaster_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9705, mountain())
        .battlefield(0, &[ogre_taskmaster()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    let taskmaster = on_battlefield(&engine, p0, ogre_taskmaster()).expect("Ogre Taskmaster");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("Grizzly Bears");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. }) || e.state().turn.step == Step::End
    });

    if matches!(engine.pending(), Pending::ChooseBlockers { .. }) {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::DeclareBlockers {
                        blockers: vec![(taskmaster, bears)],
                    },
                )
                .is_err(),
            "Ogre Taskmaster cannot block"
        );
    }
}
