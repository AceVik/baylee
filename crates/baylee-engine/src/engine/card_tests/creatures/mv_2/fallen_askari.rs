//! `cards/creatures/mv_2/fallen_askari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fallen_askari() -> CardIndex {
    card_index("4956008e-d912-48ba-9b82-176258281060")
}

#[test]
fn fallen_askari_has_flanking_and_cannot_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[fallen_askari()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, fallen_askari());
    pass_until(&mut engine, stack_is_empty);

    let askari = on_battlefield(&engine, p0, fallen_askari()).unwrap();
    assert_eq!(pt(&engine, askari), (2, 2));
    assert!(keywords(&engine, askari).contains(KeywordSet::FLANKING));

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(opp_bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[0].life, 18);
}
