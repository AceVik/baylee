//! `cards/sorceries/mv_5/cone_of_flame.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cone_of_flame() -> CardIndex {
    card_index("62879f3d-ed6d-4004-8196-f3fd7f614695")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn cone_of_flame_deals_1_2_3_to_three_distinct_targets() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[llanowar_elves(), grizzly_bears()])
        .hand(0, &[cone_of_flame()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let elves = on_battlefield(&e, P1, llanowar_elves()).unwrap();
    let bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut e, P0, cone_of_flame());
    kit::aim(&mut e, &[elves], &[]);
    kit::aim(&mut e, &[bears], &[]);
    // Third target menu must exclude elves and bears.
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("expected 3rd target prompt, got {:?}", e.pending());
    };
    assert!(!options.contains(&elves));
    assert!(!options.contains(&bears));
    assert!(player_options.contains(&P1));
    kit::aim(&mut e, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P1, llanowar_elves()).is_none());
    assert!(on_battlefield(&e, P1, grizzly_bears()).is_none());
    assert_eq!(e.state().players[1].life, 17);
}
