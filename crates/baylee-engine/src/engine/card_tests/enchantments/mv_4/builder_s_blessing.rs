//! Oracle checklist: own untapped creatures get +0/+2; a creature tapped by its real mana ability loses that bonus; opposing untapped creature is unchanged.
#[allow(clippy::wildcard_imports)]
use super::*;

fn builders_blessing() -> CardIndex {
    card_index("33a64de1-dc78-43d3-b89e-b4b13049e174")
}

fn builder_s_blessing() -> CardIndex {
    builders_blessing()
}

#[test]
fn blessing_tracks_whether_each_creature_you_control_is_untapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[builders_blessing(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let own = all_on_battlefield(&e, p0, llanowar_elves());
    assert_eq!(own.len(), 2);
    let (tapped, untapped) = (own[0], own[1]);
    let opposing = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert_eq!(pt(&e, tapped), (1, 3));
    assert_eq!(pt(&e, untapped), (1, 3));
    assert_eq!(pt(&e, opposing), (1, 1));

    assert_eq!(tap_mana_where(&mut e, p0, |id| id == tapped), 1);
    assert!(is_tapped(&e, tapped));
    assert_eq!(
        pt(&e, tapped),
        (1, 1),
        "the bonus disappears as soon as it taps"
    );
    assert_eq!(
        pt(&e, untapped),
        (1, 3),
        "the other own creature remains untapped"
    );
    assert_eq!(pt(&e, opposing), (1, 1));
}

/// "Untapped creatures you control get +0/+2": an untapped creature of the
/// controller gets it, an opponent's does not, and the bonus is gone while
/// the creature is tapped and back when it untaps.
#[test]
fn builder_s_blessing_buffs_only_untapped_creatures_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[builder_s_blessing(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, mine), (2, 4), "untapped and mine: +0/+2");
    assert_eq!(pt(&engine, theirs), (2, 2), "an opponent's gets nothing");

    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(mine)
        .expect("seated")
        .status
        .insert(Status::TAPPED);
    resync(&mut engine);
    assert_eq!(pt(&engine, mine), (2, 2), "tapped: the bonus is gone");

    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(mine)
        .expect("seated")
        .status
        .remove(Status::TAPPED);
    resync(&mut engine);
    assert_eq!(
        pt(&engine, mine),
        (2, 4),
        "untapped again: the bonus is back"
    );
}
