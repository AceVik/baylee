//! Oracle scenarios for Ancestral Reminiscence.
#[allow(clippy::wildcard_imports)]
use super::*;
fn ancestral_reminiscence() -> CardIndex {
    card_index("dbfa2743-34ff-40e3-81f7-740fc08cd18a")
}

// Checklist: three cards are drawn before the mandatory one-card discard;
// selecting one fresh draw demonstrates the order and the choice.
#[test]
fn ancestral_reminiscence_draws_three_then_discards_one_of_them() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[ancestral_reminiscence(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let before = library_size(&e, p0);
    cast_from_hand(&mut e, p0, ancestral_reminiscence());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert_eq!(
        library_size(&e, p0),
        before - 3,
        "all three draws precede discard"
    );
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        panic!("discard choice")
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    let drawn = options
        .iter()
        .copied()
        .find(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .expect("the drawn library cards are discard candidates");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![drawn],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p0, forest()).is_some());
    assert!(in_hand(&e, p0, llanowar_elves()).is_some());
    assert_eq!(library_size(&e, p0), before - 3);
}
