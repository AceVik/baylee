//! `cards/enchantments/mv_4/dreams_of_the_dead.rs`, played.
//!
//! Oracle:
//! - "{1}{U}: Return target white or black creature card from your graveyard to the battlefield. That creature gains 'Cumulative upkeep {2}.' If the creature would leave the battlefield, exile it instead of putting it anywhere else."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "3b6cf935-3b99-4371-a797-6f40b89adeeb"
fn dreams_of_the_dead() -> CardIndex {
    card_index("3b6cf935-3b99-4371-a797-6f40b89adeeb")
}

#[test]
fn dreams_of_the_dead_returns_target_white_or_black_creature_from_your_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[
            dreams_of_the_dead(),
            index::SAVANNAH_LIONS,
            index::GRIZZLY_BEARS,
        ],
        &[],
        &[],
        &[],
    );
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    bury(&mut engine, &[lions, bears]);

    let dead_lions = in_graveyard(&engine, P0, index::SAVANNAH_LIONS).expect("Lions in graveyard");
    let dead_bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in graveyard");

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    activate(&mut engine, P0, dreams_of_the_dead(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Dreams of the Dead");
    };
    assert!(
        options.contains(&dead_lions) && !options.contains(&dead_bears),
        "only white or black creature cards in your graveyard are legal targets"
    );
    lk::aim(&mut engine, &[dead_lions], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::SAVANNAH_LIONS).is_some(),
        "Savannah Lions returned from graveyard to the battlefield"
    );
}
