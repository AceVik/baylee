//! `cards/sorceries/mv_3/desperate_charge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn desperate_charge() -> CardIndex {
    card_index("26074e44-cf2b-459f-9a5d-7eed231ed2b4")
}

/// Desperate Charge (26074e44-cf2b-459f-9a5d-7eed231ed2b4) — {2}{B} Sorcery:
/// "Creatures you control get +2/+0 until end of turn."
#[test]
fn desperate_charge_pumps_only_controlled_creatures_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), llanowar_elves(), aurochs()],
        &[aurochs()],
        &[desperate_charge()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_beast = lk::obj(&engine, P1, aurochs());

    tap_mana_except(&mut engine, P0, my_elf);
    cast_with_floating(&mut engine, P0, desperate_charge());
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_elf), (3, 1));
    assert_eq!(pt(&engine, my_beast), (4, 3));
    assert_eq!(pt(&engine, their_beast), (2, 3));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_elf), (1, 1));
    assert_eq!(pt(&engine, my_beast), (2, 3));
}
