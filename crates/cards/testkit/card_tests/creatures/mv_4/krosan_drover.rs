//! Krosan Drover — `{3}{G}` 2/2 Creature — Elf (SCG).
//! Oracle: "Creature spells you cast with mana value 6 or greater cost {2} less to cast."

use super::*;

fn krosan_drover() -> CardIndex {
    card_index("cd8ae400-4f40-42aa-a905-2787de443471")
}

#[test]
fn krosan_drover_reduces_cost_of_mv_6_or_greater_creature_spells_by_two() {
    let p0 = PlayerId::new(0);
    let drover = krosan_drover();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[drover, craw_wurm(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());

    // Cast Krosan Drover for {3}{G} (4 mana).
    tap_mana_where(&mut engine, p0, |id| forests[..4].contains(&id));
    cast_with_floating(&mut engine, p0, drover);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, drover).is_some());

    // Craw Wurm ({4}{G}{G}, MV 6) now costs {2}{G}{G} (4 mana instead of 6).
    tap_mana_where(&mut engine, p0, |id| forests[4..8].contains(&id));
    cast_with_floating(&mut engine, p0, craw_wurm());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, craw_wurm()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
