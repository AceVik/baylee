#[allow(clippy::wildcard_imports)]
use super::*;

fn order_of_yawgmoth() -> CardIndex {
    card_index("c1795e9c-31c9-4892-a796-7656e09257b3")
}

#[test]
fn order_of_yawgmoth_makes_damaged_player_discard_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[order_of_yawgmoth()])
        .hand(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let knight = on_battlefield(&engine, p0, order_of_yawgmoth()).expect("seated");
    let opp_card = in_hand(&engine, p1, swamp()).expect("in hand");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || in_hand(e, p1, swamp()).is_none()
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![opp_card],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p1, swamp()).is_none());
}

#[test]
fn order_of_yawgmoth_fear_allows_artifact_and_black_blockers_only() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[order_of_yawgmoth()],
        &[grizzly_bears(), index::ORNITHOPTER, index::SCATHE_ZOMBIES],
        &[],
        &[],
    );
    let knight = lk::obj(&e, p0, order_of_yawgmoth());
    let green = lk::obj(&e, p1, grizzly_bears());
    let artifact = lk::obj(&e, p1, index::ORNITHOPTER);
    let black = lk::obj(&e, p1, index::SCATHE_ZOMBIES);
    let blocks = lk::attack(&mut e, &[knight]);
    assert!(!lk::may_block(&blocks, green, knight));
    assert!(lk::may_block(&blocks, artifact, knight));
    assert!(lk::may_block(&blocks, black, knight));
    lk::block(&mut e, &[(artifact, knight)]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 20);
}
