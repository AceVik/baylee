//! `cards/lands/utility/kjeldoran_outpost.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pay the Plains entry replacement, then create a Soldier with the remaining lands.
#[test]
fn kjeldoran_outpost_sacrifices_a_plains_then_creates_soldier() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[kjeldoran_outpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let outpost = play_land(&mut engine, p0, kjeldoran_outpost());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let payment = on_battlefield(&engine, p0, plains()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![payment],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(payment).unwrap().zone,
        Zone::Graveyard
    );
    assert!(
        !entered_tapped(&engine, outpost),
        "kjeldoran outpost prints no enters-tapped clause"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        2,
        "two Plains remain after paying the entry sacrifice"
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
