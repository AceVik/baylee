//! `cards/creatures/mv_4/sheoldred_the_apocalypse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A trigger that can find no legal target takes *itself* off the queue and
/// nothing else.
///
/// `collect_triggers` pops the entry it is working on before it asks a
/// synthetic trigger for its target, so the branch that drops a granted
/// trigger with no legal target (CR 603.3d) was popping a second time — and
/// the second pop took whatever was queued behind it, unread and unresolved.
///
/// Wizard Class at level 3 is the only card in the pool that grants a
/// *targeted* trigger, and a Class is an enchantment, so its controller can
/// hold it with no creature anywhere to put the counter on. The draw that
/// fires it fires Sheoldred across the table on the same event, and
/// Sheoldred's is the trigger that was being eaten: the life it takes is the
/// whole assertion.
#[test]
fn a_trigger_that_finds_no_target_takes_only_itself_off_the_queue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![island(); 8];
    board.push(wizard_class());
    let mut engine = Duel::new(9, quiet_artifact())
        .battlefield(0, &board)
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Both levels in one main phase: eight Islands is {2}{U} and {4}{U}
    // exactly, and a mana pool empties at the end of a step, not on a pass.
    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, wizard_class(), 2);
    pass_until(&mut engine, stack_is_empty);

    let before = engine.state().players[0].life;
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        before - 2,
        "Sheoldred's trigger was queued behind one that fizzled, and still resolved",
    );
}

/// And the same trigger *answered* takes only itself off the queue.
///
/// The fizzle branch and the answer path are two pops for one queue entry,
/// both of them after the tail pop that already removed it. This is the half
/// a player actually reaches: a creature on the board means the granted
/// trigger has a target, the question is asked, and it was the answer that
/// ate the trigger behind it — so the more a board has going on, the more
/// there is to lose.
#[test]
fn answering_a_granted_triggers_target_takes_only_itself_off_the_queue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![island(); 8];
    board.push(wizard_class());
    board.push(quiet_creature());
    let mut engine = Duel::new(9, quiet_artifact())
        .battlefield(0, &board)
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, wizard_class(), 2);
    pass_until(&mut engine, stack_is_empty);

    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elves are out");
    let before = engine.state().players[0].life;
    reach_their_main_phase(&mut engine, p1);
    // `walk_to_own_main` and not `reach_their_main_phase`: the draw step on
    // the way asks for the counter's target, which passing cannot answer.
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round"
    );

    assert_eq!(
        engine.state().players[0].life,
        before - 2,
        "Sheoldred's trigger was queued behind one that was answered, and still resolved",
    );
    assert_eq!(
        engine
            .state()
            .object(elves)
            .map(|o| o.counters.get(CounterKind::P1P1)),
        Some(1),
        "the granted trigger put its own counter down",
    );
}

/// A two-card draw is two draws, and a draw-watcher fires for both.
///
/// `draw_cards` records one `CardsDrawn { count }` for the whole draw, so an
/// ability that watches draws saw one event and fired once — entry 35's
/// defect in the shape its fix could not see, a batch that is a field rather
/// than a list of events.
///
/// Wizard Class's own level-up draws two cards and Sheoldred, the Apocalypse
/// takes 2 life per card an opponent draws, so the assertion is a **count**
/// in both directions: 2 life is the old bug, 6 would be firing per card and
/// per event both, and 4 is the card.
#[test]
fn a_two_card_draw_fires_a_draw_watcher_twice() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(21, quiet_artifact())
        .battlefield(0, &[wizard_class(), island(), island(), island()])
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    let before = engine.state().players[0].life;
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        before - 4,
        "two cards drawn, so Sheoldred took 2 life twice",
    );
}

/// Delighted Halfling: "{T}: Add {C}." and "{T}: Add one mana of any color.
/// Spend this mana only to cast a legendary spell, and that spell can't be
/// countered."
///
/// Three claims in one sentence, and each is satisfiable on its own by a
/// card that is wrong. So the board is built to separate them.
///
/// The spend restriction is read against **Ravenous Chupacabra**, which
/// costs exactly the `{2}{B}{B}` Sheoldred does and is not legendary: with
/// three Swamps tapped neither is castable, and the Halfling's fourth mana
/// makes one of them castable and not the other. Same cost, same colour,
/// same window — the supertype is the only thing left to be doing the work.
/// A test that had paired the legend with a cheaper commoner would have
/// passed against a restriction that did nothing at all.
///
/// The rider is read against a real Counterspell taken to resolution,
/// because "can't be countered" is not "can't be targeted": the Counterspell
/// must still be offered the Sheoldred as a target and must still resolve,
/// and what differs is only where the creature ends up.
#[test]
#[allow(clippy::too_many_lines)] // one printed sentence, three clauses, one board
fn delighted_halflings_mana_pays_only_for_the_legend_and_makes_it_uncounterable() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(79, forest())
        .battlefield(0, &[delighted_halfling(), swamp(), swamp(), swamp()])
        .hand(0, &[sheoldred_the_apocalypse(), ravenous_chupacabra()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let halfling = on_battlefield(&engine, p0, delighted_halfling()).expect("the Halfling is out");
    let legend = in_hand(&engine, p0, sheoldred_the_apocalypse()).expect("Sheoldred is in hand");
    let commoner = in_hand(&engine, p0, ravenous_chupacabra()).expect("the Chupacabra is in hand");

    // Two printed mana abilities, and the card is offered both of them.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(halfling, 0)) && legal.abilities.contains(&(halfling, 1)),
        "the plain {{C}} is ability 0 and the restricted any-colour is ability 1: {:?}",
        legal.abilities
    );

    // Three Swamps is one mana short of either four-drop. Neither is
    // castable yet, which is what makes the comparison below a comparison —
    // so the Halfling is named as the one source kept back, since it is the
    // fourth mana and this test is about which spell that mana may pay for.
    tap_mana_except(&mut engine, p0, halfling);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "three Swamps, and the Halfling held back"
    );
    assert!(
        !legal.castable.contains(&legend) && !legal.castable.contains(&commoner),
        "{{2}}{{B}}{{B}} is four mana and three are floating"
    );

    // "Add one mana of any color" — and the choice really is all five.
    activate(&mut engine, p0, delighted_halfling(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    let all_colors = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ];
    assert_eq!(options, all_colors, "one mana of any color");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("colour chosen");

    // The fourth mana is black and is not *free* black: the rider rides on
    // the mana, so the pool keeps it as a restricted entry.
    {
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 4, "three Swamps and the Halfling");
        assert_eq!(
            pool.available(ManaColor::Black),
            3,
            "the Halfling's black is not one of the three"
        );
        let restricted = pool.restricted();
        assert_eq!(restricted.len(), 1);
        assert_eq!(restricted[0].color, ManaColor::Black);
        assert_eq!(restricted[0].amount, 1);
        assert!(
            engine
                .state()
                .restriction_info
                .contains_key(&restricted[0].restriction.0),
            "the spend restriction is registered, or nothing can check it"
        );
    }

    // The whole of "spend this mana only to cast a legendary spell", in one
    // priority window and against one cost.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&legend),
        "a legendary spell is what this mana is for"
    );
    assert!(
        !legal.castable.contains(&commoner),
        "Ravenous Chupacabra costs the same {{2}}{{B}}{{B}} and is not legendary"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: legend })
        .expect("Sheoldred is cast with the Halfling's mana");
    // `total()` counts the restricted entry too, so nought is the whole
    // pool: the restricted black paid rather than sitting the cast out.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "all four mana went into it, the Halfling's among them"
    );

    // "…and that spell can't be countered." A hard counter, a legal target,
    // taken all the way to resolution.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "expected the opponent to hold priority, got {:?}",
        engine.pending()
    );
    tap_all_mana(&mut engine, p1);
    let cs = in_hand(&engine, p1, counterspell()).expect("Counterspell is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .expect("two Islands pay for it");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![legend],
        "can't be countered is not can't be targeted — the spell is still a legal target"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![legend],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, sheoldred_the_apocalypse()).is_some()
    });
    assert!(
        in_graveyard(&engine, p0, sheoldred_the_apocalypse()).is_none(),
        "the spell the Halfling's mana paid for arrived instead of being countered"
    );
    assert!(
        in_graveyard(&engine, p1, counterspell()).is_some(),
        "the Counterspell itself resolved, and did nothing"
    );
}

/// Sheoldred at a table of four, each seat's draw step watched on its own.
///
/// "Whenever an opponent draws a card, they lose 2 life": *they*, the one
/// who drew. Heads-up that and "each opponent" are one seat, so the drain
/// was written against every two-player test green. Here seat 2's draw must
/// cost seat 2 and leave seats 1 and 3 alone; and "whenever you draw a card,
/// you gain 2 life" fires for the controller's own draw and for nobody
/// else's.
#[test]
fn sheoldred_drains_only_the_opponent_who_drew_and_gains_for_its_own_draw() {
    let seat = PlayerId::new;
    let lives = |e: &Engine<RegistryLookup>| -> Vec<i32> {
        e.state().players.iter().map(|p| p.life).collect()
    };
    let mut engine = Duel::table(SEED, island(), 4)
        .battlefield(0, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);

    // The first turn is the controller's, and it draws: +2 for it alone.
    let initial = lives(&engine);
    reach_main_phase(&mut engine, seat(0));
    let start = lives(&engine);
    assert_eq!(
        start,
        [initial[0] + 2, initial[1], initial[2], initial[3]],
        "the controller drew on turn one: it gains 2 and nobody else moves"
    );

    reach_their_main_phase(&mut engine, seat(1));
    pass_until(&mut engine, stack_is_empty);
    let after_one = lives(&engine);
    assert_eq!(
        after_one,
        [start[0], start[1] - 2, start[2], start[3]],
        "seat 1 drew: seat 1 loses 2, the controller gains nothing, nobody else moves"
    );

    // The draw that is not the first opponent's in seat order.
    reach_their_main_phase(&mut engine, seat(2));
    pass_until(&mut engine, stack_is_empty);
    let after_two = lives(&engine);
    assert_eq!(
        after_two,
        [after_one[0], after_one[1], after_one[2] - 2, after_one[3]],
        "seat 2 drew: seat 2 loses 2, seat 1 nothing more, seat 3 nothing"
    );

    reach_their_main_phase(&mut engine, seat(3));
    pass_until(&mut engine, stack_is_empty);
    let after_three = lives(&engine);
    assert_eq!(
        after_three,
        [after_two[0], after_two[1], after_two[2], after_two[3] - 2],
        "seat 3 drew: seat 3 loses 2 and seat 2 nothing more"
    );

    // The controller's own draw: it gains 2 and nobody loses anything.
    reach_their_main_phase(&mut engine, seat(0));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        lives(&engine),
        [
            after_three[0] + 2,
            after_three[1],
            after_three[2],
            after_three[3]
        ],
        "the controller drew: it gains 2 and no opponent loses life"
    );
}
