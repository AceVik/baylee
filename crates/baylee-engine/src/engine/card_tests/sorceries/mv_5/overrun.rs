//! `cards/sorceries/mv_5/overrun.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn overrun() -> CardIndex {
    card_index("204f9afe-c20b-4933-b5cd-aa572784762a")
}

/// Overrun (`204f9afe-c20b-4933-b5cd-aa572784762a`) — {2}{G}{G}{G} Sorcery:
/// "Creatures you control get +3/+3 and gain trample until end of turn."
#[test]
fn overrun_pumps_creatures_you_control_and_grants_trample() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[overrun()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let opponent = PlayerId::new(1);
    cast_from_hand(&mut engine, P0, overrun());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, bears), (5, 5));
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );
    let giant = on_battlefield(&engine, opponent, index::HILL_GIANT).unwrap();
    assert_eq!(
        power_toughness(&engine, giant),
        (3, 3),
        "only creatures you control"
    );
    reach_their_main_phase(&mut engine, opponent);
    assert_eq!(power_toughness(&engine, bears), (2, 2));
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );
}
