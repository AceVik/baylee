//! `cards/enchantments/auras/mv_2/custody_battle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn custody_battle() -> CardIndex {
    card_index("9a9664d7-2299-43dc-89b6-d74fe69e0e4a")
}

/// Enchanted creature's controller keeps it on upkeep only by sacrificing a land;
/// declining hands control to target opponent, who then gets the upkeep trigger.
#[test]
fn custody_battle_transfers_control_unless_controller_sacrifices_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), mountain()],
        &[island()],
        &[custody_battle()],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let my_mtn = lk::obj(&e, p0, mountain());
    let opp_isl = lk::obj(&e, p1, island());

    lk::float(&mut e, p0, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut e, p0, custody_battle(), &[bear], &[]);

    // P0's next upkeep: aim trigger at P1, sacrifice Mountain -> keep Bear.
    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[], &[p1]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![my_mtn],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, my_mtn), Zone::Graveyard);
    assert_eq!(e.state().object(bear).unwrap().controller, p0);

    // P0's following upkeep (0 lands): trigger hands Bear to P1.
    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[], &[p1]);
    lk::settle(&mut e);
    assert_eq!(e.state().object(bear).unwrap().controller, p1);

    // P1's next upkeep: P1 now controls Bear, declines sacrificing Island -> back to P0.
    pass_until(&mut e, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[], &[p0]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, opp_isl), Zone::Battlefield);
    assert_eq!(e.state().object(bear).unwrap().controller, p0);
}
