//! `cards/creatures/artifacts/mv_2/haunted_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn haunted_guardian() -> CardIndex {
    card_index("20b9bc53-7f6d-4616-a2df-5783ab577707")
}

/// Haunted Guardian: a 2/1 with "Defender, first strike". It is not offered
/// as an attacker, where a Bears beside it is.
#[test]
fn haunted_guardian_has_defender_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[haunted_guardian(), grizzly_bears()], &[]);
    let guardian = on_battlefield(&engine, p0, haunted_guardian()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, guardian), (2, 1));
    assert!(keywords(&engine, guardian).contains(KeywordSet::DEFENDER));
    assert!(keywords(&engine, guardian).contains(KeywordSet::FIRST_STRIKE));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&bears));
    assert!(!attackers.contains(&guardian));
}

/// First strike: blocking a 2/2 Bears, the 2/1 deals its damage first and the
/// Bears die without dealing any; the Guardian survives. Without first
/// strike both would die.
#[test]
fn haunted_guardian_kills_a_bears_before_it_strikes_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[haunted_guardian()], &[grizzly_bears()]);
    let guardian = on_battlefield(&engine, p0, haunted_guardian()).expect("seated");
    reach_their_main_phase(&mut engine, p1);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, bears, p0);
    assert!(options.iter().any(|o| o.blocker == guardian));
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(guardian, bears)],
            },
        )
        .expect("the Guardian blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "the Bears died to first-strike damage"
    );
    assert!(
        on_battlefield(&engine, p0, haunted_guardian()).is_some(),
        "and never struck back"
    );
    assert_eq!(life_of(&engine, 0), 20, "the block stopped the attack");
}
