//! `cards/creatures/mv_5/talas_researcher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn talas_researcher() -> CardIndex {
    card_index("1ed662d7-65b9-4024-a892-c4559fb6aaf9")
}

/// `{T}: Draw a card` works on your turn before attackers are declared, and is
/// withheld once attackers are declared or during an opponent's turn.
#[test]
fn talas_researcher_draws_a_card_only_on_your_turn_before_attackers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                talas_researcher(),
            ],
        )
        .hand(0, &[talas_researcher()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let islands = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| islands.contains(&id));
    let spell = in_hand(&engine, p0, talas_researcher()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance a full turn cycle so both Researchers lose summoning sickness.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let researchers = all_on_battlefield(&engine, p0, talas_researcher());
    let (first, second) = (researchers[0], researchers[1]);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: first,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, first));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );

    // Second Researcher is offered in CombatBegin, then withheld in SecondMain and on p1's turn.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin && matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(legal.abilities.contains(&(second, 0)));

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!legal.abilities.contains(&(second, 0)));

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*player, p0);
    assert!(!legal.abilities.contains(&(second, 0)));
}
