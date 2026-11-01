//! `cards/instants/mv_1/hearth_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn hearth_charm() -> CardIndex {
    card_index("4ce95005-82b3-4921-b1c4-60d775a012a2")
}

fn iron_heart_chimera() -> CardIndex {
    card_index("fe7c70dd-76ff-4958-9973-6452c7f071ed")
}

/// `Hearth Charm` ({R}, Instant, `Coverage::Implemented`) destroys target
/// artifact creature in mode 0 or makes target creature with power 2 or less
/// unblockable this turn in mode 2.
#[test]
fn hearth_charm_destroys_artifact_creature_or_grants_unblockable() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[iron_heart_chimera(), grizzly_bears(), sol_ring()])
        .hand(0, &[hearth_charm(), hearth_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let chimera = on_battlefield(&engine, p1, iron_heart_chimera()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    tap_all_mana(&mut engine, p0);

    // Mode 0: Destroy target artifact creature.
    cast_with_floating(&mut engine, p0, hearth_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot0 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot0)).unwrap();
    let menu = aim_at(&mut engine, p0, chimera);
    assert!(
        !menu.contains(&their_bear) && !menu.contains(&ring),
        "only an artifact creature: not a plain creature, not a plain artifact"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(chimera).unwrap().zone,
        Zone::Graveyard
    );

    // Mode 2: Target creature with power 2 or less can't be blocked this turn.
    cast_with_floating(&mut engine, p0, hearth_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot2 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(2)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot2)).unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bear).contains(KeywordSet::UNBLOCKABLE));
    assert!(
        !keywords(&engine, their_bear).contains(KeywordSet::UNBLOCKABLE),
        "only the target"
    );
}

/// Picks mode `n` of the Charm that was just cast.
#[track_caller]
fn choose_mode(engine: &mut Engine<RegistryLookup>, seat: PlayerId, n: usize) {
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("a mode question, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(m) if m == n))
        .expect("the mode is offered");
    engine.apply(seat, PlayerAction::ChooseMode(slot)).unwrap();
}

/// Declares `attackers` for P0 and returns with P0 holding priority in the declare-attackers step.
#[track_caller]
fn attack_with(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(PlayerId::new(1))))
                    .collect(),
            },
        )
        .unwrap();
}

/// Mode 1, "Attacking creatures get +1/+0 until end of turn": the attacker is 3/2, the creature
/// that stayed home on either side is not, and the bonus is gone on the next turn.
#[test]
fn hearth_charm_pumps_attacking_creatures_only_and_only_for_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), grizzly_bears(), gray_ogre()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[hearth_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let at_home = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    attack_with(&mut engine, &[attacker]);
    cast_from_hand(&mut engine, p0, hearth_charm());
    choose_mode(&mut engine, p0, 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, attacker), (3, 2), "the attacker gets +1/+0");
    assert_eq!(
        pt(&engine, at_home),
        (2, 2),
        "a creature that did not attack does not"
    );
    assert_eq!(pt(&engine, their_bear), (2, 2), "nor does the opponent's");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, attacker), (2, 2), "until end of turn");
}

/// Mode 2: the target "can't be blocked this turn": an opposing creature that could block anything
/// else is refused against it, a small attacker beside it can still be blocked, and a creature with
/// power 3 or more is not a legal target.
#[test]
fn hearth_charm_makes_a_small_creature_unblockable_and_refuses_a_big_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), grizzly_bears(), gray_ogre(), HILL_GIANT])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[hearth_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let charmed = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let plain = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, hearth_charm());
    choose_mode(&mut engine, p0, 2);
    let menu = aim_at(&mut engine, p0, charmed);
    assert!(menu.contains(&plain), "power 2 is small enough");
    assert!(!menu.contains(&giant), "a 3/3 is not a legal target");
    pass_until(&mut engine, stack_is_empty);

    attack_with(&mut engine, &[charmed, plain]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, charmed)],
                },
            )
            .is_err(),
        "the target can't be blocked"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, plain)],
            },
        )
        .expect("the creature that was not targeted can be blocked");
}
