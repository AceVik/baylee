//! Oracle checklist: while Advocate is untapped, green creatures you control get +1/+1; white source is not green; non-green ally and opposing green creature are excluded; attacking taps source and removes bonus immediately.
#[allow(clippy::wildcard_imports)]
use super::*;
fn juniper_order_advocate() -> CardIndex {
    card_index("ded2fc21-1a69-48f8-865a-467e21b2758b")
}
#[test]
fn advocate_buffs_controlled_green_creatures_only_while_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[juniper_order_advocate(), wild_elephant(), gray_ogre()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, juniper_order_advocate()).unwrap();
    let green = on_battlefield(&e, p0, wild_elephant()).unwrap();
    let non_green = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let opposing = on_battlefield(&e, p1, wild_elephant()).unwrap();
    assert_eq!(pt(&e, green), (4, 4));
    assert_eq!(pt(&e, source), (1, 2));
    assert_eq!(pt(&e, non_green), (2, 2));
    assert_eq!(pt(&e, opposing), (3, 3));
    declare_band_attack(&mut e, p0, p1, &[source]);
    assert_eq!(pt(&e, green), (3, 3));
    assert_eq!(pt(&e, source), (1, 2));
}
