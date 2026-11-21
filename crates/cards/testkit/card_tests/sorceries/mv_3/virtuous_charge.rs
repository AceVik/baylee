//! `cards/sorceries/mv_3/virtuous_charge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn virtuous_charge() -> CardIndex {
    card_index("5315ed83-cb93-4522-b439-3784e7f31e37")
}

/// Virtuous Charge (5315ed83-cb93-4522-b439-3784e7f31e37) — {2}{W} Sorcery:
/// "Creatures you control get +1/+1 until end of turn."
#[test]
fn virtuous_charge_pumps_only_controlled_creatures_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[plains(), plains(), plains(), llanowar_elves(), aurochs()],
        &[aurochs()],
        &[virtuous_charge()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_beast = lk::obj(&engine, P1, aurochs());

    tap_mana_except(&mut engine, P0, my_elf);
    cast_with_floating(&mut engine, P0, virtuous_charge());
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_elf), (2, 2));
    assert_eq!(pt(&engine, my_beast), (3, 4));
    assert_eq!(pt(&engine, their_beast), (2, 3));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_elf), (1, 1));
    assert_eq!(pt(&engine, my_beast), (2, 3));
}
