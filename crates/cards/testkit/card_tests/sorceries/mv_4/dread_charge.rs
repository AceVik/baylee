//! `cards/sorceries/mv_4/dread_charge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dread_charge() -> CardIndex {
    card_index("6b7933b3-821a-4d64-9a10-af8a69fa008e")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Dread Charge (6b7933b3-821a-4d64-9a10-af8a69fa008e) — {3}{B} Sorcery:
/// "Black creatures you control can't be blocked this turn except by black
/// creatures."
#[test]
fn dread_charge_prevents_nonblack_blockers_on_controlled_black_creatures() {
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            swamp(),
            swamp(),
            swamp(),
            festering_goblin(),
            grizzly_bears(),
        ],
        &[aurochs(), lotleth_troll()],
        &[dread_charge()],
        &[],
    );
    let my_black = lk::obj(&engine, P0, festering_goblin());
    let my_green = lk::obj(&engine, P0, grizzly_bears());
    let their_nonblack = lk::obj(&engine, P1, aurochs());
    let their_black = lk::obj(&engine, P1, lotleth_troll());

    cast_from_hand(&mut engine, P0, dread_charge());
    lk::settle(&mut engine);

    let offer = lk::attack(&mut engine, &[my_black, my_green]);
    assert!(
        !lk::may_block(&offer, their_nonblack, my_black),
        "nonblack creature cannot block controlled black creature"
    );
    assert!(
        lk::may_block(&offer, their_black, my_black),
        "black creature may still block controlled black creature"
    );
    assert!(
        lk::may_block(&offer, their_nonblack, my_green),
        "controlled nonblack creature is unaffected"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(their_nonblack, my_black)],
                },
            )
            .is_err()
    );
    lk::block(&mut engine, &[(their_black, my_black)]);
}

/// Dread Charge — "this turn": evasion expires after cleanup.
#[test]
fn dread_charge_evasion_expires_at_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), swamp(), festering_goblin()],
        &[aurochs()],
        &[dread_charge()],
        &[],
    );
    let my_black = lk::obj(&engine, P0, festering_goblin());
    let their_nonblack = lk::obj(&engine, P1, aurochs());

    cast_from_hand(&mut engine, P0, dread_charge());
    lk::settle(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    let offer = lk::attack(&mut engine, &[my_black]);
    assert!(lk::may_block(&offer, their_nonblack, my_black));
}
