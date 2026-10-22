//! `cards/enchantments/mv_4/null_chamber.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "cb75aca6-dfc7-460d-b90c-80b7988f39a8"
fn null_chamber() -> CardIndex {
    card_index("cb75aca6-dfc7-460d-b90c-80b7988f39a8")
}

#[test]
fn null_chamber_enters_and_chooses_a_card_name() {
    let mut engine = lk::game(SEED, &[], &[], &[null_chamber()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, null_chamber());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: index::GRIZZLY_BEARS,
                face: 0,
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    let chamber = lk::obj(&engine, P0, null_chamber());
    assert!(
        engine
            .state()
            .object(chamber)
            .unwrap()
            .chosen_name
            .is_some()
    );
}
