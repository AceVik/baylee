//! Mind's Eye lets its controller pay for a card on an opponent's draw.

use super::*;

fn mind_s_eye() -> CardIndex {
    card_index("63fd2a57-7a47-4e07-947c-f4e9da7ee538")
}

fn board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mind_s_eye(), island()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e
}

/// Walks to the payment question p1's draw step puts to p0.
fn reach_the_offer(e: &mut Engine<RegistryLookup>) {
    let p1 = PlayerId::new(1);
    pass_until(e, |e| {
        e.state().turn.active == p1
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayTax { mana: 1 },
                    ..
                }
            )
    });
    assert_eq!(e.pending().asked(), Some(PlayerId::new(0)));
}

#[test]
fn mind_s_eye_draws_when_the_triggered_payment_is_made() {
    let p0 = PlayerId::new(0);
    let mut e = board();
    let land = on_battlefield(&e, p0, island()).unwrap();
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_the_offer(&mut e);
    assert!(!is_tapped(&e, land));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    if matches!(e.pending(), Pending::Priority { .. })
        && e.state().players[0].mana_pool.total() == 0
    {
        tap_all_mana(&mut e, p0);
        e.apply(p0, PlayerAction::PassPriority).unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, land), "the {{1}} was paid with the Island");
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn mind_s_eye_declined_draws_nothing_and_spends_nothing() {
    let p0 = PlayerId::new(0);
    let mut e = board();
    let land = on_battlefield(&e, p0, island()).unwrap();
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_the_offer(&mut e);
    e.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, land));
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), before);
}

/// "An opponent draws": the controller's own draw step asks nothing.
/// `pass_until` panics on a `PayTax` question, so a trigger on p0's draw
/// would fail this before the hand count is read.
#[test]
fn mind_s_eye_ignores_its_controller_s_own_draw() {
    let p0 = PlayerId::new(0);
    let mut e = board();
    reach_the_offer(&mut e);
    e.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut e, p0);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1,
        "only the draw step's card"
    );
}
