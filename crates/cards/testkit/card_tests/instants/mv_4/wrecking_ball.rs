//! Oracle scenarios for Wrecking Ball.
#[allow(clippy::wildcard_imports)]
use super::*;
fn wrecking_ball() -> CardIndex {
    card_index("0a1b0910-3b0c-4aa3-b47f-8eb069122743")
}

// Checklist: either a creature or land is a legal target; chosen permanent is
// destroyed; an unrelated target category remains on the battlefield.
#[test]
fn wrecking_ball_can_destroy_a_land_without_touching_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[wrecking_ball(), wrecking_ball()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p1, forest()).unwrap();
    let elf = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut e, p0, wrecking_ball());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("creature or land target")
    };
    assert!(options.contains(&land) && options.contains(&elf));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p1, forest()).is_some());
    assert!(on_battlefield(&e, p1, llanowar_elves()).is_some());

    cast_from_hand(&mut e, p0, wrecking_ball());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("creature or land target")
    };
    assert!(
        options.contains(&elf),
        "the creature category remains legal"
    );
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p1, llanowar_elves()).is_some());
}
