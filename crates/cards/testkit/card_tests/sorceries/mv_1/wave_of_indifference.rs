//! `cards/sorceries/mv_1/wave_of_indifference.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wave_of_indifference() -> CardIndex {
    card_index("932a47fa-9b78-4c7b-8df8-65e0309d663c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Wave of Indifference (932a47fa-9b78-4c7b-8df8-65e0309d663c) — {X}{R} Sorcery:
/// "X target creatures can't block this turn."
#[test]
fn wave_of_indifference_prevents_x_target_creatures_from_blocking_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), fangren_hunter()],
        &[llanowar_elves(), grizzly_bears(), aurochs()],
        &[wave_of_indifference()],
        &[],
    );
    let attacker = lk::obj(&engine, P0, fangren_hunter());
    let b1 = lk::obj(&engine, P1, llanowar_elves());
    let b2 = lk::obj(&engine, P1, grizzly_bears());
    let b3 = lk::obj(&engine, P1, aurochs());

    tap_all_mana(&mut engine, P0);
    lk::announce(
        &mut engine,
        P0,
        wave_of_indifference(),
        0,
        2,
        &[b1, b2],
        &[],
    );
    lk::settle(&mut engine);

    let offer = lk::attack(&mut engine, &[attacker]);
    assert!(!lk::may_block(&offer, b1, attacker));
    assert!(!lk::may_block(&offer, b2, attacker));
    assert!(lk::may_block(&offer, b3, attacker));
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    let next_offer = lk::attack(&mut engine, &[attacker]);
    assert!(lk::may_block(&next_offer, b1, attacker));
}
