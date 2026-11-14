//! `cards/creatures/mv_3/krosan_avenger.rs`, played.
//!
//! Oracle:
//! - "Trample"
//! - "Threshold — {1}{G}: Regenerate this creature. Activate only if seven or
//!   more cards are in your graveyard."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn krosan_avenger() -> CardIndex {
    card_index("8991b367-6fc1-4de7-a1ee-4edb57c44fc2")
}

#[test]
fn krosan_avenger_tramples_and_regenerates_only_at_threshold() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), mountain(), krosan_avenger()])
        .battlefield(1, &[index::LLANOWAR_ELVES])
        .hand(0, &[index::SHOCK, bojuka_bog()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let avenger = on_battlefield(&engine, P0, krosan_avenger()).unwrap();
    assert_eq!(pt(&engine, avenger), (3, 1));
    assert!(keywords(&engine, avenger).contains(KeywordSet::TRAMPLE));

    // At 6 cards in graveyard (< 7), regenerate ability is not offered.
    seed_graveyard(&mut engine, P0, 6);
    engine.refresh_offer();
    assert!(!ability_offered(&engine, avenger, 0));

    // Cast Shock at P1 (7th card into graveyard) using only the Mountain, leaving Forests for combat.
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .unwrap();
    cast_with_floating(&mut engine, P0, index::SHOCK);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        7
    );
    assert!(ability_offered(&engine, avenger, 0));

    // Attack with 3/1 Trample; P1 blocks with 1/1 Llanowar Elves.
    let elves = on_battlefield(&engine, P1, index::LLANOWAR_ELVES).unwrap();
    attack_and_collect_blocks(&mut engine, avenger, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, avenger)],
            },
        )
        .unwrap();

    // Activate Threshold regenerate before combat damage resolves.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    tap_all_mana(&mut engine, P0);
    assert!(ability_offered(&engine, avenger, 0));
    activate(&mut engine, P0, krosan_avenger(), 0);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Combat damage dealt 1 to Elves (destroyed), 2 trample to P1 (18 -> 16),
    // and regeneration saved 3/1 Avenger from 1 lethal damage (tapped, 0 damage).
    assert!(in_graveyard(&engine, P1, index::LLANOWAR_ELVES).is_some());
    assert_eq!(engine.state().players[1].life, 16);
    assert!(on_battlefield(&engine, P0, krosan_avenger()).is_some());
    assert!(is_tapped(&engine, avenger));
    assert_eq!(engine.state().object(avenger).unwrap().damage, 0);

    // Exiling own graveyard drops count to 0 (< 7): Threshold turns off.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    play_land(&mut engine, P0, bojuka_bog());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    tap_all_mana(&mut engine, P0);
    assert!(!ability_offered(&engine, avenger, 0));
}
