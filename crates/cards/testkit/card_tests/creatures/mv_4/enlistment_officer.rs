//! `cards/creatures/mv_4/enlistment_officer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn enlistment_officer() -> CardIndex {
    card_index("e9517fc3-d1bc-42f7-b052-9f629b302735")
}

fn shu_general() -> CardIndex {
    card_index("0fef6729-b044-4daf-834a-ac8c9711ad8a")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Enlistment Officer (e9517fc3-d1bc-42f7-b052-9f629b302735) — `{3}{W}` 2/3
/// Creature — Human Soldier:
/// - First strike
/// - When this creature enters, reveal the top four cards of your library. Put all
///   Soldier cards revealed this way into your hand and the rest on the bottom of
///   your library in any order.
#[test]
fn enlistment_officer_puts_revealed_soldiers_into_hand_and_orders_rest_on_bottom() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(
            0,
            &[
                enlistment_officer(),
                shu_general(),
                wu_elite_cavalry(),
                grizzly_bears(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let miss_bear = hand_to_library_top(&mut engine, P0, grizzly_bears());
    let _hit_a = hand_to_library_top(&mut engine, P0, shu_general());
    let miss_island = hand_to_library_top(&mut engine, P0, island());
    let _hit_b = hand_to_library_top(&mut engine, P0, wu_elite_cavalry());

    cast_from_hand(&mut engine, P0, enlistment_officer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![vec![miss_island, miss_bear]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let officer = on_battlefield(&engine, P0, enlistment_officer()).expect("Officer entered");
    assert_eq!(pt(&engine, officer), (2, 3));
    assert!(keywords(&engine, officer).contains(KeywordSet::FIRST_STRIKE));
    assert!(in_hand(&engine, P0, shu_general()).is_some());
    assert!(in_hand(&engine, P0, wu_elite_cavalry()).is_some());
    assert!(in_hand(&engine, P0, grizzly_bears()).is_none());

    let lib = engine.state().zones.list(ZoneLocation::Library(P0));
    assert!(lib.contains(&miss_island) && lib.contains(&miss_bear));

    // Verify first strike in combat against a 2/2 Grizzly Bears: kills Bears and takes 0 damage.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let p1_bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(officer, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(p1_bear, officer)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(engine.state().object(officer).unwrap().damage, 0);
}
