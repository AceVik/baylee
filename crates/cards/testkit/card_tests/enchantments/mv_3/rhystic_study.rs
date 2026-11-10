//! `cards/enchantments/mv_3/rhystic_study.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rhystic Study` is an enchantment costing `{2}{U}` under `Coverage::Implemented`.
/// It prints "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}."
/// When an opponent casts a spell, the triggered ability prompts the opponent with `YesNoPrompt::PayTax`,
/// and when the opponent declines to pay, its controller draws a card.
#[test]
fn rhystic_study_triggers_on_opponent_cast_and_draws_when_tax_declined() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[rhystic_study()])
        .hand(0, &[])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, llanowar_elves());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { .. },
                ..
            } if *player == p1
        )
    });

    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "controller drew one card after opponent declined to pay the tax"
    );
}

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn hand_of(engine: &Engine<RegistryLookup>, n: u8) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat(n))).len()
}

fn is_tax_question(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayTax { .. },
            ..
        }
    )
}

/// Seat 0's Rhystic Study at a table of four and seat `caster` on its main
/// phase with `forests` Forests and an Elves in hand.
fn a_study_with_a_caster(caster: u8, forests: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &[rhystic_study()])
        .battlefield(usize::from(caster), &vec![forest(); forests])
        .hand(usize::from(caster), &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(caster));
    engine
}

/// Seat 2 casts into seat 0's Study at a table of four: seat 2 alone is
/// asked to pay {1}. Seat 1, the first opponent in seat order, and the
/// Study's own controller are asked nothing.
#[test]
fn rhystic_study_asks_only_the_player_who_cast() {
    let mut engine = a_study_with_a_caster(2, 1);
    cast_from_hand(&mut engine, seat(2), llanowar_elves());
    pass_until(&mut engine, is_tax_question);

    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            } if *player == seat(2)
        ),
        "seat 2 is asked for {{1}}: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.awaited().iter().collect::<Vec<_>>(),
        vec![seat(2)],
        "no other seat has a question"
    );
    for other in [0, 1, 3] {
        assert!(
            engine.pending_for(seat(other)).is_none(),
            "seat {other} is asked nothing"
        );
    }
}

/// Declined: the Study's controller, and only the controller, is asked
/// whether to draw ("you *may* draw"). No draws nothing, and yes draws one.
#[test]
fn rhystic_study_declined_asks_its_controller_whether_to_draw() {
    for (draws, expected) in [(false, 0), (true, 1)] {
        let mut engine = a_study_with_a_caster(2, 1);
        let before = hand_of(&engine, 0);
        cast_from_hand(&mut engine, seat(2), llanowar_elves());
        pass_until(&mut engine, is_tax_question);
        engine.apply(seat(2), PlayerAction::YesNo(false)).unwrap();

        assert!(
            matches!(
                engine.pending(),
                Pending::YesNo {
                    player,
                    prompt: YesNoPrompt::MayDo,
                    ..
                } if *player == seat(0)
            ),
            "the controller is asked, not the caster: {:?}",
            engine.pending()
        );
        assert_eq!(
            engine.awaited().iter().collect::<Vec<_>>(),
            vec![seat(0)],
            "only the controller has a question"
        );
        assert_eq!(
            hand_of(&engine, 0),
            before,
            "nothing is drawn before the answer"
        );

        engine.apply(seat(0), PlayerAction::YesNo(draws)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            hand_of(&engine, 0),
            before + expected,
            "answered {draws}: {expected} card(s) drawn"
        );
    }
}

/// Paid: no draw and no question about one. The caster keeps one mana
/// floating beyond the Elves, pays the {1} with it, and the Study's
/// controller is never asked anything.
#[test]
fn rhystic_study_paid_asks_nobody_about_a_draw() {
    let mut engine = a_study_with_a_caster(2, 2);
    let before = hand_of(&engine, 0);
    cast_from_hand(&mut engine, seat(2), llanowar_elves());
    pass_until(&mut engine, is_tax_question);
    assert_eq!(
        engine.state().players[2].mana_pool.total(),
        1,
        "two Forests floated, the Elves took one"
    );

    engine.apply(seat(2), PlayerAction::YesNo(true)).unwrap();
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "paid: there is no draw to ask about: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[2].mana_pool.total(),
        0,
        "the {{1}} left the pool"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_of(&engine, 0), before, "paid, so nobody draws");
}

/// Seats 0 and 2 are one team against seats 1 and 3. A teammate's spell is
/// not an opponent's and does not trigger the Study; the opponent's spell,
/// cast on its own turn first, does, so the quiet is the Study's rule and
/// not a Study that was never going to fire on this board.
#[test]
fn rhystic_study_ignores_a_teammates_spell() {
    let mut duel = Duel::table(SEED, island(), 4)
        .battlefield(0, &[rhystic_study()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .battlefield(2, &[forest()])
        .hand(2, &[llanowar_elves()]);
    for (s, side) in [(0, 1), (1, 2), (2, 1), (3, 2)] {
        duel = duel.team(s, side);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);

    // Seat 1, an opponent, casts first (its turn comes first) and is asked.
    reach_their_main_phase(&mut engine, seat(1));
    cast_from_hand(&mut engine, seat(1), llanowar_elves());
    pass_until(&mut engine, is_tax_question);
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == seat(1)),
        "the opponent is taxed: {:?}",
        engine.pending()
    );
    engine.apply(seat(1), PlayerAction::YesNo(false)).unwrap();
    // The controller's "may draw", answered yes by the pass.
    pass_until(&mut engine, stack_is_empty);

    // Seat 2, seat 0's teammate: no question of any kind (`pass_until`
    // panics on a tax question it was not told to expect), and no draw.
    reach_their_main_phase(&mut engine, seat(2));
    let before = hand_of(&engine, 0);
    cast_from_hand(&mut engine, seat(2), llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, seat(2), llanowar_elves()).is_some(),
        "the teammate's Elves resolved"
    );
    assert_eq!(
        hand_of(&engine, 0),
        before,
        "a teammate's spell draws nothing"
    );
}
