//! `cards/creatures/artifacts/mv_2/wall_of_tanglecord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wall_of_tanglecord() -> CardIndex {
    card_index("423bebcc-8fa0-49c4-aa22-728bf51a535b")
}

/// The opponent's Storm Crow attacks seat 0, whose Wall of Tanglecord may
/// pay `{G}` for reach once the attack is declared. Returns the Wall, the
/// Bears beside it and the blockers the question then offers.
fn crow_attacks(pay_for_reach: bool) -> (ObjectId, ObjectId, Vec<crate::choice::BlockOption>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[wall_of_tanglecord(), grizzly_bears(), forest()],
        &[storm_crow()],
    );
    let wall = on_battlefield(&engine, p0, wall_of_tanglecord()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    reach_their_main_phase(&mut engine, p1);
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crow, Defender::Player(p0))],
            },
        )
        .expect("the Crow attacks");
    // The attacker passes; the defender now has priority in the declare
    // attackers step.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    if pay_for_reach {
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, wall_of_tanglecord(), 0);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    if pay_for_reach {
        assert!(
            keywords(&engine, wall).contains(KeywordSet::REACH),
            "reach until end of turn"
        );
    }
    (wall, bears, blockers)
}

/// Wall of Tanglecord: a 0/6 with "Defender" and "{G}: This creature gains
/// reach until end of turn". It is not an attacker.
#[test]
fn wall_of_tanglecord_has_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[wall_of_tanglecord(), grizzly_bears()], &[]);
    let wall = on_battlefield(&engine, p0, wall_of_tanglecord()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, wall), (0, 6));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&bears));
    assert!(!attackers.contains(&wall));
}

/// Without the `{G}` it cannot block a flier; with it it can, and the Bears,
/// which never gain reach, still cannot.
#[test]
fn wall_of_tanglecord_blocks_a_flier_only_after_paying_for_reach() {
    let (wall, _, without) = crow_attacks(false);
    assert!(
        !without.iter().any(|o| o.blocker == wall),
        "no reach yet: {without:?}"
    );
    let (wall, bears2, with) = crow_attacks(true);
    assert!(with.iter().any(|o| o.blocker == wall), "reach: {with:?}");
    assert!(
        !with.iter().any(|o| o.blocker == bears2),
        "the Bears still cannot"
    );
}
