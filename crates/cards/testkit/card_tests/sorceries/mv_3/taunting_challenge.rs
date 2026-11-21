//! `cards/sorceries/mv_3/taunting_challenge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn taunting_challenge() -> CardIndex {
    card_index("50c52320-9f84-4655-a91a-e09b71f9025c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Taunting Challenge (50c52320-9f84-4655-a91a-e09b71f9025c) — {1}{G}{G} Sorcery:
/// "All creatures able to block target creature this turn do so."
#[test]
fn taunting_challenge_forces_all_able_creatures_to_block_this_turn() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), fangren_hunter()],
        &[aurochs(), grizzly_bears(), llanowar_elves()],
        &[taunting_challenge()],
        &[],
    );
    let hunter = lk::obj(&engine, P0, fangren_hunter());
    let beast = lk::obj(&engine, P1, aurochs());
    let bears = lk::obj(&engine, P1, grizzly_bears());
    let tapped_elf = lk::obj(&engine, P1, llanowar_elves());

    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(tapped_elf)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    cast_from_hand(&mut engine, P0, taunting_challenge());
    aim_at(&mut engine, P0, hunter);
    lk::settle(&mut engine);

    lk::attack(&mut engine, &[hunter]);
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err()
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(beast, hunter)],
                },
            )
            .is_err()
    );
    lk::block(&mut engine, &[(beast, hunter), (bears, hunter)]);

    let mut expires = lk::game(
        SEED,
        &[forest(), forest(), forest(), fangren_hunter()],
        &[aurochs()],
        &[taunting_challenge()],
        &[],
    );
    let hunter2 = lk::obj(&expires, P0, fangren_hunter());
    cast_from_hand(&mut expires, P0, taunting_challenge());
    aim_at(&mut expires, P0, hunter2);
    lk::settle(&mut expires);

    pass_until(&mut expires, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    lk::attack(&mut expires, &[hunter2]);
    lk::block(&mut expires, &[]);
}
