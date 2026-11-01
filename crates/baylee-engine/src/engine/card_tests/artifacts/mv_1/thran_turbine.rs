#[allow(clippy::wildcard_imports)]
use super::*;

fn thran_turbine() -> CardIndex {
    card_index("6c31f06d-bf02-48a7-9c17-be2a226b8eae")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn thran_turbine_mana_pays_abilities_but_never_spells() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[thran_turbine(), index::MILLSTONE])
        .hand(0, &[index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert_eq!(lk::pool_total(&e, p0), 2);
    lk::float(&mut e, p0, &[(ManaColor::White, 1)]);
    let spell = in_hand(&e, p0, index::DISENCHANT).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.castable.contains(&spell) && !legal.payable.contains(&spell));
    let before = e
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .len();
    activate(&mut e, p0, index::MILLSTONE, 0);
    match e.pending() {
        Pending::ChooseTargets { .. } => lk::aim(&mut e, &[], &[p0]),
        Pending::ChoosePlayer { .. } => {
            e.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
        }
        other => panic!("target player: {other:?}"),
    }
    lk::settle(&mut e);
    assert_eq!(
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
            .len(),
        before - 2
    );
    assert_eq!(lk::pool_total(&e, p0), 1);
}

#[test]
fn thran_turbine_mana_can_be_declined_and_never_survives_upkeep() {
    let p0 = PlayerId::new(0);
    for yes in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[thran_turbine()])
            .start();
        keep_mulligans(&mut e);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        lk::yes_no(&mut e, yes);
        lk::settle(&mut e);
        assert_eq!(lk::pool_total(&e, p0), if yes { 2 } else { 0 });
        reach_main_phase(&mut e, p0);
        assert_eq!(lk::pool_total(&e, p0), 0);
    }
}
