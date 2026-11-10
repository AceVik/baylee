//! `cards/creatures/mv_3/flowstone_shambler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Shambler is {2}{R} for a 2/2 Beast whose whole text is
/// "{R}: This creature gets +1/-1 until end of turn."
///
/// The exchange is read off the projected body: `(3, 1)` is the only answer
/// that shows both halves, where `(3, 3)` would mean the toughness was never
/// reduced and `(2, 2)` that nothing happened at all. The four Mountains pay
/// the {2}{R} and leave exactly the {R} the ability charges, so the
/// activation is a real payment out of the pool and not a label on a free
/// ability, and the second reading — in p1's main phase, with the same
/// permanent still standing — is what "until end of turn" has to give back.
#[test]
fn flowstone_shambler_trades_toughness_for_power_and_gives_it_back_with_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(27, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[flowstone_shambler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{R} out of the four Mountains, and the {R} the ability charges is
    // what stays floating beside it: a pool survives until the step ends
    // (CR 500.5) and this whole scenario plays inside this one main phase.
    cast_from_hand(&mut engine, p0, flowstone_shambler());
    pass_until(&mut engine, stack_is_empty);
    let shambler =
        on_battlefield(&engine, p0, flowstone_shambler()).expect("the Shambler resolved");
    assert_eq!(pt(&engine, shambler), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four Mountains paid {{2}}{{R}} and left exactly the {{R}}"
    );
    assert!(!is_tapped(&engine, shambler), "nothing has tapped it yet");

    // Ability 0 is the only line the card prints.
    activate(&mut engine, p0, flowstone_shambler(), 0);
    assert!(
        !stack_is_empty(&engine),
        "the pump is no mana ability (CR 605.1), so it goes on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, shambler),
        (3, 1),
        "+1/-1: a (3, 3) would be a toughness pump the card does not print, \
         and a (2, 2) that nothing happened at all"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, shambler),
        "the price is a red mana and no tap symbol, so the Shambler could do \
         it again"
    );

    // Across the opponent's turn: "until end of turn" ends in the cleanup step
    // of the turn that made it, so p1's main phase reads the printed body
    // again — and the reading is of the same permanent, still on the table.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, flowstone_shambler()).is_some(),
        "the same permanent is still standing, so the reading below is not a \
         fresh one"
    );
    assert_eq!(
        pt(&engine, shambler),
        (2, 2),
        "\"until end of turn\": the pump went with the turn that paid for it"
    );
}
