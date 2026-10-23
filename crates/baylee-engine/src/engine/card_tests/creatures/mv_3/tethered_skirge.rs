//! `cards/creatures/mv_3/tethered_skirge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tethered_skirge() -> CardIndex {
    card_index("42bbef66-967b-4922-8b2e-e57d1b8244bb")
}

#[test]
fn tethered_skirge_loses_life_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), tethered_skirge(), llanowar_elves()],
        )
        .hand(0, &[giant_growth(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "targeting another creature does not trigger"
    );
    let skirge = on_battlefield(&engine, p0, tethered_skirge()).unwrap();
    cast_from_hand(&mut engine, p0, giant_growth());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![skirge],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(pt(&engine, skirge), (5, 5));
}

#[test]
fn flying_prevents_a_ground_creature_from_blocking() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game_with(SEED, &[tethered_skirge()], &[gray_ogre()], &[], &[], |e| {
        let Pending::YesNo { player, .. } = e.pending().clone() else {
            panic!("echo payment")
        };
        e.dev_state_mut(player).unwrap().players[usize::from(player.get())]
            .mana_pool
            .add(ManaColor::Black, 3);
        e.apply(player, PlayerAction::YesNo(true)).unwrap();
    });
    let flyer = lk::obj(&engine, lk::P0, tethered_skirge());
    let ground = lk::obj(&engine, lk::P1, gray_ogre());
    let offer = lk::attack(&mut engine, &[flyer]);
    assert!(!lk::may_block(&offer, ground, flyer));
}
