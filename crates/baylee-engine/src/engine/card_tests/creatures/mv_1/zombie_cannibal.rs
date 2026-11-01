//! Oracle scenarios for Zombie Cannibal.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn zombie_cannibal_exiles_target_card_from_damaged_players_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("baab3245-6616-4a4a-a168-3215a7878073");
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[card])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();

    let cannibal = on_battlefield(&e, p0, card).unwrap();
    let gy_bear = e.state().zones.list(ZoneLocation::Graveyard(p1))[0];

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(cannibal, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![gy_bear],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty());
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(p1)).len(), 1);
}
