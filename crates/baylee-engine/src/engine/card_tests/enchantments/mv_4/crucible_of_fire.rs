//! Oracle checklist: +3/+3 applies to Dragon creatures you control; own Dragon receives it; own non-Dragon and opposing Dragon do not.
#[allow(clippy::wildcard_imports)]
use super::*;
fn crucible_of_fire() -> CardIndex {
    card_index("0c572396-4e53-4c86-9b04-f41341dcea05")
}
#[test]
fn crucible_buffs_only_dragons_its_controller_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[crucible_of_fire(), volcanic_dragon(), wild_elephant()])
        .battlefield(1, &[volcanic_dragon()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mine = on_battlefield(&e, p0, volcanic_dragon()).unwrap();
    let not_dragon = on_battlefield(&e, p0, wild_elephant()).unwrap();
    let other = on_battlefield(&e, p1, volcanic_dragon()).unwrap();
    assert_eq!(pt(&e, mine), (7, 7));
    assert_eq!(pt(&e, not_dragon), (3, 3));
    assert_eq!(pt(&e, other), (4, 4));
}
