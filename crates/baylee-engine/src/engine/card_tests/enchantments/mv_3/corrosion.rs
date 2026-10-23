//! `cards/enchantments/mv_3/corrosion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters::RUST;

fn corrosion() -> CardIndex {
    card_index("3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043")
}

/// Walks to the next cumulative-upkeep question, checks that it is p0's and
/// names `price`, pays it out of every source p0 has and lets the upkeep's
/// other trigger resolve too.
#[track_caller]
fn pay_cumulative_upkeep(engine: &mut Engine<RegistryLookup>, price: baylee_core::mana::ManaCost) {
    let p0 = PlayerId::new(0);
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "\"at the beginning of *your* upkeep\"");
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana { cost: price },
        "one {{1}} per age counter"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(engine, stack_is_empty);
}

/// Corrosion (`{1}{B}{R}`, `Coverage::Partial`): at the beginning of your
/// upkeep each artifact the opponent controls gets a rust counter, then each
/// artifact with mana value at most its rust counters is destroyed. After the
/// first upkeep Sol Ring (value 1, 1 counter) is gone and Lightning Greaves
/// (value 2) and Chromatic Lantern (value 3) have one counter; after the
/// second Greaves (2 counters) is gone and the Lantern (2 counters, value 3)
/// is not. The opponent's own upkeep adds nothing, and p0's own Sol Ring gets
/// no counter and is not destroyed.
#[test]
fn corrosion_rusts_opponents_artifacts_and_destroys_those_whose_value_is_at_most_their_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                mountain(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                sol_ring(),
            ],
        )
        .battlefield(1, &[sol_ring(), lightning_greaves(), chromatic_lantern()])
        .hand(0, &[corrosion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let their_greaves = on_battlefield(&engine, p1, lightning_greaves()).unwrap();
    let their_lantern = on_battlefield(&engine, p1, chromatic_lantern()).unwrap();
    cast_from_hand(&mut engine, p0, corrosion());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, corrosion()).is_some());

    // The opponent's upkeep is not "your upkeep": nothing yet.
    reach_their_main_phase(&mut engine, p1);
    assert!(on_battlefield(&engine, p1, sol_ring()).is_some());
    assert_eq!(counters_on(&engine, their_greaves, RUST), 0);

    // p0's first upkeep: age 1, price {1}.
    pay_cumulative_upkeep(&mut engine, baylee_core::mana!("{1}"));
    assert!(
        in_graveyard(&engine, p1, sol_ring()).is_some(),
        "mana value 1 with 1 rust counter: destroyed"
    );
    assert_eq!(counters_on(&engine, their_greaves, RUST), 1);
    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_some(),
        "mana value 2 with 1 counter: stays"
    );
    assert_eq!(counters_on(&engine, their_lantern, RUST), 1);
    assert_eq!(
        counters_on(&engine, my_ring, RUST),
        0,
        "only the opponent's"
    );
    assert!(on_battlefield(&engine, p0, sol_ring()).is_some());

    // p0's second upkeep: age 2, price {2}.
    pay_cumulative_upkeep(&mut engine, baylee_core::mana!("{2}"));
    assert!(
        in_graveyard(&engine, p1, lightning_greaves()).is_some(),
        "mana value 2 with 2 rust counters: destroyed"
    );
    assert_eq!(counters_on(&engine, their_lantern, RUST), 2);
    assert!(
        on_battlefield(&engine, p1, chromatic_lantern()).is_some(),
        "mana value 3 with 2 counters: stays"
    );
    assert!(on_battlefield(&engine, p0, corrosion()).is_some());
}

/// Cumulative upkeep: refused, Corrosion is sacrificed.
#[test]
fn corrosion_is_sacrificed_when_its_cumulative_upkeep_goes_unpaid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), swamp()])
        .hand(0, &[corrosion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, corrosion());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, corrosion()).is_none());
    assert!(in_graveyard(&engine, p0, corrosion()).is_some());
}
