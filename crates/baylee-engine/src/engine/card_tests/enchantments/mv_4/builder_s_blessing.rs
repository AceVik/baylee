//! Oracle checklist: own untapped creatures get +0/+2; a creature tapped by its real mana ability loses that bonus; opposing untapped creature is unchanged.
#[allow(clippy::wildcard_imports)]
use super::*;

fn builders_blessing() -> CardIndex {
    card_index("33a64de1-dc78-43d3-b89e-b4b13049e174")
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
