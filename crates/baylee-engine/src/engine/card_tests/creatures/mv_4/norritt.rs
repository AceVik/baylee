//! `cards/creatures/mv_4/norritt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn norritt() -> CardIndex {
    card_index("4318c704-888f-4d85-b5f7-6d9abb14e60c")
}

fn zephyr_falcon() -> CardIndex {
    baylee_core::generated::index::ZEPHYR_FALCON
}

fn wall_of_stone() -> CardIndex {
    baylee_core::generated::index::WALL_OF_STONE
}

/// Norritt (4318c704-888f-4d85-b5f7-6d9abb14e60c) — {3}{B} 1/1 Creature — Imp:
/// - "{T}: Untap target blue creature."
/// - "{T}: Choose target non-Wall creature the active player has controlled continuously since
///   the beginning of the turn. That creature attacks this turn if able. Destroy it at the
///   beginning of the next end step if it didn't attack this turn. Activate only before
///   attackers are declared."
#[test]
fn norritt_untaps_only_blue_creatures_and_forces_non_wall_active_player_creature_to_attack_or_die()
{
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                norritt(),
                zephyr_falcon(),
                grizzly_bears(),
                wall_of_stone(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let blue_falcon = on_battlefield(&engine, P0, zephyr_falcon()).unwrap();
    let green_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let wall = on_battlefield(&engine, P0, wall_of_stone()).unwrap();
    let p1_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    // 1. Cast a second Grizzly Bears this turn so it was NOT controlled since the beginning of the turn.
    cast_from_hand(&mut engine, P0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    let all_p0_bears = all_on_battlefield(&engine, P0, grizzly_bears());
    let fresh_bears = *all_p0_bears.iter().find(|&&id| id != green_bears).unwrap();

    // Tap both blue Zephyr Falcon and green Grizzly Bears; activate ability 0 ("{T}: Untap target blue creature").
    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.set_tapped(blue_falcon, true);
        state.set_tapped(green_bears, true);
    }
    engine.refresh_offer();

    activate(&mut engine, P0, norritt(), 0);
    let untap_targets = aim_at(&mut engine, P0, blue_falcon);
    assert!(
        !untap_targets.contains(&green_bears),
        "non-blue creature cannot be targeted by '{{T}}: Untap target blue creature'"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, blue_falcon),
        "blue Zephyr Falcon is untapped by Norritt"
    );

    // 2. Untap Norritt to test ability 1: only non-Wall creatures controlled by the active player
    // since the beginning of the turn are legal targets; Wall, fresh creature, and P1's creature are excluded.
    let nor = on_battlefield(&engine, P0, norritt()).unwrap();
    engine.dev_state_mut(P0).unwrap().set_tapped(nor, false);
    engine.refresh_offer();

    activate(&mut engine, P0, norritt(), 1);
    let goad_targets = aim_at(&mut engine, P0, green_bears);
    assert!(
        !goad_targets.contains(&wall),
        "Wall is excluded by 'target non-Wall creature'"
    );
    assert!(
        !goad_targets.contains(&fresh_bears),
        "creature that entered this turn is excluded by 'controlled continuously since the beginning of the turn'"
    );
    assert!(
        !goad_targets.contains(&p1_bears),
        "non-active player's creature is excluded by 'the active player has controlled'"
    );
    pass_until(&mut engine, stack_is_empty);

    // Because green_bears is tapped, it cannot attack when DeclareAttackers arrives,
    // so at the beginning of the next end step it is destroyed because it didn't attack this turn.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        !ability_offered(&engine, nor, 1),
        "Norritt's second ability cannot be activated after attackers are declared"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(green_bears).unwrap().zone,
        Zone::Graveyard,
        "targeted creature that did not attack this turn is destroyed at the beginning of the end step"
    );
}

#[test]
fn norritt_goaded_creature_must_attack_and_is_not_destroyed_when_it_does() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[norritt(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    activate(&mut engine, P0, norritt(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    // "That creature attacks this turn if able": declaring no attackers is refused.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "the goaded creature has to attack"
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();

    // It attacked, so the next end step does not destroy it (checked a turn later).
    pass_until(&mut engine, |e| e.state().turn.number == 2);
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "'destroy it if it didn't attack': it did attack, so it stays"
    );
}

#[test]
fn norritt_on_the_opponents_turn_goads_the_active_players_creature_not_its_own() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[norritt(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    // P1's upkeep (turn 2): P1 passes, then P0 holds priority and may activate Norritt.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    activate(&mut engine, P0, norritt(), 1);
    let options = aim_at(&mut engine, P0, their_bears);
    assert!(
        !options.contains(&my_bears),
        "on P1's turn the active player is P1: Norritt's controller's own creature is not a legal target"
    );
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "the goaded creature (P1's) has to attack on P1's turn"
    );
}
