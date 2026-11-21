//! `cards/artifacts/equipment/mv_5/pariah_s_shield.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn pariah_s_shield() -> CardIndex {
    card_index("f2103ab8-a183-4db8-98dd-4146217b5125")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Pariah's Shield (f2103ab8-a183-4db8-98dd-4146217b5125) — {5} Equipment:
/// "All damage that would be dealt to you is dealt to equipped creature
///  instead."
/// "Equip {3}"
///
/// Redirects both noncombat Lightning Bolt (3) and unblocked combat damage
/// from Grizzly Bears (2) away from you onto equipped 4/5 Jasmine Boreal
/// (killing Jasmine at 5 total damage while keeping you at 20 life); once
/// unequipped, a second Bolt damages you directly (20 -> 17).
#[test]
fn pariah_s_shield_redirects_all_damage_to_equipped_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        pariah_s_shield(),
        jasmine_boreal(),
        3,
        &[],
        &[grizzly_bears(), mountain(), mountain()],
        &[lightning_bolt(), lightning_bolt()],
    );
    r.equip(3);

    reach_their_main_phase(&mut r.engine, p1);
    let p1_mtns = lands_of(&r.engine, p1);

    // 1. Noncombat Bolt at p0 -> redirected to equipped 4/5 Jasmine Boreal!
    tap_mana_where(&mut r.engine, p1, |id| id == p1_mtns[0]);
    cast_with_floating(&mut r.engine, p1, lightning_bolt());
    r.engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, stack_is_empty);

    assert_eq!(life_of(&r.engine, p0), 20, "Bolt redirected off p0");
    assert_eq!(
        r.engine.state().object(r.host).unwrap().damage,
        3,
        "equipped creature absorbed 3 Bolt damage"
    );

    // 2. Unblocked 2/2 Grizzly Bears combat damage to p0 -> redirected to Jasmine (3 + 2 = 5 -> dies!).
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    r.engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(r.theirs, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    r.engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(
        life_of(&r.engine, p0),
        20,
        "combat damage redirected off p0"
    );
    assert_eq!(
        r.engine.state().object(r.host).unwrap().zone,
        Zone::Graveyard,
        "equipped 4/5 died from 3 + 2 redirected damage"
    );
    assert_eq!(r.attached_to(), None, "Shield is now unequipped");

    // 3. Second Bolt at p0 while Shield is unequipped hits p0 directly (20 -> 17).
    tap_mana_where(&mut r.engine, p1, |id| id == p1_mtns[1]);
    cast_with_floating(&mut r.engine, p1, lightning_bolt());
    r.engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, stack_is_empty);
    assert_eq!(
        life_of(&r.engine, p0),
        17,
        "unequipped Pariah's Shield no longer redirects damage"
    );
}
