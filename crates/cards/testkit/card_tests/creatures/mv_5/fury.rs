//! `cards/creatures/mv_5/fury.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Any number" includes none (CR 115.6): the trigger goes on the stack
/// untargeted and resolves dealing nothing, and nothing is asked.
#[test]
fn fury_with_no_target_deals_nothing() {
    let p1 = PlayerId::new(1);
    let mut engine = fury_enters(&[thundering_giant()], false);
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    fury_aims(&mut engine, vec![]);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "got {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(marked(&engine, giant), 0);
    assert!(on_battlefield(&engine, PlayerId::new(0), fury()).is_some());
}

/// "Evoke—Exile a red card from your hand." Evoked, Fury still divides its
/// damage as it enters, and is sacrificed by the evoke trigger (CR 702.74a).
#[test]
fn fury_evoked_divides_its_damage_and_is_sacrificed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = fury_enters(&[striped_bears(), llanowar_elves()], true);
    let bears = on_battlefield(&engine, p1, striped_bears()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    fury_aims(&mut engine, vec![bears, elves]);
    assert_eq!(fury_share(&engine, bears, 0, 2, 4), (1, 3));
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, striped_bears()).is_some());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p0, fury()).is_some(), "sacrificed");
    assert!(
        in_hand(&engine, p0, lightning_bolt()).is_none()
            && in_graveyard(&engine, p0, lightning_bolt()).is_none(),
        "the Bolt was exiled to pay"
    );
}
