//! `cards/creatures/mv_7/resolute_archangel.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("0d6dc95d-55ce-4ef8-b065-d7203f5bf642")
}

#[test]
fn resolute_archangel_restores_starting_life_only_when_below_it() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[subject(), subject()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0].life = 7;
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 20);
    let angel = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    e.dev_state_mut(p0).unwrap().players[0].life = 25;
    reach_their_main_phase(&mut e, PlayerId::new(1));
    reach_their_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 25, "already above starting life");
}

/// "If your life total is less than your starting life total" is an
/// intervening "if" (CR 603.4): at or above it as the Angel enters, nothing
/// triggers, and life lost before the trigger would have resolved is not
/// restored. Without the condition the trigger went on the stack anyway and
/// restored 20 from the 10 the player had fallen to.
#[test]
fn resolute_archangel_entering_above_starting_life_does_not_trigger() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[subject()])
        .life(0, 25)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, |e| on_battlefield(e, p0, subject()).is_some());
    assert!(stack_is_empty(&e), "nothing triggered at 25 life");
    e.dev_state_mut(p0).unwrap().players[0].life = 10;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 10);
}
