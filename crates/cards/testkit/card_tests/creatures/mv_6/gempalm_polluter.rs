//! `cards/creatures/mv_6/gempalm_polluter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn gempalm_polluter() -> CardIndex {
    card_index("bd532f8d-789f-4ee1-a2f3-ae30e8282c76")
}

fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

fn firdoch_core() -> CardIndex {
    card_index("126c75c0-414b-4e55-8747-2b5566ce716f")
}

/// Cycling Gempalm Polluter for `{B}{B}` lets you make target player lose life
/// equal to the number of Zombies on the battlefield across both players
/// (including Kindred Changeling permanents, excluding the cycled Polluter in
/// graveyard and non-Zombies), and may be declined while still drawing a card.
#[test]
fn gempalm_polluter_counts_all_battlefield_zombies_and_may_be_declined() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                scathe_zombies(),
                firdoch_core(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[gempalm_polluter(), gempalm_polluter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let swamps = all_on_battlefield(&engine, p0, swamp());

    // 1. Cycle first Polluter and accept: 3 Zombies on battlefield (P0 Scathe
    // Zombies, P0 Firdoch Core changeling, P1 Festering Goblin Zombie Goblin).
    tap_mana_where(&mut engine, p0, |id| id == swamps[0] || id == swamps[1]);
    activate(&mut engine, p0, gempalm_polluter(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(life(&engine, p1), 17, "lost 3 life for 3 Zombies on board");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // 2. Cycle second Polluter and decline MayDo -> no life lost, still draws.
    tap_mana_where(&mut engine, p0, |id| id == swamps[2] || id == swamps[3]);
    activate(&mut engine, p0, gempalm_polluter(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(life(&engine, p1), 17, "declining MayDo causes no life loss");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
