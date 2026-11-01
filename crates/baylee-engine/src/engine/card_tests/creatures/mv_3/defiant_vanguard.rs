//! `cards/creatures/mv_3/defiant_vanguard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Defiant Vanguard ({2}{W} 2/2 Creature — Human Rebel):
/// "When this creature blocks, at end of combat, destroy it and all creatures it blocked this turn.
/// {5}, {T}: Search your library for a Rebel permanent card with mana value 4 or less, put it onto the battlefield, then shuffle."
#[test]
fn defiant_vanguard_destroys_itself_and_blocked_creature_at_end_of_combat_and_recruits_rebels() {
    let subject = card_index("680766e2-637b-40bc-a15f-0b4dc55bd619");
    let mut engine = Duel::new(SEED, index::NIGHTWIND_GLIDER)
        .battlefield(0, &[subject, forest()])
        .hand(0, &[index::FOG])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // First test the {5}, {T} Rebel recruitment ability.
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 5);
    engine.refresh_offer();

    activate(&mut engine, P0, subject, 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search prompt");
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, index::NIGHTWIND_GLIDER).is_some(),
        "Defiant Vanguard recruited Nightwind Glider onto the battlefield"
    );

    // Let Vanguard untap normally after recruitment, then prevent combat damage
    // so both creatures are still present when the delayed destruction resolves.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    let vanguard = on_battlefield(&engine, P0, subject).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    cast_from_hand(&mut engine, P0, index::FOG);
    pass_until(&mut engine, stack_is_empty);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(vanguard, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd && !stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(vanguard).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(giant).unwrap().zone,
        Zone::Battlefield
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(vanguard).unwrap().zone,
        Zone::Graveyard,
        "Defiant Vanguard was destroyed at end of combat"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().zone,
        Zone::Graveyard,
        "the blocked Hill Giant was destroyed at end of combat"
    );
}
