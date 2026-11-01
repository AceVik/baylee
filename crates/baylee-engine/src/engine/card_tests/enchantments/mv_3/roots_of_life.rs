//! `cards/enchantments/mv_3/roots_of_life.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::subtypes;

fn roots_of_life() -> CardIndex {
    card_index("29ec9b49-fbe5-49ac-8ad6-7159cb88bc66")
}

/// Roots of Life (`{1}{G}{G}`, `Coverage::Implemented`): chooses Island or
/// Swamp as it enters, and gains 1 life whenever a land of the chosen type an
/// opponent controls becomes tapped.
#[test]
fn roots_of_life_gains_life_only_when_opponent_taps_chosen_land_type() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), island()])
        .battlefield(1, &[island(), swamp(), mountain()])
        .hand(0, &[roots_of_life()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_island = on_battlefield(&engine, p0, island()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id != my_island);
    cast_with_floating(&mut engine, p0, roots_of_life());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![subtypes::land::ISLAND, subtypes::land::SWAMP]);
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::land::ISLAND))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Tapping your own Island does not trigger Roots of Life.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: my_island })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);

    // Pass priority to p1: tapping p1's Swamp does not trigger, tapping p1's Island does.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let their_swamp = on_battlefield(&engine, p1, swamp()).unwrap();
    let their_island = on_battlefield(&engine, p1, island()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: their_swamp,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);

    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: their_island,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21);
}

/// The other answer: with Swamp chosen their Swamp pays and their Island does
/// not, and each trigger is resolved before the life total is read.
#[test]
fn roots_of_life_naming_swamp_gains_life_for_their_swamp_and_not_their_island() {
    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[], &[island(), swamp()], &[roots_of_life()], &[]);
    let their_island = lk::obj(&engine, P1, island());
    let their_swamp = lk::obj(&engine, P1, swamp());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, roots_of_life());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseSubtype(subtypes::land::SWAMP))
        .unwrap();
    lk::settle(&mut engine);

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::ActivateManaAbility {
                source: their_island,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(lk::life(&engine, P0), 20, "Island was not the chosen type");

    engine
        .apply(
            P1,
            PlayerAction::ActivateManaAbility {
                source: their_swamp,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(lk::life(&engine, P0), 21);
}
