//! `cards/sorceries/mv_3/mogg_alarm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn mogg_alarm_creates_two_goblin_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[card_index("1c2117d5-4600-489c-be5b-dc0ed69b30ca")])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(
        &mut engine,
        p0,
        card_index("1c2117d5-4600-489c-be5b-dc0ed69b30ca"),
    );
    pass_until(&mut engine, stack_is_empty);
    let goblins = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p0 && pt(&engine, id) == (1, 1)
        })
        .count();
    assert_eq!(goblins, 2);
}
