#[allow(clippy::wildcard_imports)]
use super::*;

fn somnophore() -> CardIndex {
    card_index("86541bba-7db8-4893-addf-8a4a42168ce4")
}

#[test]
fn somnophore_taps_and_locks_damaged_players_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[somnophore(), grizzly_bears()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let somno = on_battlefield(&engine, p0, somnophore()).expect("somnophore on bf");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    let _ = attack_and_collect_blocks(&mut engine, somno, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let own_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let options = aim_at(&mut engine, p0, ogre);
    assert!(!options.contains(&own_bear));
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    kill(&mut engine, somno);
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    assert!(!is_tapped(&engine, ogre), "leaving ends the lock");
}
