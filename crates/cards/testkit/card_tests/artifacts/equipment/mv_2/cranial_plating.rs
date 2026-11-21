//! `cards/artifacts/equipment/mv_2/cranial_plating.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn cranial_plating() -> CardIndex {
    card_index("75564721-e4e9-463a-b49e-f0c7cd6f53a7")
}

/// Cranial Plating (75564721-e4e9-463a-b49e-f0c7cd6f53a7) — {2} Equipment:
/// "Equipped creature gets +1/+0 for each artifact you control."
/// "{B}{B}: Attach this Equipment to target creature you control."
/// "Equip {1}"
///
/// Equip {1} attaches to a creature you control and grants +1/+0 per artifact
/// YOU control (counting Plating itself, ignoring opponent's artifacts, and
/// updating dynamically when another artifact enters).
#[test]
fn cranial_plating_equips_for_one_and_scales_with_own_artifacts_only() {
    let p0 = PlayerId::new(0);
    let mut r = rig_vs(
        cranial_plating(),
        goblin_piker(),
        2,
        &[sol_ring()],
        &[llanowar_elves(), sol_ring(), sol_ring()],
        &[],
    );

    // Bystander is Steel Wall (an artifact creature), so p0 starts with 2 artifacts
    // (Cranial Plating + Steel Wall) while p1 has 2 Sol Rings.
    assert_eq!(pt(&r.engine, r.host), (2, 1), "unequipped host is 2/1");

    r.equip_keeping(1, 1);
    assert_eq!(
        pt(&r.engine, r.host),
        (4, 1),
        "+2/+0 for 2 own artifacts (Plating + Steel Wall), ignoring opponent's 2 Sol Rings"
    );
    assert_eq!(
        pt(&r.engine, r.bystander),
        (0, 4),
        "unequipped bystander unaffected"
    );

    // Cast Sol Ring with the spare {G}: own artifact count rises from 2 to 3 -> 5/1.
    cast_with_floating(&mut r.engine, p0, sol_ring());
    pass_until(&mut r.engine, stack_is_empty);
    assert_eq!(
        pt(&r.engine, r.host),
        (5, 1),
        "bonus updates dynamically to +3/+0 as third own artifact enters"
    );
}

/// "{B}{B}: Attach this Equipment to target creature you control" activates at
/// instant speed (on the opponent's turn, when Equip {1} is illegal) and moves
/// the +1/+0 bonus to the new host.
#[test]
fn cranial_plating_bb_attaches_at_instant_speed_on_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                cranial_plating(),
                goblin_piker(),
                jasmine_boreal(),
                forest(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let plating = on_battlefield(&engine, p0, cranial_plating()).unwrap();
    let piker = on_battlefield(&engine, p0, goblin_piker()).unwrap();
    let jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // Equip {1} onto Goblin Piker (1 own artifact -> +1/+0 => 3/1).
    let my_forest = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_forest);
    activate(&mut engine, p0, cranial_plating(), 0);
    aim_at(&mut engine, p0, piker);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, piker), (3, 1));
    assert_eq!(pt(&engine, jasmine), (4, 5));

    // Opponent's turn: p1 passes priority to p0. Float {B}{B} + {G}.
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);

    assert!(
        !ability_offered(&engine, plating, 0),
        "Equip {{1}} is sorcery-speed only and cannot activate on opponent's turn"
    );
    assert!(
        ability_offered(&engine, plating, 2),
        "{{B}}{{B}} attach is instant-speed and offered on opponent's turn"
    );

    activate(&mut engine, p0, cranial_plating(), 2);
    let menu = aim_at(&mut engine, p0, jasmine);
    assert!(menu.contains(&jasmine) && menu.contains(&piker));
    assert!(
        !menu.contains(&their_elf),
        "cannot attach to opponent's creature: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(plating).unwrap().attached_to,
        Some(jasmine)
    );
    assert_eq!(pt(&engine, jasmine), (5, 5), "Jasmine gained +1/+0");
    assert_eq!(pt(&engine, piker), (2, 1), "Piker lost +1/+0");
}
