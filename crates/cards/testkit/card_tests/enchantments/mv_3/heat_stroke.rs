//! `cards/enchantments/mv_3/heat_stroke.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn heat_stroke() -> CardIndex {
    card_index("a8127c04-f67f-4780-86bf-bb24480ec51a")
}

fn ironroot_treefolk() -> CardIndex {
    card_index("b7c0bb85-fb87-4c73-bc1b-7b4dc763c7e8")
}

fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

#[test]
fn heat_stroke_destroys_creatures_that_blocked_or_were_blocked_at_end_of_combat() {
    let mut e = Duel::new(SEED, mountain())
        .battlefield(0, &[heat_stroke(), ironroot_treefolk(), grizzly_bears()])
        .battlefield(1, &[wall_of_stone()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    // 3/5 Ironroot Treefolk and 2/2 Bears attack; 0/8 Wall of Stone blocks Treefolk only.
    let treefolk = on_battlefield(&e, P0, ironroot_treefolk()).unwrap();
    let bears = on_battlefield(&e, P0, grizzly_bears()).unwrap();
    let wall = on_battlefield(&e, P1, wall_of_stone()).unwrap();
    kit::attack(&mut e, &[treefolk, bears]);
    kit::block(&mut e, &[(wall, treefolk)]);
    pass_until(&mut e, |e| {
        e.state().turn.phase == baylee_engine::turn::Phase::SecondMain && stack_is_empty(e)
    });

    // Treefolk (was blocked) and Wall (blocked) are destroyed; unblocked Bears survives!
    assert!(on_battlefield(&e, P0, ironroot_treefolk()).is_none());
    assert!(on_battlefield(&e, P1, wall_of_stone()).is_none());
    assert!(on_battlefield(&e, P0, grizzly_bears()).is_some());
}
