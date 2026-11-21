//! `cards/sorceries/mv_3/alluring_scent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn alluring_scent() -> CardIndex {
    card_index("ad218276-a44b-4a61-8e42-26a27929bbbb")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Alluring Scent (ad218276-a44b-4a61-8e42-26a27929bbbb) — {1}{G}{G} Sorcery:
/// "All creatures able to block target creature this turn do so."
#[test]
fn alluring_scent_forces_every_able_blocker_to_block_target_creature() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), fangren_hunter()],
        &[aurochs(), grizzly_bears(), llanowar_elves()],
        &[alluring_scent()],
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

    cast_from_hand(&mut engine, P0, alluring_scent());
    aim_at(&mut engine, P0, hunter);
    lk::settle(&mut engine);

    let offer = lk::attack(&mut engine, &[hunter]);
    assert!(lk::may_block(&offer, beast, hunter));
    assert!(lk::may_block(&offer, bears, hunter));
    assert!(!lk::may_block(&offer, tapped_elf, hunter));

    assert!(
        engine
            .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "declaring zero blockers violates the requirement"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, hunter)],
                },
            )
            .is_err(),
        "leaving an able blocker back violates the requirement"
    );
    lk::block(&mut engine, &[(beast, hunter), (bears, hunter)]);
}

/// Alluring Scent — "this turn": the lure expires at end of turn.
#[test]
fn alluring_scent_requirement_expires_after_the_turn_ends() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), fangren_hunter()],
        &[aurochs()],
        &[alluring_scent()],
        &[],
    );
    let hunter = lk::obj(&engine, P0, fangren_hunter());

    cast_from_hand(&mut engine, P0, alluring_scent());
    aim_at(&mut engine, P0, hunter);
    lk::settle(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    lk::attack(&mut engine, &[hunter]);
    lk::block(&mut engine, &[]);
}
