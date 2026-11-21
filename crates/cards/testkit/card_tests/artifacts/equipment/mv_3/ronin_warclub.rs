//! `cards/artifacts/equipment/mv_3/ronin_warclub.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ronin_warclub() -> CardIndex {
    card_index("11b969f2-ff68-4c9e-b3f3-839736084303")
}

/// Ronin Warclub (11b969f2-ff68-4c9e-b3f3-839736084303) — {3} Equipment:
/// "Equipped creature gets +2/+1."
/// "Whenever a creature you control enters, attach this Equipment to that
///  creature."
/// "Equip {5}"
///
/// Auto-attaches when your creature enters (giving +2/+1), ignores opponent's
/// creatures entering, and Equip {5} attaches manually to another creature you
/// control.
#[test]
fn ronin_warclub_auto_attaches_to_own_etb_ignores_opponents_and_equips_for_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ronin_warclub(),
                jasmine_boreal(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let club = on_battlefield(&engine, p0, ronin_warclub()).unwrap();
    let jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();
    assert_eq!(engine.state().object(club).unwrap().attached_to, None);

    // 1. Cast your own Llanowar Elves -> Ronin Warclub triggers and auto-attaches (+2/+1 -> 3/2).
    let my_forests = forests(&engine, p0, 6);
    tap_mana_where(&mut engine, p0, |id| id == my_forests[0]);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert_eq!(
        engine.state().object(club).unwrap().attached_to,
        Some(my_elf),
        "auto-attached to own entering creature"
    );
    assert_eq!(pt(&engine, my_elf), (3, 2), "equipped creature gets +2/+1");
    assert_eq!(pt(&engine, jasmine), (4, 5));

    // 2. Equip {5} using the remaining 5 Forests to move Ronin Warclub to Jasmine Boreal.
    tap_mana_where(&mut engine, p0, |id| my_forests[1..6].contains(&id));
    activate(&mut engine, p0, ronin_warclub(), 0);
    aim_at(&mut engine, p0, jasmine);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().object(club).unwrap().attached_to,
        Some(jasmine)
    );
    assert_eq!(pt(&engine, jasmine), (6, 6), "Jasmine gets +2/+1");
    assert_eq!(pt(&engine, my_elf), (1, 1), "Elf lost +2/+1");

    // 3. Opponent casts Llanowar Elves -> Ronin Warclub does NOT move to opponent's creature.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(club).unwrap().attached_to,
        Some(jasmine),
        "opponent's creature ETB does not trigger Ronin Warclub"
    );
}
