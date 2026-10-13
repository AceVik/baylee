//! `cards/lands/check/archway_of_innovation.rs`, played.
//!
//! Archway of Innovation: "This land enters tapped unless you control an
//! Island. / {T}: Add {U}. / {U}, {T}: The next spell you cast this turn has
//! improvise. (Your artifacts can help cast that spell. Each artifact you tap
//! after you're done activating mana abilities pays for {1}.)"
//!
//! Improvise is asked as a tap question during the cast
//! (`Pending::ChooseTargets` with the convoke prompt), after the spell's
//! total cost is known and before it is paid (CR 702.126a, 702.126b). The
//! spell it is tried on is a Darksteel Pendant, `{2}` with no coloured part
//! and no mana ability to be tapped by mistake; its four copies on the board
//! are the artifacts that pay. The grant is found by what the Archway is
//! offered: the ability after the mana ability.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

/// Player 0 in the first main phase with the Archway, `islands` Islands and
/// four untapped Darksteel Pendants on the battlefield and `hand` in hand.
/// Returns the engine, the Archway and the Islands.
#[track_caller]
fn the_archway(
    islands: usize,
    extra: &[CardIndex],
    hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let mut board = vec![archway_of_innovation()];
    board.extend(vec![island(); islands]);
    board.extend(vec![darksteel_pendant(); 4]);
    board.extend_from_slice(extra);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let archway = on_battlefield(&engine, kit::P0, archway_of_innovation()).expect("the Archway");
    let islands = objects_of(&engine, island());
    (engine, archway, islands)
}

/// Player 0's untapped-or-not permanents printed as `card`, in battlefield
/// order.
fn objects_of(engine: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == kit::P0 && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Activates the Archway's grant, `{U}, {T}`, off the blue mana that is
/// floating, and lets it resolve. The grant is the last ability offered,
/// after the mana ability.
#[track_caller]
fn grant_improvise(engine: &mut Engine<RegistryLookup>, archway: ObjectId) {
    let offered = kit::offered(engine, archway);
    let grant = offered.iter().copied().max().expect("something is offered");
    assert!(
        grant >= 1,
        "an ability beyond the mana ability is offered: {offered:?}"
    );
    kit::activate_object(engine, kit::P0, archway, grant);
    pass_until(engine, stack_is_empty);
}

/// Archway of Innovation: "This land enters tapped unless you control an
/// Island." / "{T}: Add {U}."
///
/// Controlling an Island allows Archway of Innovation to enter untapped and
/// immediately tap for blue mana. (The first test of the card, kept as it
/// was: only its comment, which named the unfinished card, is rewritten.)
#[test]
fn archway_of_innovation_enters_untapped_with_island_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(302, forest())
        .battlefield(0, &[island()])
        .hand(0, &[archway_of_innovation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, archway_of_innovation());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, archway_of_innovation(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// "This land enters tapped unless you control an Island."
///
/// Untapped for an Island and for a nonbasic land that has the Island type
/// (Irrigated Farmland), tapped for no lands, for a Forest and for a Plains
/// and a Swamp: the tapped cases are what make the untapped ones count.
#[test]
fn archway_of_innovation_enters_tapped_unless_you_control_an_island() {
    let cases: [(&str, Vec<CardIndex>, bool); 5] = [
        ("an Island", vec![island()], false),
        (
            "an Irrigated Farmland (Plains Island)",
            vec![irrigated_farmland()],
            false,
        ),
        ("no land", vec![], true),
        ("a Forest", vec![forest()], true),
        ("a Plains and a Swamp", vec![plains(), swamp()], true),
    ];
    for (name, board, tapped) in cases {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .hand(0, &[archway_of_innovation()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, kit::P0);

        let land = play_land(&mut engine, kit::P0, archway_of_innovation());
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(
            entered_tapped(&engine, land),
            tapped,
            "controlling {name}, the Archway enters {}",
            if tapped { "tapped" } else { "untapped" }
        );
    }
}

/// "{U}, {T}: The next spell you cast this turn has improvise."
///
/// With the grant, a `{2}` Pendant is cast off two untapped artifacts and no
/// mana at all: the cast asks which artifacts to tap, offers all four, and
/// the two named are tapped while the other two are not. Improvise is not a
/// cost reduction (CR 702.126b): the spell on the stack is still mana value 2.
///
/// The grant is spent by that cast. A second Pendant cast right after, off two
/// Islands, asks no tap question although three artifacts are untapped, and
/// the Islands pay the whole cost.
#[test]
fn archway_of_innovation_gives_only_the_next_spell_improvise() {
    let (mut engine, archway, islands) =
        the_archway(3, &[], &[darksteel_pendant(), darksteel_pendant()]);
    let pendants = objects_of(&engine, darksteel_pendant());
    assert_eq!(pendants.len(), 4, "four Pendants on the table");
    tap_mana_where(&mut engine, kit::P0, |id| id == islands[0]);
    grant_improvise(&mut engine, archway);
    assert!(is_tapped(&engine, archway), "{{T}} is part of the cost");

    cast_with_floating(&mut engine, kit::P0, darksteel_pendant());
    let (options, _) = tap_to_pay_question(&engine)
        .expect("the next spell has improvise, so the cast asks for artifacts to tap");
    for pendant in &pendants {
        assert!(
            options.contains(pendant),
            "{pendant:?} is an untapped artifact: {options:?}"
        );
    }
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseTargets {
                objects: vec![pendants[0], pendants[1]],
                players: vec![],
            },
        )
        .expect("two artifacts pay the {2}");
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
    let spell = on_stack(&engine, darksteel_pendant()).expect("the Pendant is on the stack");
    assert_eq!(
        engine
            .state()
            .object(spell)
            .unwrap()
            .characteristics()
            .mana_value(),
        2,
        "improvise changes what is paid, not the mana value"
    );
    assert!(is_tapped(&engine, pendants[0]) && is_tapped(&engine, pendants[1]));
    assert!(!is_tapped(&engine, pendants[2]) && !is_tapped(&engine, pendants[3]));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        objects_of(&engine, darksteel_pendant()).len(),
        5,
        "the cast Pendant resolved"
    );

    // The next spell has no improvise: two Islands pay it and nothing asks.
    tap_mana_where(&mut engine, kit::P0, |id| {
        id == islands[1] || id == islands[2]
    });
    cast_with_floating(&mut engine, kit::P0, darksteel_pendant());
    assert!(
        tap_to_pay_question(&engine).is_none(),
        "the grant was spent: {:?}",
        engine.pending()
    );
    assert!(on_stack(&engine, darksteel_pendant()).is_some());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Islands paid the whole {{2}}"
    );
}

/// CR 702.126a: improvise stands in for each *generic* mana of the total
/// cost, so the grant helps with generic mana only (the Whir of Invention rulings of
/// 2017-02-09 say it outright: it cannot pay a coloured symbol). An Ondu
/// Cleric (`{1}{W}`) is cast with the grant: with no white source it cannot be
/// cast however many artifacts are untapped; with a Plains tapped for the
/// `{W}`, one artifact pays the `{1}`.
#[test]
fn archway_of_innovation_improvise_pays_generic_mana_only() {
    // No white anywhere: three artifacts and the grant do not make a Cleric.
    let (mut engine, archway, islands) = the_archway(1, &[], &[ondu_cleric()]);
    tap_mana_where(&mut engine, kit::P0, |id| id == islands[0]);
    grant_improvise(&mut engine, archway);
    let cleric = in_hand(&engine, kit::P0, ondu_cleric()).expect("the Cleric");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&cleric),
        "{{W}} cannot be paid by tapping artifacts"
    );
    assert!(
        engine
            .apply(kit::P0, PlayerAction::CastSpell { card: cleric })
            .is_err(),
        "and the engine refuses the cast"
    );

    // With a Plains: the {W} off the Plains, the {1} off one artifact.
    let (mut engine, archway, islands) = the_archway(1, &[plains()], &[ondu_cleric()]);
    tap_mana_where(&mut engine, kit::P0, |id| id == islands[0]);
    grant_improvise(&mut engine, archway);
    let plains_ids = objects_of(&engine, plains());
    tap_mana_where(&mut engine, kit::P0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, kit::P0, ondu_cleric());
    let (options, _) = tap_to_pay_question(&engine).expect("the {1} may be improvised");
    let pendant = *options.first().expect("an artifact to tap");
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseTargets {
                objects: vec![pendant],
                players: vec![],
            },
        )
        .expect("one artifact pays the {1}");
    assert!(
        on_stack(&engine, ondu_cleric()).is_some(),
        "the Cleric is cast"
    );
    assert!(is_tapped(&engine, pendant));
}

/// "…the next spell you cast **this turn**": the grant ends with the turn.
/// Armed on turn one and not used, it is gone on turn three: a Pendant cast
/// off two Islands then asks no tap question although four artifacts stand
/// untapped. (Twin of the test above, where the same cast on the same turn
/// does.)
#[test]
fn archway_of_innovation_grant_ends_with_the_turn() {
    let (mut engine, archway, islands) = the_archway(3, &[], &[darksteel_pendant()]);
    tap_mana_where(&mut engine, kit::P0, |id| id == islands[0]);
    grant_improvise(&mut engine, archway);

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    assert_eq!(engine.state().turn.number, 3, "player 0's next turn");
    tap_mana_where(&mut engine, kit::P0, |id| {
        id == islands[1] || id == islands[2]
    });
    cast_with_floating(&mut engine, kit::P0, darksteel_pendant());

    assert!(
        tap_to_pay_question(&engine).is_none(),
        "the grant lapsed: {:?}",
        engine.pending()
    );
    assert!(on_stack(&engine, darksteel_pendant()).is_some());
}
