//! `cards/instants/mv_4/ramosian_rally.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "2731c237-eda1-4b74-a294-502d574f4435"
fn ramosian_rally() -> CardIndex {
    card_index("2731c237-eda1-4b74-a294-502d574f4435")
}

/// `Ramosian Rally` (`Coverage::Partial` — mana-cost mode `{3}{W}`):
/// "Creatures you control get +1/+1 until end of turn."
///
/// Verifies that `Ramosian Rally` gives all creatures its controller controls
/// +1/+1 until end of turn while leaving an opponent's creatures unchanged, and
/// wears off at cleanup.
#[test]
fn ramosian_rally_gives_creatures_you_control_plus_one_plus_one_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9506, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[ramosian_rally()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, ramosian_rally());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, my_bears), (3, 3));
    assert_eq!(pt(&engine, my_elf), (2, 2));
    assert_eq!(
        pt(&engine, their_bears),
        (2, 2),
        "opponent's creature is unaffected"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, my_bears), (2, 2));
    assert_eq!(pt(&engine, my_elf), (1, 1));
}
