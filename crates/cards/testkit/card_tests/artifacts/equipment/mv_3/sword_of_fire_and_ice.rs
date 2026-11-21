//! `cards/artifacts/equipment/mv_3/sword_of_fire_and_ice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sword_of_fire_and_ice() -> CardIndex {
    card_index("2ccdc60a-49a9-44b9-a7af-0ebf18b26785")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

/// Sword of Fire and Ice (2ccdc60a-49a9-44b9-a7af-0ebf18b26785) — {3} Equipment:
/// "Equipped creature gets +2/+2 and has protection from red and from blue."
/// "Whenever equipped creature deals combat damage to a player, this Equipment
///  deals 2 damage to any target and you draw a card."
/// "Equip {2}"
///
/// Verifies Equip {2}, +2/+2, protection from red and from blue (cannot be
/// blocked by red or blue creatures, cannot be targeted by red Bolt or blue
/// Unsummon), and the combat-damage trigger dealing 2 damage to any target and
/// drawing a card.
#[test]
fn sword_of_fire_and_ice_stats_protection_from_red_and_blue_and_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        sword_of_fire_and_ice(),
        jasmine_boreal(),
        2,
        &[],
        &[goblin_piker(), air_elemental(), mountain(), island()],
        &[lightning_bolt(), unsummon()],
    );
    let red_blocker = on_battlefield(&r.engine, p1, goblin_piker()).unwrap();
    let blue_blocker = on_battlefield(&r.engine, p1, air_elemental()).unwrap();

    r.equip(2);
    assert_eq!(
        pt(&r.engine, r.host),
        (6, 7),
        "equipped creature gets +2/+2"
    );
    assert_eq!(pt(&r.engine, r.bystander), (2, 1));

    // Attack with 6/7 host: neither red Goblin Piker nor blue Air Elemental may block it!
    let Pending::ChooseBlockers { blockers, .. } = r.attack_to_the_blocks() else {
        unreachable!("attack_to_the_blocks returns ChooseBlockers")
    };
    assert!(
        blockers
            .iter()
            .all(|b| b.blocker != red_blocker && b.blocker != blue_blocker),
        "protection from red and blue prevents red & blue blockers: {blockers:?}"
    );
    assert!(
        r.engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(red_blocker, r.host)],
                },
            )
            .is_err()
    );
    r.engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Unblocked 6 combat damage hits p1 (20 -> 14) and triggers Sword: 2 damage to red_blocker + draw 1!
    let hand_before = r.hand_size();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut r.engine, p0, red_blocker);
    pass_until(&mut r.engine, stack_is_empty);

    assert_eq!(life_of(&r.engine, p1), 14, "6 combat damage to p1");
    assert_eq!(
        r.engine.state().object(red_blocker).unwrap().zone,
        Zone::Graveyard,
        "Sword trigger dealt 2 damage killing 2/1 Goblin Piker"
    );
    assert_eq!(r.hand_size(), hand_before + 1, "Sword trigger drew 1 card");

    // Opponent's red Lightning Bolt and blue Unsummon both cannot target the equipped host.
    reach_their_main_phase(&mut r.engine, p1);
    let p1_mtn = on_battlefield(&r.engine, p1, mountain()).unwrap();
    tap_mana_where(&mut r.engine, p1, |id| id == p1_mtn);
    cast_with_floating(&mut r.engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = r.engine.pending().clone() else {
        panic!("expected ChooseTargets for Bolt")
    };
    assert!(
        !options.contains(&r.host),
        "protection from red prevents Bolt from targeting host"
    );
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

    let p1_isl = on_battlefield(&r.engine, p1, island()).unwrap();
    tap_mana_where(&mut r.engine, p1, |id| id == p1_isl);
    cast_with_floating(&mut r.engine, p1, unsummon());
    let Pending::ChooseTargets { options, .. } = r.engine.pending().clone() else {
        panic!("expected ChooseTargets for Unsummon")
    };
    assert!(
        !options.contains(&r.host),
        "protection from blue prevents Unsummon from targeting host"
    );
}
