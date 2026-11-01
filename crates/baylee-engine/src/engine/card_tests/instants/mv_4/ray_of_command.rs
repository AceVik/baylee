//! `cards/instants/mv_4/ray_of_command.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ray_of_command() -> CardIndex {
    card_index("adf41d29-403d-4936-9689-3148d103e700")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Ray of Command (adf41d29-403d-4936-9689-3148d103e700) — {3}{U} Instant:
/// "Untap target creature an opponent controls and gain control of it until end of turn.
/// That creature gains haste until end of turn. When you lose control of the creature, tap it."
#[test]
fn ray_of_command_untaps_and_steals_opponents_creature_with_haste_until_end_of_turn() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), grizzly_bears()],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[ray_of_command()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let their_giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .set_tapped(their_giant, true);
    engine.refresh_offer();

    cast_from_hand(&mut engine, P0, ray_of_command());
    let options = aim_at(&mut engine, P0, their_giant);
    assert!(
        !options.contains(&my_bears),
        "P0's own creature is excluded by 'target creature an opponent controls'"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, their_giant),
        "Ray of Command untaps the targeted creature"
    );
    assert_eq!(
        engine.state().object(their_giant).unwrap().controller,
        P0,
        "P0 gains control of the targeted creature"
    );
    assert!(
        keywords(&engine, their_giant).contains(KeywordSet::HASTE),
        "the creature gains haste until end of turn"
    );

    // It is offered as an attacker immediately, despite changing controllers this turn.
    let attackers = lk::attackers_offered(&mut engine);
    assert!(
        attackers.contains(&their_giant),
        "haste permits an immediate attack"
    );
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // Leave it untapped so cleanup's delayed tap cannot be masked by combat.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Cleanup && !stack_is_empty(e)
    });
    assert_eq!(engine.state().object(their_giant).unwrap().controller, P1);
    assert!(!keywords(&engine, their_giant).contains(KeywordSet::HASTE));
    assert!(
        !is_tapped(&engine, their_giant),
        "the tap trigger is still on the stack"
    );
    lk::settle(&mut engine);
    assert!(
        is_tapped(&engine, their_giant),
        "losing control at cleanup taps the creature"
    );
}

fn borrowed_giant(hand: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = lk::game(SEED, &[], &[hill_giant()], hand, &[ray_of_command()]);
    let giant = lk::obj(&engine, P1, hill_giant());
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, ray_of_command(), &[giant], &[]);
    assert_eq!(engine.state().object(giant).unwrap().controller, P0);
    assert!(!is_tapped(&engine, giant));
    (engine, giant)
}

#[test]
fn ray_of_command_taps_the_creature_when_control_is_lost_before_cleanup() {
    let (mut engine, giant) = borrowed_giant(&[ray_of_command()]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P1, ray_of_command());
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, |e| {
        e.state().object(giant).unwrap().controller == P1 && !stack_is_empty(e)
    });
    assert_ne!(engine.state().turn.step, Step::Cleanup);
    assert!(
        !is_tapped(&engine, giant),
        "the tap waits for its delayed trigger"
    );
    lk::settle(&mut engine);
    assert!(
        is_tapped(&engine, giant),
        "the original Ray trigger taps the creature"
    );
}

#[test]
fn ray_of_command_does_not_tap_a_new_incarnation_of_the_creature() {
    let (mut engine, giant) = borrowed_giant(&[ray_of_command(), index::FLICKER]);
    let old_version = engine.state().object(giant).unwrap().version;
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P0, index::FLICKER, &[giant], &[]);
    let returned = lk::obj(&engine, P1, hill_giant());
    assert_ne!(
        engine.state().object(returned).unwrap().version,
        old_version
    );
    assert!(
        !is_tapped(&engine, returned),
        "the delayed tap cannot follow a new incarnation"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert!(!is_tapped(&engine, returned));
}
