//! `cards/instants/mv_4/lure_of_prey.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "3c83ea11-7141-41a2-ba77-1ed2722dfe74"
fn lure_of_prey() -> CardIndex {
    card_index("3c83ea11-7141-41a2-ba77-1ed2722dfe74")
}

#[test]
fn lure_of_prey_puts_green_creature_card_from_hand_onto_battlefield() {
    let mut engine = lk::game(SEED, &[], &[], &[lure_of_prey(), index::GRIZZLY_BEARS], &[]);
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, lure_of_prey());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears was put from hand onto the battlefield"
    );
}
