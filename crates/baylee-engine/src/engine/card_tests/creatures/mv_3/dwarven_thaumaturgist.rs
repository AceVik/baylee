//! `cards/creatures/mv_3/dwarven_thaumaturgist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dwarven_thaumaturgist() -> CardIndex {
    card_index("019d0013-52fd-4261-9cc8-7efcee5a4616")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Dwarven Thaumaturgist (019d0013-52fd-4261-9cc8-7efcee5a4616) — "{T}:
/// Switch target creature's power and toughness until end of turn." Aimed at
/// the opponent's 2/1 it makes a 1/2, taps the Thaumaturgist, can be asked
/// only once per untap, and the switch ends with the turn.
#[test]
fn dwarven_thaumaturgist_switches_an_opponents_creature_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[dwarven_thaumaturgist()],
        &[savannah_lions()],
        &[],
        &[],
    );
    let shaman = lk::obj(&e, P0, dwarven_thaumaturgist());
    let lions = lk::obj(&e, P1, savannah_lions());
    assert_eq!(pt(&e, lions), (2, 1));

    activate(&mut e, P0, dwarven_thaumaturgist(), 0);
    let menu = aim_at(&mut e, P0, lions);
    assert!(
        menu.contains(&shaman) && menu.contains(&lions),
        "any creature is a target, yours or theirs: {menu:?}"
    );
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, lions), (1, 2), "power and toughness switched");
    assert_eq!(
        pt(&e, shaman),
        (1, 2),
        "the Thaumaturgist itself is untouched"
    );
    assert!(lk::tapped(&e, shaman), "{{T}} is the cost");
    assert!(
        !lk::offered(&e, dwarven_thaumaturgist(), P0, 0),
        "a tapped Thaumaturgist cannot do it again"
    );

    reach_their_main_phase(&mut e, P1);
    assert_eq!(pt(&e, lions), (2, 1), "until end of turn only");
}

/// Dwarven Thaumaturgist — "target creature" includes itself: its own 1/2
/// becomes 2/1.
#[test]
fn dwarven_thaumaturgist_may_switch_itself() {
    let mut e = lk::game(SEED, &[dwarven_thaumaturgist()], &[], &[], &[]);
    let shaman = lk::obj(&e, P0, dwarven_thaumaturgist());
    assert_eq!(pt(&e, shaman), (1, 2));

    activate(&mut e, P0, dwarven_thaumaturgist(), 0);
    aim_at(&mut e, P0, shaman);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, shaman), (2, 1));
}
