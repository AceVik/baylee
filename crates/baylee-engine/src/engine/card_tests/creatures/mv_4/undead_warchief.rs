//! Undead Warchief — `{2}{B}{B}` 1/1 Creature — Zombie (SCG).
//! Oracle: "Zombie spells you cast cost {1} less to cast. Zombie creatures you control get +2/+1."

use super::*;

fn undead_warchief() -> CardIndex {
    card_index("e6af56bf-bd78-4801-8f6e-033cdd68de3d")
}

#[test]
fn undead_warchief_reduces_zombie_spell_cost_and_pumps_your_zombies() {
    let p0 = PlayerId::new(0);
    let warchief = undead_warchief();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[warchief, warchief])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swamps = all_on_battlefield(&engine, p0, swamp());
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // First Undead Warchief costs {2}{B}{B} (4 mana).
    tap_mana_where(&mut engine, p0, |id| swamps[..4].contains(&id));
    cast_with_floating(&mut engine, p0, warchief);
    pass_until(&mut engine, stack_is_empty);

    let first = on_battlefield(&engine, p0, warchief).unwrap();
    // 1/1 base + 2/1 from its own static ability = 3/2; non-Zombie stays 2/2.
    assert_eq!(pt(&engine, first), (3, 2));
    assert_eq!(pt(&engine, bears), (2, 2));

    // Second Undead Warchief costs {1} less -> {1}{B}{B} (3 mana).
    tap_mana_where(&mut engine, p0, |id| swamps[4..7].contains(&id));
    cast_with_floating(&mut engine, p0, warchief);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let warchiefs = all_on_battlefield(&engine, p0, warchief);
    assert_eq!(warchiefs.len(), 2);
    for id in warchiefs {
        assert_eq!(pt(&engine, id), (5, 3));
    }
    assert_eq!(pt(&engine, bears), (2, 2));
}
