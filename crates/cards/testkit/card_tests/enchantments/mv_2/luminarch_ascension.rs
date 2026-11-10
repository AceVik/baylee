//! `cards/enchantments/mv_2/luminarch_ascension.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Luminarch Ascension: a "may" *inside* an intervening-if clause
/// (CR 603.4), which is the shape that put a suspending effect inside a
/// nested branch for the first time.
///
/// The assertion that matters is the count. Running the branch a second
/// time on resume would have asked twice and taken two quest counters —
/// four end steps would have finished the card in two — and nothing in the
/// engine would have complained.
#[test]
fn an_optional_clause_inside_a_condition_is_offered_once_and_taken_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, plains())
        .battlefield(0, &[luminarch_ascension()])
        .start();
    keep_mulligans(&mut engine);
    let ascension =
        on_battlefield(&engine, p0, luminarch_ascension()).expect("the enchantment is out");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(ascension)
            .expect("still on the battlefield")
            .counters
            .get(baylee_cards_dsl::counters::QUEST),
        1,
        "one offer, one counter"
    );
}

/// Luminarch Ascension's other branch: "if you didn't lose life this turn"
/// on a turn its controller did. The test above is the turn with no loss.
///
/// The opponent casts a Lightning Bolt in their own main phase, and the two
/// games differ only in who it hits. Aimed at the opponent, the end step
/// still offers the counter. That is the control, and it is what keeps the
/// other half from passing on a trigger that never fired. Aimed at the
/// Ascension's controller, the condition is false on resolution, so nothing
/// is asked and no counter is placed (CR 603.4).
#[test]
fn luminarch_ascension_asks_nothing_on_a_turn_its_controller_lost_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let end_step = |bolted: PlayerId| {
        let mut engine = Duel::new(42, plains())
            .battlefield(0, &[luminarch_ascension()])
            .battlefield(1, &[mountain()])
            .hand(1, &[lightning_bolt()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");
        tap_all_mana(&mut engine, p1);
        cast_with_floating(&mut engine, p1, lightning_bolt());
        engine
            .apply(
                p1,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![bolted],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[bolted.get() as usize].life,
            17,
            "the Bolt hit"
        );
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::MayDo,
                    ..
                }
            ) || e.state().turn.active == p0
        });
        let asked = matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        );
        let ascension =
            on_battlefield(&engine, p0, luminarch_ascension()).expect("the enchantment is out");
        let quest = engine
            .state()
            .object(ascension)
            .expect("still on the battlefield")
            .counters
            .get(baylee_cards_dsl::counters::QUEST);
        (asked, quest)
    };
    assert_eq!(
        end_step(p1),
        (true, 0),
        "the opponent's loss is not the controller's: the counter is offered"
    );
    assert_eq!(
        end_step(p0),
        (false, 0),
        "three damage to the controller is three life lost this turn"
    );
}

// ---- Abilities no test had fired (L4 sweep, 2026-10-01) ----

/// Luminarch Ascension: "{1}{W}: Create a 4/4 white Angel creature token
/// with flying. Activate only if this enchantment has four or more quest
/// counters on it." Three counters: not offered. Four: an Angel arrives.
#[test]
fn luminarch_ascension_makes_an_angel_only_from_four_quest_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2501, forest())
        .battlefield(0, &[luminarch_ascension(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let asc = on_battlefield(&engine, p0, luminarch_ascension()).expect("ascension");
    tap_mana_where(&mut engine, p0, |id| id != asc);
    let set = |engine: &mut Engine<RegistryLookup>, n: u16| {
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up")
            .object_mut(asc)
            .expect("on the table")
            .counters
            .set(baylee_cards_dsl::counters::QUEST, n);
        engine.refresh_offer();
    };

    set(&mut engine, 3);
    assert!(
        !priority_offer(&engine).abilities.contains(&(asc, 1)),
        "three quest counters are not enough"
    );
    set(&mut engine, 4);
    assert!(priority_offer(&engine).abilities.contains(&(asc, 1)));

    activate(&mut engine, p0, luminarch_ascension(), 1);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Angel");
    assert_eq!(pt(&engine, tokens[0]), (4, 4));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
}
