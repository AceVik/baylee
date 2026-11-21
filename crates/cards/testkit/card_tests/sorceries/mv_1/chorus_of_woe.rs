//! `cards/sorceries/mv_1/chorus_of_woe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chorus_of_woe() -> CardIndex {
    card_index("e175a3c1-df6c-4aa7-a4c9-893b466ad9e0")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Chorus of Woe (e175a3c1-df6c-4aa7-a4c9-893b466ad9e0) — {B} Sorcery:
/// "Creatures you control get +1/+0 until end of turn."
#[test]
fn chorus_of_woe_pumps_only_controlled_creatures_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), llanowar_elves(), grizzly_bears()],
        &[grizzly_bears()],
        &[chorus_of_woe()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());

    tap_mana_except(&mut engine, P0, my_elf);
    cast_with_floating(&mut engine, P0, chorus_of_woe());
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_elf), (2, 1));
    assert_eq!(pt(&engine, my_bears), (3, 2));
    assert_eq!(
        pt(&engine, their_bears),
        (2, 2),
        "opponent's creature is not controlled by the caster"
    );
    assert!(in_graveyard(&engine, P0, chorus_of_woe()).is_some());

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_elf), (1, 1));
    assert_eq!(pt(&engine, my_bears), (2, 2));
}
