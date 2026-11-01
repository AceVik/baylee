//! `cards/creatures/mv_8/scion_of_darkness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Scion of Darkness (`bd940cab-d23a-44bf-ab96-8aa1dcd3f0ff`) — {5}{B}{B}{B} Creature — Avatar 6/6:
/// "Trample
/// Whenever this creature deals combat damage to a player, you may put target creature card from that player's graveyard onto the battlefield under your control.
/// Cycling {3}"
#[test]
fn scion_of_darkness_reanimates_from_damaged_players_graveyard_and_cycles() {
    let scion_card = card_index("bd940cab-d23a-44bf-ab96-8aa1dcd3f0ff");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[scion_card, swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[scion_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    bury(&mut engine, &[opp_bear]);

    // Cycle the copy in hand for {3}.
    let hand_before = hand_size(&engine, 0);
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, scion_card, 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, scion_card).is_some());
    assert_eq!(hand_size(&engine, 0), hand_before);

    // Attack with the battlefield Scion of Darkness and reanimate Grizzly Bears from p1's graveyard.
    let scion = on_battlefield(&engine, p0, scion_card).unwrap();
    let dead_bear = in_graveyard(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, scion).contains(KeywordSet::TRAMPLE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(scion, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![dead_bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(life_of(&engine, 1), 14);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
