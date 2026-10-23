//! `cards/creatures/mv_4/asmira_holy_avenger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, LIGHTNING_BOLT};

fn asmira_holy_avenger() -> CardIndex {
    card_index("a0397f06-d557-4c9f-af89-238ab38ada3e")
}

/// Asmira, Holy Avenger (Partial): "At the beginning of each end step, put a
/// +1/+1 counter on Asmira for each creature put into your graveyard from the
/// battlefield this turn." The count of dead creatures takes every graveyard,
/// so the ability is not played: Asmira is a 2/3 flyer. A pinned limitation:
/// this test moves when "your graveyard" can be said.
#[test]
fn asmira_holy_avenger_is_a_flyer_and_grows_on_no_end_step_yet() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[LIGHTNING_BOLT])
        .battlefield(0, &[asmira_holy_avenger(), GRIZZLY_BEARS, mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let asmira = on_battlefield(&engine, p0, asmira_holy_avenger()).unwrap();
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    assert!(keywords(&engine, asmira).contains(baylee_cards_dsl::KeywordSet::FLYING));
    assert_eq!(pt(&engine, asmira), (2, 3));
    // One of p0's own creatures goes to p0's graveyard.
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine
            .state()
            .object(asmira)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "no end-step counters are played"
    );
    assert_eq!(pt(&engine, asmira), (2, 3));
}
