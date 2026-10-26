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
    let mut engine = Duel::new(SEED, index::NIGHTWIND_GLIDER)
        .battlefield(0, &[card_index("680766e2-637b-40bc-a15f-0b4dc55bd619")])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // First test the {5}, {T} Rebel recruitment ability.
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 5);
    engine.refresh_offer();

    activate(
        &mut engine,
        P0,
        card_index("680766e2-637b-40bc-a15f-0b4dc55bd619"),
        1,
    );
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

    // Untap on P1's turn? No, untap Defiant Vanguard and let P1's 3/3 Hill Giant attack;
    // give Defiant Vanguard +0/+4 so it survives combat damage, then verify both die at end of combat!
    reach_their_main_phase(&mut engine, P1);
    let vanguard = on_battlefield(
        &engine,
        P0,
        card_index("680766e2-637b-40bc-a15f-0b4dc55bd619"),
    )
    .unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(vanguard)
        .unwrap()
        .status
        .remove(crate::object::Status::TAPPED);
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(vanguard)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 3);
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    engine
        .dev_state_mut(P1)
        .unwrap()
        .object_mut(giant)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 3);
    engine.refresh_offer();

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
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        on_battlefield(
            &engine,
            P0,
            card_index("680766e2-637b-40bc-a15f-0b4dc55bd619")
        )
        .is_none()
            && in_graveyard(
                &engine,
                P0,
                card_index("680766e2-637b-40bc-a15f-0b4dc55bd619")
            )
            .is_some(),
        "Defiant Vanguard was destroyed at end of combat"
    );
    assert!(
        on_battlefield(&engine, P1, index::HILL_GIANT).is_none()
            && in_graveyard(&engine, P1, index::HILL_GIANT).is_some(),
        "the 6/6 Hill Giant blocked by Defiant Vanguard was destroyed at end of combat"
    );
}
