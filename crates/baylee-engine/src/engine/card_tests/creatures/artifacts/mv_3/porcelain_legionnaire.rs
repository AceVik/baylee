//! `cards/creatures/artifacts/mv_3/porcelain_legionnaire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn porcelain_legionnaire() -> CardIndex {
    card_index("66442396-f804-47f3-8daa-0d43d8cfb482")
}

/// Porcelain Legionnaire — {2}{W/P}, 3/1 first strike. Cast off a Plains and
/// two Forests the {W/P} is paid with the {W}: no life.
#[test]
fn porcelain_legionnaire_paid_with_white_costs_no_life() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[porcelain_legionnaire()], &[plains(), forest(), forest()]);

    cast_and_settle(&mut engine, porcelain_legionnaire());

    let id = on_battlefield(&engine, p0, porcelain_legionnaire()).expect("it resolved");
    assert_eq!(pt(&engine, id), (3, 1));
    assert!(keywords(&engine, id).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(life_of(&engine, 0), 20);
}

/// Cast with only the generic {2} available, the {W/P} is paid with 2 life.
#[test]
fn porcelain_legionnaire_paid_with_life_costs_two() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[porcelain_legionnaire()], &[forest(), forest()]);

    cast_and_settle(&mut engine, porcelain_legionnaire());

    assert!(on_battlefield(&engine, p0, porcelain_legionnaire()).is_some());
    assert_eq!(life_of(&engine, 0), 18, "{{W/P}} paid with 2 life");
}

/// First strike: blocked by a 1/1 that deals 1 damage — lethal to a 3/1 —
/// the Legionnaire deals its damage first and takes none.
#[test]
fn porcelain_legionnaire_kills_its_blocker_before_it_can_hurt_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[porcelain_legionnaire()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, porcelain_legionnaire()).expect("seated");
    let blocker = on_battlefield(&engine, p1, quiet_creature()).expect("seated");

    attack_to_blocks(&mut engine, &[attacker]);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p1, quiet_creature()).is_none(), "the blocker died");
    assert!(
        on_battlefield(&engine, p0, porcelain_legionnaire()).is_some(),
        "and dealt no damage back"
    );
}
