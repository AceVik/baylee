//! `cards/creatures/artifacts/mv_2/hydra_assault_robot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hydra_assault_robot() -> CardIndex {
    card_index("a2558476-5506-4a09-93fc-b8ab33c98985")
}

/// HYDRA Assault Robot: a 1/3; "Whenever another Villain and/or artifact you
/// control enters, this creature deals 1 damage to target opponent." A Gold
/// Myr entering costs the opponent one life, aimed at them.
#[test]
fn hydra_assault_robot_pings_the_opponent_when_another_artifact_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[gold_myr()],
        &[hydra_assault_robot(), forest(), forest()],
        &[],
    );
    let hydra = on_battlefield(&engine, p0, hydra_assault_robot()).expect("seated");
    assert_eq!(pt(&engine, hydra), (1, 3));
    cast_and_land(&mut engine, gold_myr());
    settle_aiming_at_opponent(&mut engine);
    assert_eq!(life_of(&engine, 1), 19, "one damage");
    assert_eq!(life_of(&engine, 0), 20);
}

/// A creature that is neither an artifact nor a Villain does nothing.
#[test]
fn hydra_assault_robot_ignores_a_plain_creature() {
    let mut engine = table(
        &[grizzly_bears()],
        &[hydra_assault_robot(), forest(), forest()],
        &[],
    );
    cast_and_land(&mut engine, grizzly_bears());
    let asked = settle_aiming_at_opponent(&mut engine);
    assert!(!asked, "no trigger, no target question");
    assert_eq!(life_of(&engine, 1), 20);
}

/// "Another": the Robot entering itself pings nobody.
#[test]
fn hydra_assault_robot_entering_pings_nobody() {
    let mut engine = table(&[hydra_assault_robot()], &[mountain(), forest()], &[]);
    cast_and_land(&mut engine, hydra_assault_robot());
    let asked = settle_aiming_at_opponent(&mut engine);
    assert!(!asked);
    assert_eq!(life_of(&engine, 1), 20);
}
