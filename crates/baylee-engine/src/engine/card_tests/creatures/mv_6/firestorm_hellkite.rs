//! `cards/creatures/mv_6/firestorm_hellkite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn firestorm_hellkite() -> CardIndex {
    card_index("533e41cc-7cae-47f2-886b-8a3ffb82bf81")
}

/// P0's board for the cumulative upkeep: two Islands and four Mountains, the Hellkite in hand.
fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                island(),
                island(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[firestorm_hellkite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, firestorm_hellkite());
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// Passes until P0 is asked whether to pay the cumulative upkeep and says for how much.
#[track_caller]
fn walk_to_price(engine: &mut Engine<RegistryLookup>) -> baylee_core::mana::ManaCost {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0, "the controller is asked");
    assert_eq!(engine.state().turn.active, P0, "on its own upkeep");
    cost
}

/// `Firestorm Hellkite` ({4}{U}{R}, 6/6 Dragon): flying and trample.
#[test]
fn firestorm_hellkite_is_a_six_six_flier_with_trample() {
    let engine = board();
    let hellkite = on_battlefield(&engine, P0, firestorm_hellkite()).unwrap();
    assert_eq!(pt(&engine, hellkite), (6, 6));
    assert!(keywords(&engine, hellkite).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, hellkite).contains(KeywordSet::TRAMPLE));
}

/// "Cumulative upkeep {U}{R}": the first upkeep puts one age counter on it and asks {U}{R}, paid
/// with exactly one Island and one Mountain; the second asks {U}{R} twice, one more counter. The
/// opponent's upkeep in between asks nothing.
#[test]
fn firestorm_hellkite_cumulative_upkeep_grows_by_ur_each_turn() {
    let mut engine = board();
    let hellkite = on_battlefield(&engine, P0, firestorm_hellkite()).unwrap();
    let age = baylee_cards_dsl::counters::AGE;

    assert_eq!(
        walk_to_price(&mut engine),
        baylee_core::mana!("{U}{R}"),
        "one counter, {{U}}{{R}}"
    );
    assert_eq!(counters_on(&engine, hellkite, age), 1);
    let blue_land = on_battlefield(&engine, P0, island()).unwrap();
    let red_land = on_battlefield(&engine, P0, mountain()).unwrap();
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_mana_where(&mut engine, P0, |land| {
        land == blue_land || land == red_land
    });
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, firestorm_hellkite()).is_some());
    assert_eq!(
        lands_of(&engine, P0)
            .into_iter()
            .filter(|land| is_tapped(&engine, *land))
            .count(),
        2,
        "two lands paid {{U}}{{R}}"
    );

    assert_eq!(
        walk_to_price(&mut engine),
        baylee_core::mana!("{U}{R}{U}{R}"),
        "two counters, {{U}}{{R}} twice"
    );
    assert_eq!(counters_on(&engine, hellkite, age), 2);
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, firestorm_hellkite()).is_some(),
        "paid twice over, it stays"
    );
}

/// Refusing the price sacrifices it.
#[test]
fn firestorm_hellkite_is_sacrificed_when_the_upkeep_is_not_paid() {
    let mut engine = board();
    let hellkite = on_battlefield(&engine, P0, firestorm_hellkite()).unwrap();
    walk_to_price(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(hellkite).unwrap().zone,
        Zone::Graveyard
    );
}
