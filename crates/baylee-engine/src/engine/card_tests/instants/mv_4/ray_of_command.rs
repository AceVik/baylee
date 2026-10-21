//! `cards/instants/mv_4/ray_of_command.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::{Phase, Step};

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
/// (Coverage::Partial — untaps, gains control, and grants haste until end of turn.)
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

    // Because it gained haste, P0 can attack with it immediately this turn.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(their_giant, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 17);

    // At end of turn, control of Hill Giant returns to P1.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().object(their_giant).unwrap().controller,
        P1,
        "control of the creature returns to P1 at end of turn"
    );
}
