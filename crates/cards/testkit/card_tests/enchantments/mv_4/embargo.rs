//! `cards/enchantments/mv_4/embargo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Embargo — {3}{U} Enchantment: "Nonland permanents don't untap during
/// their controllers' untap steps" and "At the beginning of your upkeep, you
/// lose 2 life."
///
/// Both printed sentences are read off one turn cycle, and each is held up by
/// a control the words themselves demand. The Islands are the word *nonland*:
/// they come back while the Elves and the Sol Ring beside them stay down, so a
/// permanent still tapped after an untap step is the sentence and not a turn
/// that never ran. The Elf across the table is the same sentence on the other
/// side of it, and p1's untouched life total tells "your upkeep" from "each
/// upkeep". Nothing here was tapped by the harness — every permanent went down
/// paying its own mana ability — so the untap step is the only thing that
/// could stand one back up.
#[test]
fn embargo_holds_every_nonland_permanent_down_and_drains_its_controller_each_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                quiet_creature(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[forest(), quiet_creature()])
        .hand(0, &[embargo()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3}{U} out of every source this seat has: four Islands, the Elves'
    // printed {T}: Add {G} and the Sol Ring's own tap, which `tap_all_mana`
    // presses because each one's whole price is its own {T} (#159).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "four Islands, one mana creature and one Sol Ring: seven mana"
    );
    cast_with_floating(&mut engine, p0, embargo());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, embargo()).is_some(),
        "the Embargo resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{3}}{{U}} came out of the pool it was cast from"
    );

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("my Elves are out");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    assert!(
        is_tapped(&engine, elf) && is_tapped(&engine, ring),
        "both nonland permanents went down paying for the enchantment"
    );

    // p1's own turn: their Forest and their Elves are tapped by their own mana
    // abilities, which is the only way a permanent may be found lying down
    // here without the harness having put it that way.
    reach_their_main_phase(&mut engine, p1);
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elves are out");
    tap_all_mana(&mut engine, p1);
    assert!(
        is_tapped(&engine, their_forest) && is_tapped(&engine, their_elf),
        "their Forest and their Elves paid for their own mana"
    );

    // Back to p0's next turn: the untap step is the first printed sentence and
    // the upkeep before the main phase is the second, so one arrival reads both.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"At the beginning of your upkeep, you lose 2 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and it is the controller's upkeep and not every seat's: p1 has already \
         taken an upkeep with the Embargo on the table"
    );

    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 4, "four Islands were dealt");
    assert!(
        islands.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: the lands come back, because \"nonland\" is read"
    );
    assert!(
        is_tapped(&engine, elf) && is_tapped(&engine, ring),
        "while the Elves and the Sol Ring, which are no lands, stayed down"
    );

    // And the same sentence on the other side of the table, which is where a
    // static quietly scoped to its own controller would show.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, their_forest),
        "p1's untap step ran as well: their Forest is standing again"
    );
    assert!(
        is_tapped(&engine, their_elf),
        "\"Nonland permanents don't untap during their controllers' untap \
         steps\": the Elf across the table is one of them"
    );
}
