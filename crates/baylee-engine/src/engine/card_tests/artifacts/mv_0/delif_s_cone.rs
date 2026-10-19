//! `cards/artifacts/mv_0/delif_s_cone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn delif_s_cone() -> CardIndex {
    card_index("8af33199-d3ad-44fe-8ee0-abbe36a2278b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// p0's attackers go at p1.
#[track_caller]
fn declare(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("the attackers were on the offer");
}

/// p1 declares `blockers` and the game goes on.
#[track_caller]
fn block(engine: &mut Engine<RegistryLookup>, blockers: Vec<(ObjectId, ObjectId)>) {
    pass_until(engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    engine
        .apply(PlayerId::new(1), PlayerAction::DeclareBlockers { blockers })
        .expect("the blocks were on the offer");
}

/// The Cone sacrificed at `target`, resolved, in p0's first main phase.
fn cone_aimed_at(
    p0_board: &[CardIndex],
    p1_board: &[CardIndex],
    target: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, p0_board)
        .battlefield(1, p1_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, target).expect("the target is out");
    activate(&mut engine, p0, delif_s_cone(), 0);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, delif_s_cone()).is_some(),
        "{{T}}, sacrifice: the Cone went to the graveyard"
    );
    (engine, creature)
}

/// Delif's Cone (8af33199-d3ad-44fe-8ee0-abbe36a2278b) — "{T}, Sacrifice this
/// artifact: This turn, when target creature you control attacks and isn't
/// blocked, you may gain life equal to its power. If you do, it assigns no
/// combat damage this turn."
///
/// Bears (power 2) and Lions (power 2) attack, nothing blocks: the Cone's
/// creature gains 2 and deals nothing; the Lions, not the target, still hit.
#[test]
fn delif_s_cone_gains_life_and_stops_the_damage_of_its_unblocked_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) = cone_aimed_at(
        &[delif_s_cone(), grizzly_bears(), savannah_lions()],
        &[],
        grizzly_bears(),
    );
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("Lions");
    declare(&mut engine, &[bears, lions]);
    block(&mut engine, vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == p0),
        "\"you may\": p0 is asked, once, for the Cone's creature"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(
        life(&engine, p0),
        22,
        "gained life equal to the Bears' power"
    );
    assert_eq!(
        life(&engine, p1),
        18,
        "only the Lions dealt combat damage; the Bears assigned none"
    );
}

/// Declining the "may" gains nothing and leaves the damage in place.
#[test]
fn delif_s_cone_declined_changes_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) =
        cone_aimed_at(&[delif_s_cone(), grizzly_bears()], &[], grizzly_bears());
    declare(&mut engine, &[bears]);
    block(&mut engine, vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(life(&engine, p0), 20, "no life gained");
    assert_eq!(life(&engine, p1), 18, "the Bears dealt their 2");
}

/// "Isn't blocked": a blocked creature does not set the trigger off at all,
/// so there is no question, no life, and the Bears fight the Giant as usual.
#[test]
fn delif_s_cone_does_nothing_when_the_creature_is_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) = cone_aimed_at(
        &[delif_s_cone(), grizzly_bears()],
        &[hill_giant()],
        grizzly_bears(),
    );
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    declare(&mut engine, &[bears]);
    block(&mut engine, vec![(giant, bears)]);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(life(&engine, p0), 20, "no life was gained");
    assert_eq!(life(&engine, p1), 20, "the Bears were blocked");
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "the Giant's 3 damage killed the Bears: combat went on as normal"
    );
}
