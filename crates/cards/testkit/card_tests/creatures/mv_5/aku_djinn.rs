//! `cards/creatures/mv_5/aku_djinn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn aku_djinn() -> CardIndex {
    card_index("233c6686-ead8-4569-941d-3bfa768ff270")
}

#[test]
fn aku_djinn_upkeep_puts_plus_one_counter_on_each_opponents_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), swamp(), grizzly_bears()],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[aku_djinn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aku_djinn());
    pass_until(&mut engine, stack_is_empty);

    let djinn = on_battlefield(&engine, p0, aku_djinn()).unwrap();
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let my_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, djinn), (5, 6));
    assert!(keywords(&engine, djinn).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, opp_bear),
        (2, 2),
        "only the Djinn's controller's upkeep counts: nothing on the opponent's upkeep"
    );
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(pt(&engine, opp_bear), (3, 3));
    assert_eq!(
        pt(&engine, my_bear),
        (2, 2),
        "only creatures the opponent controls"
    );
    assert_eq!(pt(&engine, djinn), (5, 6));
}
