//! `cards/creatures/mv_2/dwarven_soldier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Player 0 attacks with the Soldier into `blockers`, which all block it.
/// Returns the game at the point the "blocked by" trigger has resolved,
/// before any combat damage.
fn soldier_attacks(blockers: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("88f9075c-ea96-4f8d-b0d1-e5e244e824c1")])
        .battlefield(1, blockers)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let soldier = on_battlefield(&e, P0, card_index("88f9075c-ea96-4f8d-b0d1-e5e244e824c1"))
        .expect("the Soldier is out");
    let theirs: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| e.state().object(*id).is_some_and(|o| o.controller == P1))
        .collect();
    assert_eq!(theirs.len(), blockers.len());
    assert_eq!(pt(&e, soldier), (2, 1));

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(soldier, Defender::Player(P1))],
        },
    )
    .expect("the Soldier attacks");
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: theirs.iter().map(|b| (*b, soldier)).collect(),
        },
    )
    .expect("they block");
    pass_until(&mut e, stack_is_empty);
    (e, soldier)
}

/// Player 1 attacks with `attacker`; the Soldier blocks it.
fn soldier_blocks(attacker: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("88f9075c-ea96-4f8d-b0d1-e5e244e824c1")])
        .battlefield(1, &[attacker])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let soldier = on_battlefield(&e, P0, card_index("88f9075c-ea96-4f8d-b0d1-e5e244e824c1"))
        .expect("the Soldier is out");
    let enemy = on_battlefield(&e, P1, attacker).expect("the attacker is out");

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::DeclareAttackers
    });
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(enemy, Defender::Player(P0))],
        },
    )
    .expect("they attack");
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(soldier, enemy)],
        },
    )
    .expect("the Soldier blocks");
    pass_until(&mut e, stack_is_empty);
    (e, soldier)
}

/// Dwarven Soldier (88f9075c-ea96-4f8d-b0d1-e5e244e824c1) — "Whenever this
/// creature blocks or becomes blocked by one or more Orcs, this creature
/// gets +0/+2 until end of turn."
#[test]
fn dwarven_soldier_blocked_by_an_orc_gets_two_toughness() {
    let (e, soldier) = soldier_attacks(&[card_index("0d058991-3751-4afe-80c9-79f0c6e10282")]);
    assert_eq!(pt(&e, soldier), (2, 3));
}

#[test]
fn dwarven_soldier_blocked_by_two_orcs_gets_the_bonus_once() {
    let (e, soldier) = soldier_attacks(&[
        card_index("0d058991-3751-4afe-80c9-79f0c6e10282"),
        card_index("0d058991-3751-4afe-80c9-79f0c6e10282"),
    ]);
    assert_eq!(
        pt(&e, soldier),
        (2, 3),
        "one trigger for the group, not two"
    );
}

#[test]
fn dwarven_soldier_blocked_by_a_non_orc_gets_nothing() {
    let (e, soldier) = soldier_attacks(&[grizzly_bears()]);
    assert_eq!(pt(&e, soldier), (2, 1));
}

#[test]
fn dwarven_soldier_blocking_an_orc_gets_two_toughness() {
    let (e, soldier) = soldier_blocks(card_index("0d058991-3751-4afe-80c9-79f0c6e10282"));
    assert_eq!(pt(&e, soldier), (2, 3));
}

#[test]
fn dwarven_soldier_blocking_a_non_orc_gets_nothing() {
    let (e, soldier) = soldier_blocks(grizzly_bears());
    assert_eq!(pt(&e, soldier), (2, 1));
}

#[test]
fn dwarven_soldier_bonus_survives_a_lone_orcs_damage_but_not_for_the_next_turn() {
    // The Spy deals 1 damage to a 2/3 Soldier; without the bonus the
    // 2/1 would die.
    let (mut e, soldier) = soldier_attacks(&[card_index("0d058991-3751-4afe-80c9-79f0c6e10282")]);
    pass_until(&mut e, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert!(
        on_battlefield(&e, P0, card_index("88f9075c-ea96-4f8d-b0d1-e5e244e824c1")).is_some(),
        "1 damage against toughness 3"
    );
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert_eq!(pt(&e, soldier), (2, 1), "only until end of turn");
}
