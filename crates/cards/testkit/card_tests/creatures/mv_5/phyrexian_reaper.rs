//! `cards/creatures/mv_5/phyrexian_reaper.rs`, played.
//!
//! Oracle:
//! - "Whenever Phyrexian Reaper becomes blocked by a green creature, destroy
//!   that creature. It can't be regenerated."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn phyrexian_reaper() -> CardIndex {
    card_index("245c3681-fc47-47fe-b6e5-34c96315c135")
}

#[test]
fn phyrexian_reaper_destroys_green_blocker_ignoring_regeneration_and_spares_nongreen() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[index::RIVER_BOA, index::GRAY_OGRE, forest()])
        .hand(0, &[phyrexian_reaper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, phyrexian_reaper());
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    let reaper = on_battlefield(&engine, P0, phyrexian_reaper()).expect("Reaper resolved");
    assert_eq!(pt(&engine, reaper), (3, 3));

    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let boa = on_battlefield(&engine, P1, index::RIVER_BOA).unwrap();
    let ogre = on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap();
    let p1_forest = on_battlefield(&engine, P1, forest()).unwrap();

    attack_and_collect_blocks(&mut engine, reaper, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(boa, reaper), (ogre, reaper)],
            },
        )
        .unwrap();

    // Only the green blocker triggers Reaper (1 trigger, not 2).
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);

    // P1 passes priority to respond; regenerate River Boa in response to the trigger.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    engine
        .apply(P1, PlayerAction::ActivateManaAbility { source: p1_forest })
        .unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected P1 priority");
    };
    let regen_idx = legal
        .abilities
        .iter()
        .find_map(|(id, idx)| (*id == boa).then_some(*idx))
        .expect("River Boa regenerate offered");
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: boa,
                ability_index: regen_idx,
            },
        )
        .unwrap();

    // Resolve the regenerate shield, then let Reaper's trigger resolve.
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    assert_eq!(engine.state().object(boa).unwrap().regeneration_shields, 1);

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // River Boa was destroyed before combat damage despite its regeneration shield,
    // so only Gray Ogre (2/2) dealt combat damage (2) to Reaper.
    assert!(
        in_graveyard(&engine, P1, index::RIVER_BOA).is_some(),
        "green blocker destroyed and cannot be regenerated"
    );
    assert!(
        on_battlefield(&engine, P0, phyrexian_reaper()).is_some(),
        "Reaper took only 2 damage from Gray Ogre and survived"
    );
    assert_eq!(engine.state().object(reaper).unwrap().damage, 2);
    assert!(
        in_graveyard(&engine, P1, index::GRAY_OGRE).is_some(),
        "Gray Ogre survived the trigger, then died to 3 combat damage"
    );
}
