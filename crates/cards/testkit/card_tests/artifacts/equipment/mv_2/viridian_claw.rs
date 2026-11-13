//! `cards/artifacts/equipment/mv_2/viridian_claw.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn viridian_claw() -> CardIndex {
    card_index("11ba27f4-decf-4447-9808-0d58844655cf")
}

/// Viridian Claw:
/// > Equipped creature gets +1/+0 and has first strike.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn viridian_claw_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        viridian_claw(),
        1,
        llanowar_elves(),
        (1, 0),
        KeywordSet::FIRST_STRIKE,
    );
}

/// The pumped 2/1 deals its first-strike damage before the 1/1 blocker can
/// answer, so the blocker dies and the equipped creature survives untouched.
#[test]
fn viridian_claw_plays_first_strike_in_combat() {
    let p1 = PlayerId::new(1);
    let mut r = rig(viridian_claw(), llanowar_elves(), 1);
    r.equip(1);
    let Pending::ChooseBlockers { .. } = r.attack_to_the_blocks() else {
        panic!("expected declare blockers")
    };
    r.engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(r.theirs, r.host)],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&r.engine, p1, llanowar_elves()).is_some());
    assert_eq!(r.engine.state().object(r.host).unwrap().damage, 0);
    assert_eq!(pt(&r.engine, r.host), (2, 1));
    assert_eq!(r.engine.state().players[1].life, 20);
}
