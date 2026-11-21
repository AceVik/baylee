//! `cards/sorceries/mv_2/steadfastness.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn steadfastness() -> CardIndex {
    card_index("8f8dfc24-f466-4607-8991-acbcbb415db3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Steadfastness (8f8dfc24-f466-4607-8991-acbcbb415db3) — {1}{W} Sorcery:
/// "Creatures you control get +0/+3 until end of turn."
#[test]
fn steadfastness_pumps_only_controlled_creatures_toughness_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[plains(), plains(), llanowar_elves(), grizzly_bears()],
        &[grizzly_bears()],
        &[steadfastness()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());

    tap_mana_except(&mut engine, P0, my_elf);
    cast_with_floating(&mut engine, P0, steadfastness());
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_elf), (1, 4));
    assert_eq!(pt(&engine, my_bears), (2, 5));
    assert_eq!(pt(&engine, their_bears), (2, 2));
    assert!(in_graveyard(&engine, P0, steadfastness()).is_some());

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_elf), (1, 1));
    assert_eq!(pt(&engine, my_bears), (2, 2));
}
