#[allow(clippy::wildcard_imports)]
use super::*;

fn songstitcher() -> CardIndex {
    card_index("eec72bcf-ccbc-43fc-8bdd-7bf4faa1fad7")
}

fn somnophore() -> CardIndex {
    card_index("86541bba-7db8-4893-addf-8a4a42168ce4")
}

#[test]
fn songstitcher_prevents_combat_damage_from_attacking_flyer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[songstitcher(), plains(), plains()])
        .battlefield(1, &[somnophore()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let flyer = on_battlefield(&engine, p1, somnophore()).expect("flyer on bf");
    let _ = attack_and_collect_blocks(&mut engine, flyer, p0);
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, songstitcher(), 0);
    aim_at(&mut engine, p0, flyer);
    pass_until(&mut engine, stack_is_empty);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn songstitcher_excludes_ground_attackers_and_idle_flyers() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[songstitcher()],
        &[somnophore(), gray_ogre()],
        &[],
        &[],
    );
    lk::float(&mut e, p0, &[(ManaColor::White, 2)]);
    assert!(
        !lk::offered(&e, songstitcher(), p0, 0),
        "idle flyer is not a target"
    );
    reach_their_main_phase(&mut e, p1);
    let flyer = lk::obj(&e, p1, somnophore());
    let ground = lk::obj(&e, p1, gray_ogre());
    lk::attack(&mut e, &[flyer, ground]);
    lk::block(&mut e, &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    lk::float(&mut e, p0, &[(ManaColor::White, 2)]);
    activate(&mut e, p0, songstitcher(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(options.contains(&flyer) && !options.contains(&ground));
    lk::aim(&mut e, &[flyer], &[]);
    lk::settle(&mut e);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 18, "ground attack is unaffected");
}
