//! Oracle: target creature gets +2/+2 and first strike until end of turn.
#[allow(clippy::wildcard_imports)]
use super::*;
fn zealous_strike() -> CardIndex {
    card_index("d820ddf4-315d-470a-8d66-663a76a240c6")
}
/// Checklist: creature target only; exact +2/+2 and first strike; temporary duration.
#[test]
fn pumps_and_grants_first_strike_for_this_turn() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[zealous_strike()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let c = on_battlefield(&g, p, llanowar_elves()).unwrap();
    cast_from_hand(&mut g, p, zealous_strike());
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        panic!("creature target required")
    };
    assert_eq!(options, vec![c]);
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![c],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(pt(&g, c), (3, 3));
    assert!(keywords(&g, c).contains(KeywordSet::FIRST_STRIKE));
    pass_until(&mut g, |e| e.state().turn.active != p);
    assert_eq!(pt(&g, c), (1, 1), "the +2/+2 expires at cleanup");
    assert!(!keywords(&g, c).contains(KeywordSet::FIRST_STRIKE));
}
