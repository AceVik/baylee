//! `cards/creatures/mv_3/zuo_ci_the_mocking_sage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn zuo_ci() -> CardIndex {
    card_index("ae8773a3-05f2-4074-9a53-033b0c127235")
}

/// Zuo Ci, the Mocking Sage ({1}{G}{G} 1/2 Legendary Human Advisor):
/// "Hexproof; Zuo Ci can't be blocked by creatures with horsemanship."
#[test]
fn zuo_ci_has_hexproof_and_cannot_be_blocked_by_horsemanship() {
    let mut e = lk::game(
        SEED,
        &[zuo_ci(), llanowar_elves()],
        &[index::SHU_CAVALRY, index::GRIZZLY_BEARS],
        &[giant_growth()],
        &[index::SHOCK],
    );
    let sage = lk::obj(&e, lk::P0, zuo_ci());
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let cavalry = lk::obj(&e, lk::P1, index::SHU_CAVALRY);
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    // P0 may target Zuo Ci with Giant Growth; P1's Shock may not target Zuo Ci (Hexproof).
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, lk::P0, giant_growth());
    let Pending::ChooseTargets {
        options: my_opts, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert!(my_opts.contains(&sage));
    lk::aim(&mut e, &[sage], &[]);

    lk::float(&mut e, lk::P1, &[(ManaColor::Red, 1)]);
    e.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut e, lk::P1, index::SHOCK);
    let Pending::ChooseTargets {
        options: their_opts,
        ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert!(!their_opts.contains(&sage));
    assert!(their_opts.contains(&elf));
    lk::aim(&mut e, &[elf], &[]);
    lk::settle(&mut e);

    // Horsemanship creature cannot block Zuo Ci; non-Horsemanship Bear can.
    let offer = lk::attack(&mut e, &[sage]);
    assert!(!lk::may_block(&offer, cavalry, sage));
    assert!(lk::may_block(&offer, bear, sage));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 16);
}
