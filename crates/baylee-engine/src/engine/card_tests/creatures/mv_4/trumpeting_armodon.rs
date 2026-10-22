//! `cards/creatures/mv_4/trumpeting_armodon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn trumpeting_armodon() -> CardIndex {
    card_index("7fc3c2a5-2754-4237-8a37-cb3166c20c3c")
}

/// `Trumpeting Armodon` ({3}{G}, 3/3 Creature — Elephant, `Coverage::Partial`):
/// activates `{1}{G}` to require target creature to block this turn if able.
#[test]
fn trumpeting_armodon_forces_target_creature_to_block_if_able() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), trumpeting_armodon()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let armodon = on_battlefield(&engine, p0, trumpeting_armodon()).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, trumpeting_armodon(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(armodon, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Declaring zero blockers is illegal because Grizzly Bears must block if able.
    assert!(
        engine
            .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err()
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, armodon)],
            },
        )
        .unwrap();
}
