//! `cards/artifacts/equipment/mv_2/echo_circlet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn echo_circlet() -> CardIndex {
    card_index("17134950-fdb3-48d0-b541-419db673f4c9")
}

/// Echo Circlet: "Equipped creature can block an additional creature each
/// combat. Equip {1}". The opponent attacks with two creatures; only the
/// equipped creature is offered as able to block both.
#[test]
fn echo_circlet_lets_the_equipped_creature_block_two_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        echo_circlet(),
        llanowar_elves(),
        1,
        &[],
        &[goblin_piker(), goblin_piker()],
        &[],
    );
    r.equip(1);
    let attackers = all_on_battlefield(&r.engine, p1, goblin_piker());
    assert_eq!(attackers.len(), 2, "two attackers");
    let Pending::ChooseBlockers {
        player,
        blockers,
        capacity,
        ..
    } = r.defend_against(&attackers)
    else {
        unreachable!("defend_against returns the blockers question")
    };
    assert_eq!(player, p0, "we are the defender");
    let host_offer = blockers
        .iter()
        .find(|b| b.blocker == r.host)
        .expect("the equipped creature may block");
    assert!(
        attackers.iter().all(|a| host_offer.attackers.contains(a)),
        "and either attacker: {host_offer:?}"
    );
    assert!(
        capacity
            .iter()
            .any(|c| c.blocker == r.host && c.most == Some(2)),
        "the equipped creature may block two: {capacity:?}"
    );
    assert!(
        capacity.iter().all(|c| c.blocker != r.bystander),
        "the creature nobody equipped blocks one at most: {capacity:?}"
    );
    r.engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(r.host, attackers[0]), (r.host, attackers[1])],
            },
        )
        .expect("one creature blocking both attackers is a legal declaration");
}
