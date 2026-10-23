//! `cards/creatures/mv_6/mist_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "448f5c55-7334-4fa4-9d90-0e57bae19444"
fn mist_dragon() -> CardIndex {
    card_index("448f5c55-7334-4fa4-9d90-0e57bae19444")
}

#[test]
fn mist_dragon_gains_and_loses_flying_indefinitely_and_phases_out() {
    let mut engine = lk::game(SEED, &[mist_dragon()], &[], &[], &[]);
    let dragon = lk::obj(&engine, P0, mist_dragon());

    // Ability 0: gains flying indefinitely.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(dragon)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    // Ability 1: loses flying indefinitely.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine
            .state()
            .object(dragon)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    // Ability 2: {3}{U}{U}: phases out.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 2)],
    );
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 2,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(dragon)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
}

/// "This effect lasts indefinitely": flying granted by the first ability is
/// still there in the opponent's turn.
#[test]
fn mist_dragon_flying_outlasts_the_turn() {
    let mut engine = lk::game(SEED, &[mist_dragon()], &[], &[], &[]);
    let dragon = lk::obj(&engine, P0, mist_dragon());

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, P1);
    assert!(
        engine
            .state()
            .object(dragon)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "an indefinite effect does not end with the turn"
    );
}

/// "It phases in before you untap during your next untap step": still out
/// during the opponent's turn, back in at its controller's next turn.
#[test]
fn mist_dragon_phases_in_at_its_controllers_next_untap_step() {
    let mut engine = lk::game(SEED, &[mist_dragon()], &[], &[], &[]);
    let dragon = lk::obj(&engine, P0, mist_dragon());
    let phased_out = |e: &Engine<RegistryLookup>| {
        e.state()
            .object(dragon)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    };

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 2)],
    );
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 2,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(phased_out(&engine));

    reach_their_main_phase(&mut engine, P1);
    assert!(
        phased_out(&engine),
        "the opponent's untap step does not phase it in"
    );
    reach_their_main_phase(&mut engine, P0);
    assert!(
        !phased_out(&engine),
        "its controller's next untap phases it in"
    );
}
