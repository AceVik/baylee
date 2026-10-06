//! `cards/lands/utility/kjeldoran_outpost.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kjeldoran Outpost prints an enter replacement to sacrifice a Plains, `{{T}}: Add {{W}}.`, and `{{1}}{{W}}, {{T}}: Create a 1/1 white Soldier creature token.`
///
/// Under `Coverage::Partial`, the enter replacement is omitted because no `EnterModifier` expresses sacrificing a Plains or going to the graveyard instead.
/// Playing Kjeldoran Outpost from hand enters the battlefield untapped without sacrificing a controlled `plains()`.
/// Floating `{{1}}{{W}}` from two `plains()` and activating ability 1 creates a 1/1 white Soldier creature token, tapping the outpost.
#[test]
fn kjeldoran_outpost_enters_without_sacrifice_and_creates_soldier() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[kjeldoran_outpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let outpost = play_land(&mut engine, p0, kjeldoran_outpost());
    assert!(
        !entered_tapped(&engine, outpost),
        "kjeldoran outpost prints no enters-tapped clause"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        2,
        "controlled plains remain intact under `Coverage::Partial`"
    );

    // Float {{1}}{{W}} from the two Plains while keeping Kjeldoran Outpost untapped.
    tap_mana_except(&mut engine, p0, outpost);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, outpost));

    activate(&mut engine, p0, kjeldoran_outpost(), 1);
    pass_until(&mut engine, stack_is_empty);

    let soldiers = tokens_of(&engine, p0);
    assert_eq!(soldiers.len(), 1, "one soldier token created");
    assert_eq!(pt(&engine, soldiers[0]), (1, 1), "soldier is 1/1");
    assert!(is_tapped(&engine, outpost));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
