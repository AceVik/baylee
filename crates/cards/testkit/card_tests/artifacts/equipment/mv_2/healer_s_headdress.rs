//! `cards/artifacts/equipment/mv_2/healer_s_headdress.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn healer_s_headdress() -> CardIndex {
    card_index("bf689834-dcb7-4370-a986-bf542f88aba5")
}

/// Finds the granted `{T}: Prevent the next 1 damage` ability index on `host`.
fn granted_ability_on(engine: &Engine<RegistryLookup>, host: ObjectId) -> Option<u32> {
    priority_offer(engine)
        .abilities
        .iter()
        .find_map(|&(src, idx)| (src == host).then_some(idx))
}

fn prevent_with_grant(
    engine: &mut Engine<RegistryLookup>,
    host: ObjectId,
    objects: Vec<ObjectId>,
    players: Vec<PlayerId>,
) {
    let p0 = PlayerId::new(0);
    let grant_idx = granted_ability_on(engine, host).expect("granted tap prevention");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: host,
                ability_index: grant_idx,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Healer's Headdress (bf689834-dcb7-4370-a986-bf542f88aba5) — {2} Equipment:
/// "Equipped creature gets +0/+2 and has '{T}: Prevent the next 1 damage that
///  would be dealt to any target this turn.'"
/// "{W}{W}: Attach this Equipment to target creature you control."
/// "Equip {1}"
///
/// Verifies +0/+2, the granted `{T}` prevention ability preventing 1 damage to
/// both a player and a creature, and no grant on unequipped creatures.
#[test]
fn healer_s_headdress_grants_toughness_and_damage_prevention() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                healer_s_headdress(),
                jasmine_boreal(),
                goblin_piker(),
                forest(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();
    let piker = on_battlefield(&engine, p0, goblin_piker()).unwrap();
    assert_eq!(pt(&engine, jasmine), (4, 5));
    assert_eq!(granted_ability_on(&engine, jasmine), None);

    // Equip {1} to Jasmine Boreal -> becomes 4/7 and gains granted {T} prevention.
    let my_forest = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_forest);
    activate(&mut engine, p0, healer_s_headdress(), 0);
    aim_at(&mut engine, p0, jasmine);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, jasmine), (4, 7), "equipped creature gets +0/+2");
    assert_eq!(pt(&engine, piker), (2, 1), "unequipped bystander untouched");
    assert_eq!(granted_ability_on(&engine, piker), None);

    // 1. Tap Jasmine to prevent 1 damage to p0, then Bolt p0 (3 - 1 = 2 -> 18 life).
    prevent_with_grant(&mut engine, jasmine, vec![], vec![p0]);
    assert!(is_tapped(&engine, jasmine));

    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| id == mountains[0]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 18, "prevented 1 of Bolt's 3 damage");

    // 2. Next turn: tap Jasmine to prevent 1 damage to Jasmine, then Bolt Jasmine (3 - 1 = 2).
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);

    prevent_with_grant(&mut engine, jasmine, vec![jasmine], vec![]);
    tap_mana_where(&mut engine, p0, |id| id == mountains[1]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, jasmine);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(jasmine).unwrap().damage,
        2,
        "prevented 1 of Bolt's 3 damage to creature"
    );
}

/// "{W}{W}: Attach this Equipment to target creature you control" works at
/// instant speed on the opponent's turn and moves both +0/+2 and the granted
/// ability to the new host.
#[test]
fn healer_s_headdress_ww_attaches_at_instant_speed_and_moves_grant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                healer_s_headdress(),
                jasmine_boreal(),
                goblin_piker(),
                forest(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let headdress = on_battlefield(&engine, p0, healer_s_headdress()).unwrap();
    let jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();
    let piker = on_battlefield(&engine, p0, goblin_piker()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    let my_forest = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_forest);
    activate(&mut engine, p0, healer_s_headdress(), 0);
    aim_at(&mut engine, p0, jasmine);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);

    assert!(
        !ability_offered(&engine, headdress, 0),
        "Equip {{1}} is sorcery-speed only"
    );
    assert!(
        ability_offered(&engine, headdress, 1),
        "{{W}}{{W}} attach activates at instant speed on opponent's turn"
    );

    activate(&mut engine, p0, healer_s_headdress(), 1);
    let menu = aim_at(&mut engine, p0, piker);
    assert!(menu.contains(&piker) && menu.contains(&jasmine));
    assert!(!menu.contains(&their_elf));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, piker), (2, 3), "Piker gained +0/+2");
    assert_eq!(pt(&engine, jasmine), (4, 5), "Jasmine lost +0/+2");
    // Pass p1's active-player priority so p0 holds priority to inspect granted abilities.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(granted_ability_on(&engine, piker).is_some());
    assert_eq!(granted_ability_on(&engine, jasmine), None);
}
