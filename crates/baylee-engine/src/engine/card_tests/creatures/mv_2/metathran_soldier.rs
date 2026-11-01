//! `cards/creatures/mv_2/metathran_soldier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn metathran_soldier() -> CardIndex {
    card_index("b723fcec-f208-4397-a178-e1e47b729cf1")
}

/// Metathran Soldier (b723fcec-f208-4397-a178-e1e47b729cf1) — 1/1 with "This
/// creature can't be blocked."
#[test]
fn metathran_soldier_cannot_be_blocked() {
    let mut e = lk::game(
        SEED,
        &[metathran_soldier(), llanowar_elves()],
        &[gray_ogre()],
        &[],
        &[],
    );
    let soldier = lk::obj(&e, lk::P0, metathran_soldier());
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let their_ogre = lk::obj(&e, lk::P1, gray_ogre());
    assert_eq!(pt(&e, soldier), (1, 1));
    assert!(keywords(&e, soldier).contains(KeywordSet::UNBLOCKABLE));

    let offer = lk::attack(&mut e, &[soldier, elf]);
    assert!(
        lk::may_block(&offer, their_ogre, elf),
        "normal creature may be blocked"
    );
    assert!(
        !lk::may_block(&offer, their_ogre, soldier),
        "Metathran Soldier cannot be blocked"
    );
    assert!(
        e.apply(
            lk::P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_ogre, soldier)],
            },
        )
        .is_err()
    );
    lk::block(&mut e, &[(their_ogre, elf)]);
    lk::through_combat(&mut e);
    assert_eq!(
        lk::life(&e, lk::P1),
        19,
        "Metathran Soldier dealt 1 unblockable combat damage"
    );
}
