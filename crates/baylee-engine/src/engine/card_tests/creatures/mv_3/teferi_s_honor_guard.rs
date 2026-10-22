//! `cards/creatures/mv_3/teferi_s_honor_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn teferi_s_honor_guard() -> CardIndex {
    card_index("8a3a8a02-b7da-4648-a772-c9efb7c332da")
}

/// `Teferi's Honor Guard` ({2}{W}, 2/2 Creature — Human Knight,
/// `Coverage::Implemented`) has flanking and `{U}{U}: This creature phases out.`
#[test]
fn teferi_s_honor_guard_has_flanking_and_phases_out() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[teferi_s_honor_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, teferi_s_honor_guard());
    pass_until(&mut engine, stack_is_empty);
    let guard = on_battlefield(&engine, p0, teferi_s_honor_guard()).unwrap();

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
                attackers: vec![(guard, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, guard)],
            },
        )
        .unwrap();
    // Resolve the flanking trigger first, then activate {U}{U} to phase out
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, blocker), (1, 1));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, teferi_s_honor_guard(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(guard)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
