//! `cards/sorceries/mv_3/panic_attack.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn panic_attack() -> CardIndex {
    card_index("c1346677-d89e-49d0-9819-08805f00bb64")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Panic Attack (c1346677-d89e-49d0-9819-08805f00bb64) — {2}{R} Sorcery:
/// "Up to three target creatures can't block this turn."
#[test]
fn panic_attack_stops_up_to_three_target_creatures_from_blocking_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), fangren_hunter()],
        &[
            llanowar_elves(),
            grizzly_bears(),
            savannah_lions(),
            aurochs(),
        ],
        &[panic_attack()],
        &[],
    );
    let attacker = lk::obj(&engine, P0, fangren_hunter());
    let c1 = lk::obj(&engine, PlayerId::new(1), llanowar_elves());
    let c2 = lk::obj(&engine, PlayerId::new(1), grizzly_bears());
    let c3 = lk::obj(&engine, PlayerId::new(1), savannah_lions());
    let c4 = lk::obj(&engine, PlayerId::new(1), aurochs());

    cast_from_hand(&mut engine, P0, panic_attack());
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 3));
    lk::aim(&mut engine, &[c1, c2, c3], &[]);
    lk::settle(&mut engine);

    let offer = lk::attack(&mut engine, &[attacker]);
    for stopped in [c1, c2, c3] {
        assert!(!lk::may_block(&offer, stopped, attacker));
    }
    assert!(
        lk::may_block(&offer, c4, attacker),
        "fourth untargeted creature may still block"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    // Next turn of P0, c1 may block again.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    let next_offer = lk::attack(&mut engine, &[attacker]);
    assert!(lk::may_block(&next_offer, c1, attacker));
}

/// Panic Attack — "Up to three": choosing zero targets is legal.
#[test]
fn panic_attack_may_be_cast_with_zero_targets() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[aurochs()],
        &[panic_attack()],
        &[],
    );
    cast_from_hand(&mut engine, P0, panic_attack());
    lk::aim(&mut engine, &[], &[]);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, panic_attack()).is_some());
}
