//! `cards/artifacts/equipment/mv_2/manriki_gusari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn manriki_gusari() -> CardIndex {
    card_index("cc2eb3e4-084c-471b-9fe0-72aa9ff2019a")
}

/// Manriki-Gusari (cc2eb3e4-084c-471b-9fe0-72aa9ff2019a) — {2} Equipment:
/// "Equipped creature gets +1/+2 and has '{T}: Destroy target Equipment.'"
/// "Equip {1}"
///
/// Gives +1/+2 and grants `{T}: Destroy target Equipment` (targeting Equipment
/// only, excluding non-Equipment artifacts and creatures) to the equipped host.
#[test]
fn manriki_gusari_gives_stats_and_grants_tap_destroy_equipment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        manriki_gusari(),
        jasmine_boreal(),
        1,
        &[],
        &[llanowar_elves(), bonesplitter(), sol_ring()],
        &[],
    );
    let their_equip = on_battlefield(&r.engine, p1, bonesplitter()).unwrap();
    let their_ring = on_battlefield(&r.engine, p1, sol_ring()).unwrap();

    // Before equipping: no granted ability on host.
    assert!(
        !priority_offer(&r.engine)
            .abilities
            .iter()
            .any(|&(src, _)| src == r.host)
    );

    r.equip(1);
    assert_eq!(
        pt(&r.engine, r.host),
        (5, 7),
        "equipped creature gets +1/+2"
    );
    assert_eq!(pt(&r.engine, r.bystander), (2, 1), "bystander untouched");
    assert!(
        !priority_offer(&r.engine)
            .abilities
            .iter()
            .any(|&(src, _)| src == r.bystander),
        "bystander gets no granted ability"
    );

    let grant_idx = priority_offer(&r.engine)
        .abilities
        .iter()
        .find_map(|&(src, idx)| (src == r.host).then_some(idx))
        .expect("equipped creature gained {T}: Destroy target Equipment");

    r.engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: r.host,
                ability_index: grant_idx,
            },
        )
        .unwrap();
    let menu = aim_at(&mut r.engine, p0, their_equip);
    assert!(
        menu.contains(&their_equip) && menu.contains(&r.equipment),
        "Equipment permanents are legal targets: {menu:?}"
    );
    assert!(
        !menu.contains(&their_ring) && !menu.contains(&r.theirs),
        "non-Equipment artifact and creature are excluded: {menu:?}"
    );
    assert!(is_tapped(&r.engine, r.host));

    pass_until(&mut r.engine, stack_is_empty);
    assert_eq!(
        r.engine.state().object(their_equip).unwrap().zone,
        Zone::Graveyard,
        "target Equipment was destroyed"
    );
    assert_eq!(
        r.engine.state().object(their_ring).unwrap().zone,
        Zone::Battlefield
    );
}
