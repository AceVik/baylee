//! `cards/enchantments/auras/mv_2/insubordination.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "0d3e843e-eeb4-421e-b55d-ceb21f51a080"
fn insubordination() -> CardIndex {
    card_index("0d3e843e-eeb4-421e-b55d-ceb21f51a080")
}

/// `Insubordination` (`Coverage::Implemented`):
/// "Enchant creature. At the beginning of the end step of enchanted creature's controller,
/// this Aura deals 2 damage to that player unless that creature attacked this turn."
///
/// Verifies that `Insubordination` deals 2 damage to the enchanted creature's
/// controller at their end step when the creature did not attack this turn, and
/// deals 0 damage on a turn the creature attacked.
#[test]
fn insubordination_damages_enchanted_creatures_controller_unless_it_attacked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9411, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[insubordination()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 bear");
    cast_from_hand(&mut engine, p0, insubordination());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // P1's first turn: P1 does not attack with Grizzly Bears -> takes 2 damage at P1's end step.
    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(
        engine.state().players[0].life,
        20,
        "did not trigger on P0's end step"
    );
    assert!(walk_to_own_main(&mut engine, p0));
    assert_eq!(
        engine.state().players[1].life,
        18,
        "Insubordination dealt 2 damage to P1 at P1's end step when bear did not attack"
    );

    // P1's second turn: P1 attacks with Grizzly Bears -> takes 0 damage from Insubordination at P1's end step.
    assert!(walk_to_own_main(&mut engine, p1));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    assert!(walk_to_own_main(&mut engine, p0));
    assert_eq!(
        engine.state().players[1].life,
        18,
        "Insubordination dealt 0 damage when the enchanted creature attacked this turn"
    );
}

#[test]
fn insubordination_damages_the_triggering_player_after_enchanted_creature_changes_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[insubordination(), index::RAY_OF_COMMAND],
        &[],
    );
    let bear = lk::obj(&engine, p1, index::GRIZZLY_BEARS);
    lk::float(&mut engine, p0, &[(ManaColor::Black, 2)]);
    lk::cast_at(&mut engine, p0, insubordination(), &[bear], &[]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::End && !stack_is_empty(e)
    });
    assert_eq!(lk::life(&engine, p0), 20);
    assert_eq!(lk::life(&engine, p1), 20);

    // The ability says "that player": changing the host's controller in response
    // cannot move the damage away from the player whose end step triggered it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, p0, index::RAY_OF_COMMAND, &[bear], &[]);
    assert_eq!(engine.state().object(bear).unwrap().controller, p0);
    assert_eq!(
        lk::life(&engine, p1),
        18,
        "the triggering player still takes 2 damage"
    );
    assert_eq!(
        lk::life(&engine, p0),
        20,
        "the new controller is not that player"
    );
}
