//! `cards/creatures/mv_5/savage_firecat.rs`, played.
//!
//! Oracle:
//! - "Trample"
//! - "This creature enters with seven +1/+1 counters on it."
//! - "Whenever you tap a land for mana, remove a +1/+1 counter from this
//!   creature."

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn savage_firecat() -> CardIndex {
    card_index("084e11d9-dd1d-4e8e-afc1-9be4ee04a510")
}

#[test]
fn savage_firecat_enters_with_seven_counters_and_drains_only_on_your_lands() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                sol_ring(),
            ],
        )
        .battlefield(1, &[forest()])
        .hand(0, &[savage_firecat()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mountains = all_on_battlefield(&engine, P0, mountain());
    for &m in &mountains[..5] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, savage_firecat());
    pass_until(&mut engine, stack_is_empty);

    let firecat = on_battlefield(&engine, P0, savage_firecat()).expect("Firecat resolved");
    assert_eq!(counters_on(&engine, firecat, CounterKind::P1P1), 7);
    assert_eq!(pt(&engine, firecat), (7, 7));
    assert!(keywords(&engine, firecat).contains(KeywordSet::TRAMPLE));

    // Tapping a land you control for mana triggers removing one +1/+1 counter.
    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: mountains[5],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, firecat, CounterKind::P1P1), 6);
    assert_eq!(pt(&engine, firecat), (6, 6));

    // Tapping a nonland mana source (Sol Ring) does not trigger Firecat.
    let ring = on_battlefield(&engine, P0, sol_ring()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: ring,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(stack_is_empty(&engine));
    assert_eq!(counters_on(&engine, firecat, CounterKind::P1P1), 6);

    // Opponent tapping a land for mana does not trigger Firecat.
    reach_their_main_phase(&mut engine, P1);
    let p1_forest = on_battlefield(&engine, P1, forest()).unwrap();
    engine
        .apply(P1, PlayerAction::ActivateManaAbility { source: p1_forest })
        .unwrap();
    assert!(stack_is_empty(&engine));
    assert_eq!(counters_on(&engine, firecat, CounterKind::P1P1), 6);

    // Next turn: tapping 6 lands removes all remaining counters, sending 0/0 Firecat to graveyard.
    assert!(walk_to_own_main(&mut engine, P0));
    for m in all_on_battlefield(&engine, P0, mountain()) {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }
    assert!(
        in_graveyard(&engine, P0, savage_firecat()).is_some(),
        "last +1/+1 counter removed leaves a 0/0 that dies to SBA"
    );
}
