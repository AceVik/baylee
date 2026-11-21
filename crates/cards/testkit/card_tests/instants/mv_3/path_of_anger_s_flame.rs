//! `cards/instants/mv_3/path_of_anger_s_flame.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9e89db98-fc9f-4574-b5bc-ee421fa29428"
fn path_of_anger_s_flame() -> CardIndex {
    card_index("9e89db98-fc9f-4574-b5bc-ee421fa29428")
}

/// Path of Anger's Flame ({2}{R}): creatures YOU control get +2/+0 until end
/// of turn (leaving opponent's creatures unpumped) and wears off at cleanup.
#[test]
fn path_of_angers_flame_pumps_only_your_creatures_by_two_zero_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[index::GRIZZLY_BEARS],
        &[path_of_anger_s_flame()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let my_elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut engine, P0, path_of_anger_s_flame());
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_bears), (4, 2));
    assert_eq!(pt(&engine, my_elf), (3, 1));
    assert_eq!(
        pt(&engine, their_bears),
        (2, 2),
        "opponent's creature unpumped"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_bears), (2, 2));
    assert_eq!(pt(&engine, my_elf), (1, 1));
}
