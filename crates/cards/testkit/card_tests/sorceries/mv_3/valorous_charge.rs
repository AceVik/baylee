//! `cards/sorceries/mv_3/valorous_charge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn valorous_charge() -> CardIndex {
    card_index("1f51fea2-ce19-44ef-a330-65cc0fafcd64")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Valorous Charge (1f51fea2-ce19-44ef-a330-65cc0fafcd64) — {1}{W}{W} Sorcery:
/// "White creatures get +2/+0 until end of turn." Pumps white creatures on
/// both sides of the battlefield while leaving nonwhite creatures unchanged.
#[test]
fn valorous_charge_pumps_white_creatures_on_both_sides_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[
            plains(),
            plains(),
            plains(),
            savannah_lions(),
            grizzly_bears(),
        ],
        &[serra_angel(), grizzly_bears()],
        &[valorous_charge()],
        &[],
    );
    let my_lions = lk::obj(&engine, P0, savannah_lions());
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let their_angel = lk::obj(&engine, P1, serra_angel());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());

    cast_from_hand(&mut engine, P0, valorous_charge());
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, my_lions), (4, 1), "your white creature +2/+0");
    assert_eq!(
        pt(&engine, their_angel),
        (6, 4),
        "opponent's white creature also gets +2/+0"
    );
    assert_eq!(pt(&engine, my_bears), (2, 2), "nonwhite creature unchanged");
    assert_eq!(pt(&engine, their_bears), (2, 2));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_lions), (2, 1));
    assert_eq!(pt(&engine, their_angel), (4, 4));
}
