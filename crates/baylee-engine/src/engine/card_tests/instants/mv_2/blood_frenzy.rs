//! `cards/instants/mv_2/blood_frenzy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn blood_frenzy() -> CardIndex {
    card_index("8d3d5e87-8b12-4549-a46e-70a103dbcf44")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 has a Llanowar Elves (the attacker), a Grizzly Bears that stays home,
/// two Mountains and Blood Frenzy in hand; p1 has a Hill Giant and a Grizzly
/// Bears. p0's first main phase.
fn a_frenzy_before_combat() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[quiet_creature(), grizzly_bears(), mountain(), mountain()],
        )
        .battlefield(1, &[hill_giant(), grizzly_bears()])
        .hand(0, &[blood_frenzy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let attacker = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    (engine, attacker)
}

/// p0 attacks p1 with `attacker`; stops at p1's declare-blockers question.
#[track_caller]
fn declare_attack(engine: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
}

/// Blood Frenzy (8d3d5e87-8b12-4549-a46e-70a103dbcf44) — {1}{R} Instant:
/// "Cast this spell only before the combat damage step. Target attacking or blocking creature gets +4/+0 until end of turn. Destroy that creature at the beginning of the next end step."
#[test]
fn blood_frenzy_pumps_an_attacker_and_destroys_it_at_the_end_step_not_before() {
    let (mut engine, attacker) = a_frenzy_before_combat();
    let home_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    declare_attack(&mut engine, attacker);
    // p1 blocks nothing; the Frenzy is cast in the declare blockers step.
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    cast_from_hand(&mut engine, P0, blood_frenzy());
    let menu = aim_at(&mut engine, P0, attacker);
    assert!(menu.contains(&attacker));
    assert!(
        !menu.contains(&home_bears) && !menu.contains(&giant) && !menu.contains(&their_bears),
        "only an attacking or blocking creature is a target: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, attacker), (5, 1), "+4/+0");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 15, "5 damage got through");
    assert!(
        on_battlefield(&engine, P0, quiet_creature()).is_some(),
        "alive until the beginning of the end step"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(in_graveyard(&engine, P0, quiet_creature()).is_some());
}

#[test]
fn blood_frenzy_may_target_a_blocking_creature_of_the_opponent() {
    let (mut engine, attacker) = a_frenzy_before_combat();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    declare_attack(&mut engine, attacker);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_bears, attacker)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    cast_from_hand(&mut engine, P0, blood_frenzy());
    let menu = aim_at(&mut engine, P0, their_bears);
    assert!(menu.contains(&attacker) && menu.contains(&their_bears));
    assert!(
        !menu.contains(&giant),
        "a creature that is neither attacking nor blocking is not: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, their_bears), (6, 2));

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "the blocker survives until the end step"
    );
    reach_their_main_phase(&mut engine, P1);
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
}

#[test]
fn blood_frenzy_cannot_be_cast_once_combat_damage_has_been_dealt() {
    let (mut engine, attacker) = a_frenzy_before_combat();
    declare_attack(&mut engine, attacker);
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatDamage
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(
        on_battlefield(&engine, P0, quiet_creature()).is_some(),
        "the unblocked attacker survived the damage"
    );

    tap_all_mana(&mut engine, P0);
    let frenzy = in_hand(&engine, P0, blood_frenzy()).unwrap();
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: frenzy })
            .is_err(),
        "\"only before the combat damage step\""
    );
}

#[test]
fn blood_frenzy_has_no_target_before_combat() {
    let (mut engine, _) = a_frenzy_before_combat();

    tap_all_mana(&mut engine, P0);
    let frenzy = in_hand(&engine, P0, blood_frenzy()).unwrap();
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: frenzy })
            .is_err(),
        "no creature is attacking or blocking in the main phase"
    );
}
