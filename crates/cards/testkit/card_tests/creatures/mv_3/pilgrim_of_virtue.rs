//! `cards/creatures/mv_3/pilgrim_of_virtue.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn pilgrim_of_virtue() -> CardIndex {
    card_index("e2f73743-95e7-4f03-9b98-5ab5ebc4b724")
}

/// Pilgrim of Virtue ({2}{W} 1/3 Human Cleric):
/// "Protection from black
/// {W}, Sacrifice this creature: The next time a black source of your choice
/// would deal damage this turn, prevent that damage."
#[test]
fn pilgrim_of_virtue_has_protection_from_black_and_prevents_next_damage_from_chosen_black_source() {
    let mut e = lk::game(
        SEED,
        &[pilgrim_of_virtue()],
        &[scathe_zombies(), grizzly_bears()],
        &[],
        &[],
    );
    let pilgrim = lk::obj(&e, lk::P0, pilgrim_of_virtue());
    let zombie = lk::obj(&e, lk::P1, scathe_zombies());
    let bear = lk::obj(&e, lk::P1, grizzly_bears());

    // Protection from black: Scathe Zombies cannot block Pilgrim of Virtue.
    let blocks = lk::attack(&mut e, &[pilgrim]);
    assert!(!lk::may_block(&blocks, zombie, pilgrim));
    assert!(lk::may_block(&blocks, bear, pilgrim));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    // On P1's turn, sacrifice Pilgrim to prevent Scathe Zombies' next combat damage to P0.
    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[zombie]);
    lk::block(&mut e, &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == lk::P0),
    );

    lk::float(&mut e, lk::P0, &[(ManaColor::White, 1)]);
    activate(&mut e, lk::P0, pilgrim_of_virtue(), 1);
    let selected = baylee_core::ids::DamageSourceRef {
        object: zombie,
        version: e.state().object(zombie).unwrap().version,
    };
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        options, choice, ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(options, vec![selected], "only black sources are offered");
    e.apply(
        lk::P0,
        PlayerAction::ChooseDamageSource {
            choice,
            source: selected,
        },
    )
    .unwrap();
    lk::through_combat(&mut e);

    assert!(in_graveyard(&e, lk::P0, pilgrim_of_virtue()).is_some());
    assert_eq!(lk::life(&e, lk::P0), 20);
}
