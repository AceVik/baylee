//! `cards/enchantments/auras/mv_1/laccolith_rig.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn laccolith_rig() -> CardIndex {
    card_index("0b583980-1508-4247-8a28-7127241d8ec0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Accepting the trigger has the enchanted creature deal damage equal to its
/// power to target creature and assign no combat damage this turn.
#[test]
fn laccolith_rig_accept_deals_power_to_target_and_assigns_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[savannah_lions(), grizzly_bears()],
        &[laccolith_rig()],
        &[],
    );
    let attacker = lk::obj(&e, p0, grizzly_bears());
    let blocker = lk::obj(&e, p1, savannah_lions());
    let bystander = lk::obj(&e, p1, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, laccolith_rig(), &[attacker], &[]);

    let offer = lk::attack(&mut e, &[attacker]);
    assert!(lk::may_block(&offer, blocker, attacker));
    lk::block(&mut e, &[(blocker, attacker)]);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[bystander], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);

    assert_eq!(lk::zone_of(&e, bystander), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, blocker), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, blocker), 0);
}

/// Declining the optional trigger lets the enchanted creature assign its
/// normal combat damage.
#[test]
fn laccolith_rig_decline_preserves_normal_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[savannah_lions(), grizzly_bears()],
        &[laccolith_rig()],
        &[],
    );
    let attacker = lk::obj(&e, p0, grizzly_bears());
    let blocker = lk::obj(&e, p1, savannah_lions());
    let bystander = lk::obj(&e, p1, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, laccolith_rig(), &[attacker], &[]);

    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(blocker, attacker)]);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[bystander], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::through_combat(&mut e);
    lk::settle(&mut e);

    assert_eq!(lk::zone_of(&e, blocker), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, bystander), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, bystander), 0);
}

/// Enchanting an opponent's creature still gives the Rig's controller the
/// "you may have it deal damage ..." choice when that creature becomes blocked.
#[test]
fn laccolith_rig_on_opponents_creature_lets_its_controller_redirect() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[savannah_lions()],
        &[grizzly_bears(), llanowar_elves()],
        &[laccolith_rig()],
        &[],
    );
    let my_blocker = lk::obj(&e, p0, savannah_lions());
    let their_attacker = lk::obj(&e, p1, grizzly_bears());
    let their_elves = lk::obj(&e, p1, llanowar_elves());

    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, laccolith_rig(), &[their_attacker], &[]);

    reach_their_main_phase(&mut e, p1);
    let offer = lk::attack(&mut e, &[their_attacker]);
    assert!(lk::may_block(&offer, my_blocker, their_attacker));
    lk::block(&mut e, &[(my_blocker, their_attacker)]);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(e.pending().asked(), Some(p0));
    lk::aim(&mut e, &[their_elves], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert_eq!(e.pending().asked(), Some(p0));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);

    assert_eq!(lk::zone_of(&e, their_elves), Zone::Graveyard);
    assert_eq!(lk::damage_on(&e, my_blocker), 0);
}
