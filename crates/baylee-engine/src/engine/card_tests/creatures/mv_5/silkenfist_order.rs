//! `cards/creatures/mv_5/silkenfist_order.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn silkenfist_order_untaps_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[card_index("5cca974b-1a6c-485f-adc0-81d2b7dca286")])
        .battlefield(0, &[plains(); 5])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("5cca974b-1a6c-485f-adc0-81d2b7dca286"),
    );
    pass_until(&mut engine, stack_is_empty);

    let order = on_battlefield(
        &engine,
        p0,
        card_index("5cca974b-1a6c-485f-adc0-81d2b7dca286"),
    )
    .unwrap();
    let blocker = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(order, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, order));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, order)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, order));
}
