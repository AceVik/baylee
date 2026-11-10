//! `cards/creatures/mv_1/rime_dryad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rime_dryad() -> CardIndex {
    card_index("0464176d-75d2-403a-aa9d-40e85f106d01")
}

fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}

/// Rime Dryad (0464176d-75d2-403a-aa9d-40e85f106d01) — {G} 1/2 Creature — Dryad:
/// "Snow forestwalk (This creature can't be blocked as long as defending player controls a snow Forest.)"
#[test]
fn rime_dryad_can_be_blocked_with_ordinary_forest_but_not_snow_forest() {
    // 1. Defending player controls only an ordinary (non-snow) Forest: Rime Dryad CAN be blocked.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rime_dryad()])
        .battlefield(1, &[forest(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dryad = on_battlefield(&engine, P0, rime_dryad()).unwrap();
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let blocks = attack_and_collect_blocks(&mut engine, dryad, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&dryad)),
        "ordinary Forest does not trigger snow forestwalk: Rime Dryad can be blocked"
    );

    // 2. Defending player controls a Snow-Covered Forest: Rime Dryad CANNOT be blocked,
    // while a non-landwalk attacker (Gray Ogre) CAN be blocked.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rime_dryad(), gray_ogre()])
        .battlefield(1, &[snow_covered_forest(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dryad = on_battlefield(&engine, P0, rime_dryad()).unwrap();
    let ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dryad, Defender::Player(P1)), (ogre, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers");
    };
    assert!(
        !blockers
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&dryad)),
        "Rime Dryad is unblockable when defending player controls a snow Forest"
    );
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&ogre)),
        "Gray Ogre without snow forestwalk can still be blocked"
    );
}
