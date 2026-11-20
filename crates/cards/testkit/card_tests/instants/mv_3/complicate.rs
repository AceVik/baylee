//! `cards/instants/mv_3/complicate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn complicate() -> CardIndex {
    card_index("fc09cbc0-2d28-44c6-b9a9-b95e9e58bb1b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Cast normally, Complicate counters target spell unless its controller pays {3}.
#[test]
fn complicate_cast_counters_unless_controller_pays_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[complicate()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, grizzly_bears());
    let bear_spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    pass_once(&mut engine);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, complicate());
    aim(&mut engine, &[bear_spell], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p1));

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());

    // When target spell's controller pays {3}, Complicate lets the spell resolve.
    let mut paid = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[forest(); 5])
        .hand(0, &[complicate()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut paid);
    reach_their_main_phase(&mut paid, p1);

    tap_all_mana(&mut paid, p1);
    cast_with_floating(&mut paid, p1, grizzly_bears());
    let bear_spell2 = paid.state().zones.list(ZoneLocation::Stack)[0];
    pass_once(&mut paid);

    tap_all_mana(&mut paid, p0);
    cast_with_floating(&mut paid, p0, complicate());
    aim(&mut paid, &[bear_spell2], &[]);
    pass_until(&mut paid, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut paid, true);
    pass_until(&mut paid, |e| at_rest(e, p1));

    assert!(on_battlefield(&paid, p1, grizzly_bears()).is_some());
    assert_eq!(pool_total(&paid, p1), 0, "spent floating {{3}} to pay tax");
}

/// Cycling Complicate triggers before its draw: accepting `MayDo` counters target
/// spell unless its controller pays {1} (tested both unpaid and paid), while
/// declining `MayDo` never asks the spell's controller to pay {1}.
#[test]
fn complicate_cycling_counter_unless_one_accept_pay_and_decline_paths() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 9])
        .battlefield(1, &[forest(); 7])
        .hand(0, &[complicate(), complicate(), complicate()])
        .hand(1, &[grizzly_bears(), grizzly_bears(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let islands0 = all_on_battlefield(&engine, p0, island());
    let forests1 = all_on_battlefield(&engine, p1, forest());

    // 1. P1 casts Bear #1; P0 cycles Complicate, says Yes, P1 declines {1} -> countered before draw.
    tap_mana_where(&mut engine, p1, |id| forests1[..2].contains(&id));
    cast_with_floating(&mut engine, p1, grizzly_bears());
    let spell1 = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    pass_once(&mut engine);

    let hand0_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_where(&mut engine, p0, |id| islands0[..3].contains(&id));
    activate(&mut engine, p0, complicate(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[spell1], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo {
        player: decider, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(decider, p0, "P0 answers MayDo first");
    yes_no(&mut engine, true);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player: payer, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(payer, p1, "P1 answers tax {{1}}");
    yes_no(&mut engine, false);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand0_before - 1,
        "counter trigger resolved before P0's cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand0_before
    );

    // 2. P1 casts Bear #2 with {1} extra floating; P0 cycles & says Yes; P1 pays {1} -> resolves.
    tap_mana_where(&mut engine, p1, |id| forests1[2..5].contains(&id));
    cast_with_floating(&mut engine, p1, grizzly_bears());
    let spell2 = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    pass_once(&mut engine);

    tap_mana_where(&mut engine, p0, |id| islands0[3..6].contains(&id));
    activate(&mut engine, p0, complicate(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[spell2], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(all_on_battlefield(&engine, p1, grizzly_bears()).len(), 1);

    // 3. P1 casts Bear #3; P0 cycles & declines MayDo -> P1 is NOT asked to pay {1}, Bear resolves.
    tap_mana_where(&mut engine, p1, |id| forests1[5..7].contains(&id));
    cast_with_floating(&mut engine, p1, grizzly_bears());
    let spell3 = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    pass_once(&mut engine);

    tap_mana_where(&mut engine, p0, |id| islands0[6..9].contains(&id));
    activate(&mut engine, p0, complicate(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[spell3], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    // Settling without any further YesNo proves P1 was not prompted to pay {1}.
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(all_on_battlefield(&engine, p1, grizzly_bears()).len(), 2);
}
