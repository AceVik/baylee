//! Mask of Memory's optional draw-two/discard-one combat trigger.

use super::*;

fn mask_of_memory() -> CardIndex {
    card_index("d6b2c998-a226-426c-a40d-6e6007041bfe")
}

#[test]
fn mask_of_memory_draws_two_then_discards_one_if_accepted() {
    let p0 = PlayerId::new(0);
    let mut r = rig(mask_of_memory(), llanowar_elves(), 1);
    r.equip(1);
    r.attack_to_the_blocks();
    r.engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    r.engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert_eq!(r.hand_size(), 2, "both cards were drawn before discarding");
    let Pending::ChooseCards { options, .. } = r.engine.pending().clone() else {
        unreachable!()
    };
    r.engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        e.state().turn.phase == Phase::Ending && stack_is_empty(e)
    });
    assert_eq!(r.hand_size(), 1);
    assert_eq!(r.engine.state().players[1].life, 19);
}

/// "You may draw two cards. If you do, discard a card": declined, nothing
/// is drawn and nothing is asked to be discarded.
#[test]
fn mask_of_memory_declined_draws_nothing() {
    let p0 = PlayerId::new(0);
    let mut r = rig(mask_of_memory(), llanowar_elves(), 1);
    r.equip(1);
    r.attack_to_the_blocks();
    r.engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let before = r.hand_size();
    r.engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut r.engine, |e| {
        e.state().turn.phase == Phase::Ending && stack_is_empty(e)
    });
    assert_eq!(r.hand_size(), before);
    assert_eq!(r.engine.state().players[1].life, 19, "the hit still landed");
}
