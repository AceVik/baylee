//! `cards/creatures/mv_3/hulking_ogre.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn hulking_ogre() -> CardIndex {
    card_index("a95aa9ba-fa61-4a3d-a493-1ea790bd20ad")
}

/// Hulking Ogre (a95aa9ba-fa61-4a3d-a493-1ea790bd20ad) — 3/3 with "This
/// creature can't block." It attacks for 3, and when an opponent attacks it is
/// excluded from the blocker offer while a normal creature can block.
#[test]
fn hulking_ogre_attacks_for_three_and_cannot_block() {
    let mut e = lk::game(
        SEED,
        &[hulking_ogre(), llanowar_elves()],
        &[gray_ogre()],
        &[],
        &[],
    );
    let ogre = lk::obj(&e, lk::P0, hulking_ogre());
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let their_ogre = lk::obj(&e, lk::P1, gray_ogre());
    assert_eq!(pt(&e, ogre), (3, 3));

    // P0 attacks with Hulking Ogre; P1 does not block and takes 3 damage.
    let _ = lk::attack(&mut e, &[ogre]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);

    // Put a second, untapped Hulking Ogre on P0's side for P1's combat.
    let mut e2 = lk::game(
        SEED,
        &[hulking_ogre(), llanowar_elves()],
        &[gray_ogre()],
        &[],
        &[],
    );
    let ogre2 = lk::obj(&e2, lk::P0, hulking_ogre());
    let elf2 = lk::obj(&e2, lk::P0, llanowar_elves());
    let their_ogre2 = lk::obj(&e2, lk::P1, gray_ogre());
    let _ = (elf, their_ogre);

    reach_their_main_phase(&mut e2, lk::P1);
    let offer = lk::attack(&mut e2, &[their_ogre2]);
    assert!(
        lk::may_block(&offer, elf2, their_ogre2),
        "Llanowar Elves may block"
    );
    assert!(
        !lk::may_block(&offer, ogre2, their_ogre2),
        "Hulking Ogre cannot block"
    );
    assert!(
        e2.apply(
            lk::P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre2, their_ogre2)],
            },
        )
        .is_err(),
        "declaring Hulking Ogre as a blocker is rejected"
    );
}
