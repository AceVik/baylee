//! `cards/creatures/mv_3/musician.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn musician() -> CardIndex {
    card_index("0d6e9468-adb2-4feb-87cf-9113db8717b1")
}

/// Musician (0d6e9468-adb2-4feb-87cf-9113db8717b1) — {2}{U} 1/3 Creature — Human Wizard:
/// - "Cumulative upkeep {1}"
/// - "{T}: Put a music counter on target creature. If it doesn't have 'At the beginning of your
///   upkeep, destroy this creature unless you pay {1} for each music counter on it,' it gains
///   that ability."
#[test]
fn musician_puts_music_counters_on_target_creature_and_has_cumulative_upkeep_one() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[musician(), island()])
        .battlefield(1, &[grizzly_bears(), forest()])
        .start();
    keep_mulligans(&mut engine);

    let mus = on_battlefield(&engine, P0, musician()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    // Upkeep 1 (turn 1): Musician's cumulative upkeep {1} puts 1 age counter on it; P0 pays {1}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine
            .state()
            .object(mus)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // Main phase (turn 1): P0 taps Musician targeting P1's Grizzly Bears -> 1 music counter + upkeep ability.
    reach_main_phase(&mut engine, P0);
    activate(&mut engine, P0, musician(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::MUSIC),
        1
    );

    // P1's first upkeep (turn 2): Grizzly Bears triggers; P1 pays {1} and keeps Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P1);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "paying {{1}} for the music counter keeps Grizzly Bears alive"
    );

    // P0's second upkeep (turn 3): Musician puts 2nd age counter; P0 declines cumulative upkeep {2},
    // sacrificing Musician, and in response taps Musician to put a 2nd music counter on Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, musician()).is_none()
            && in_graveyard(&engine, P0, musician()).is_some(),
        "Musician is sacrificed when cumulative upkeep is declined"
    );

    // P1's second upkeep (turn 4): P1 declines to pay {1} for the music counter -> Grizzly Bears is destroyed.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_none()
            && in_graveyard(&engine, P1, grizzly_bears()).is_some(),
        "declining to pay for the music counter destroys Grizzly Bears"
    );
}

#[test]
fn musician_second_music_counter_raises_the_price_without_a_second_upkeep_trigger() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[musician(), island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    // Turn 1 upkeep: Musician's own cumulative upkeep {1}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });

    // First music counter: the creature gains the upkeep ability.
    reach_main_phase(&mut engine, P0);
    activate(&mut engine, P0, musician(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    // Turn 2 (P1's upkeep): one music counter, so {1}.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        ),
        "one music counter: pay {{1}}, got {:?}",
        engine.pending()
    );
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P1);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });

    // Turn 3: Musician's cumulative upkeep is {2} now (paid), and a second music counter goes on
    // the same creature, which already has the ability.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    reach_main_phase(&mut engine, P0);
    activate(&mut engine, P0, musician(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::MUSIC),
        2
    );

    // Turn 4 (P1's upkeep): two music counters, so {2}, asked once. A second copy of the ability
    // would ask twice, and the pass below would stop on the second question.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            }
        ),
        "two music counters: pay {{2}}, got {:?}",
        engine.pending()
    );
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P1);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P1, grizzly_bears()).is_some(),
        "paying {{2}} for two music counters keeps the creature"
    );
}
