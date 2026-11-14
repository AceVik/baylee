//! `cards/creatures/mv_4/phyrexian_slayer.rs`, played.
//!
//! Oracle:
//! - "Flying"
//! - "Whenever Phyrexian Slayer becomes blocked by a white creature, destroy
//!   that creature. It can't be regenerated."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn phyrexian_slayer() -> CardIndex {
    card_index("da74cb57-f6de-4d05-a339-6412beb8df25")
}

#[test]
fn phyrexian_slayer_destroys_white_blocker_without_regeneration_and_spares_nonwhite() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(
            1,
            &[
                forest(),
                forest(),
                index::SERRA_ANGEL,
                index::PHANTOM_MONSTER,
            ],
        )
        .hand(0, &[phyrexian_slayer()])
        .hand(1, &[index::REGENERATE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, phyrexian_slayer());
    pass_until(&mut engine, stack_is_empty);
    let slayer = on_battlefield(&engine, P0, phyrexian_slayer()).unwrap();
    assert_eq!(pt(&engine, slayer), (2, 2));
    assert!(keywords(&engine, slayer).contains(KeywordSet::FLYING));

    // Next own turn: attack with Slayer; P1 blocks with white 4/4 Serra Angel and casts Regenerate.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let angel = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();
    attack_and_collect_blocks(&mut engine, slayer, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(angel, slayer)],
            },
        )
        .unwrap();
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::REGENERATE);
    aim_at(&mut engine, P1, angel);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Angel was destroyed despite regeneration before combat damage, leaving 2/2 Slayer unharmed.
    assert!(in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some());
    assert!(on_battlefield(&engine, P0, phyrexian_slayer()).is_some());
    assert_eq!(engine.state().object(slayer).unwrap().damage, 0);

    // Next own turn: blocking with blue 3/3 Phantom Monster does NOT trigger destruction; combat kills Slayer.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let monster = on_battlefield(&engine, P1, index::PHANTOM_MONSTER).unwrap();
    attack_and_collect_blocks(&mut engine, slayer, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(monster, slayer)],
            },
        )
        .unwrap();
    assert!(
        stack_is_empty(&engine),
        "blue blocker does not trigger Slayer"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, P0, phyrexian_slayer()).is_some());
    assert_eq!(engine.state().object(monster).unwrap().damage, 2);
}
