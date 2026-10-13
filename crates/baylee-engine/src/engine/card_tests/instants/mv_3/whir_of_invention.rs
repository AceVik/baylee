//! `cards/instants/mv_3/whir_of_invention.rs`, played.
//!
//! Whir of Invention, `{X}{U}{U}{U}`, instant: "Improvise (Your artifacts can
//! help cast this spell. Each artifact you tap after you're done activating
//! mana abilities pays for {1}.) / Search your library for an artifact card
//! with mana value X or less, put it onto the battlefield, then shuffle."
//!
//! Improvise is asked as a tap question during the cast
//! (`Pending::ChooseTargets` with the convoke prompt) after X is chosen, once
//! the total cost is known, and before it is paid (CR 702.126a, 702.126b).
//! The library is Sol Rings (mana value 1, the kit's quiet artifact); a
//! Worn Powerstone (mana value 3) is put on top of it where the ceiling is
//! tried. Darksteel Pendants (`{2}`, no mana ability) are the artifacts that
//! improvise.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use crate::event::GameEvent;

/// Worn Powerstone: an artifact of mana value 3 for the search to find or
/// leave.
fn worn_powerstone_card() -> CardIndex {
    card_index("b166b670-febc-4821-855e-f8d465644c03")
}

/// Player 0 in their first main phase with `islands` Islands and `pendants`
/// Darksteel Pendants (and `extra`) on the battlefield and Whir of Invention
/// plus `hand` in hand. The library is Sol Rings.
#[track_caller]
fn the_table(
    islands: usize,
    pendants: usize,
    extra: &[CardIndex],
    hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut board = vec![island(); islands];
    board.extend(vec![darksteel_pendant(); pendants]);
    board.extend_from_slice(extra);
    let mut hand_cards = vec![whir_of_invention()];
    hand_cards.extend_from_slice(hand);
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &board)
        .hand(0, &hand_cards)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    engine
}

/// Player 0's permanents printed as `card`.
fn permanents_of(engine: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
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

/// Casts Whir off whatever is floating with `x` chosen, and returns the
/// engine at the next question.
#[track_caller]
fn cast_for(engine: &mut Engine<RegistryLookup>, x: u32) {
    cast_with_floating(engine, kit::P0, whir_of_invention());
    engine
        .apply(kit::P0, PlayerAction::ChooseNumber(x))
        .unwrap_or_else(|e| panic!("X = {x} was refused: {e:?}"));
}

/// The cards the search offers, once the spell has been let resolve.
#[track_caller]
fn the_search_offers(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    options
}

/// Whether `seat`'s library was shuffled after journal entry `since`.
fn shuffled_since(engine: &Engine<RegistryLookup>, seat: PlayerId, since: u64) -> bool {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .skip(usize::try_from(since).unwrap_or(usize::MAX))
        .any(|entry| matches!(entry.event, GameEvent::Shuffled { player, .. } if player == seat))
}

/// Whir of Invention — {X}{U}{U}{U}: "Search your library for an artifact
/// card with mana value X or less, put it onto the battlefield, then
/// shuffle." The third card in the pool to write `Filter::CmcAtMostX`, and
/// the only one of the three that is an **instant** — so that is what this
/// plays, on the opponent's turn, where a sorcery-speed reading would never
/// have offered the spell at all.
///
/// (The first test of the card, kept as it was: its comment and the message
/// of its mana assertion, which spoke of improvise as missing, are rewritten.
/// With no artifact on the table there is nothing to improvise with: four
/// Islands pay `{1}{U}{U}{U}` with nothing left over, the full printed price.)
#[test]
fn whir_of_invention_is_an_instant_and_finds_an_artifact_within_its_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, quiet_artifact())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[whir_of_invention()])
        .start();
    keep_mulligans(&mut engine);

    // The opponent's main phase, with priority back on p0: an instant may be
    // cast here and a sorcery may not, which is the half of this card a mode
    // flag could get wrong. The active player holds priority first (CR
    // 117.3a), so the walk is to the pass after that and not to the phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, crate::turn::Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, which is {{1}}{{U}}{{U}}{{U}} exactly"
    );
    cast_with_floating(&mut engine, p0, whir_of_invention());
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("X = 1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        !options.is_empty(),
        "Sol Ring is mana value 1 and the bound announced was 1"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no artifact stood on the table to improvise with, so the four \
         Islands paid the whole printed cost"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact arrived on the battlefield, on the opponent's turn"
    );
}

/// Improvise: two untapped artifacts pay `{2}` of an X = 2 Whir.
///
/// Three Islands make `{U}{U}{U}`; X = 2 makes the total cost
/// `{2}{U}{U}{U}` (mana value 5 on the stack, since improvise changes what is
/// paid and not the cost, CR 702.126b); the cast asks which artifacts to tap
/// and the two Pendants named pay the `{2}`. Nothing is left floating, both
/// Pendants are tapped, and the spell goes on to find a Sol Ring (mana value
/// 1, within X) and put it onto the battlefield, shuffling.
#[test]
fn whir_of_invention_improvise_lets_two_artifacts_pay_two_generic() {
    let mut engine = the_table(3, 2, &[], &[]);
    let pendants = permanents_of(&engine, darksteel_pendant());
    assert_eq!(pendants.len(), 2, "two Pendants stand on the table");
    tap_all_mana(&mut engine, kit::P0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, and the Pendants make no mana"
    );
    let since = engine.state().journal.last_seq();

    cast_for(&mut engine, 2);
    let (options, max) = tap_to_pay_question(&engine)
        .expect("improvise: the cast asks which artifacts to tap for the generic part");
    assert_eq!(max, 2, "at most the {{2}} that is generic");
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
                objects: pendants.clone(),
                players: vec![],
            },
        )
        .expect("two artifacts pay {2}");

    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the spell is cast: {:?}",
        engine.pending()
    );
    let spell = on_stack(&engine, whir_of_invention()).expect("Whir is on the stack");
    assert_eq!(
        engine
            .state()
            .object(spell)
            .unwrap()
            .characteristics()
            .mana_value(),
        5,
        "improvise does not change the mana value: {{2}}{{U}}{{U}}{{U}}"
    );
    for pendant in &pendants {
        assert!(
            is_tapped(&engine, *pendant),
            "{pendant:?} was tapped to pay"
        );
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{U}}{{U}}{{U}} was paid by the Islands and {{2}} by the artifacts, nothing more"
    );

    let offered = the_search_offers(&mut engine);
    assert!(
        !offered.is_empty(),
        "a Sol Ring, mana value 1, is within X = 2"
    );
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![offered[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, kit::P0));
    assert!(
        on_battlefield(&engine, kit::P0, quiet_artifact()).is_some(),
        "the Sol Ring arrived"
    );
    assert!(
        shuffled_since(&engine, kit::P0, since),
        "\"then shuffle\": the library was shuffled"
    );
}

/// "Each artifact you tap … pays for {1}" — one artifact pays one, and the
/// rest is paid as usual. X = 2 with four Islands and one Pendant: the
/// Pendant pays one of the `{2}`, the fourth Island the other, and the pool is
/// empty after: an artifact did not pay for more than `{1}`.
#[test]
fn whir_of_invention_improvise_pays_one_generic_per_artifact() {
    let mut engine = the_table(4, 1, &[], &[]);
    let pendant = permanents_of(&engine, darksteel_pendant())[0];
    tap_all_mana(&mut engine, kit::P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    cast_for(&mut engine, 2);
    let (options, _) = tap_to_pay_question(&engine)
        .expect("one generic mana of the {2} is still unpaid, and an artifact could pay it");
    assert_eq!(options, vec![pendant]);
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseTargets {
                objects: vec![pendant],
                players: vec![],
            },
        )
        .expect("one artifact");

    assert!(
        on_stack(&engine, whir_of_invention()).is_some(),
        "Whir is cast"
    );
    assert!(is_tapped(&engine, pendant));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Pendant paid {{1}} and four mana paid the other {{4}}: \
         {{2}}{{U}}{{U}}{{U}} is five"
    );
}

/// Ruling (2017-02-09): "Improvise can't pay for {W}, {U}, {B}, {R}, {G}, or
/// {C} mana symbols in a spell's total cost."
///
/// Two Islands and five artifacts cannot cast Whir at any X, because the
/// third `{U}` is not generic; the same artifacts with a third Island can
/// (here X = 3: `{3}{U}{U}{U}` is three Islands and three of the artifacts).
#[test]
fn whir_of_invention_improvise_cannot_pay_a_coloured_symbol() {
    let engine = the_table(2, 5, &[], &[]);
    let whir = in_hand(&engine, kit::P0, whir_of_invention()).expect("Whir");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&whir),
        "two blue sources and five artifacts: the third {{U}} is nobody's to pay"
    );

    // Control: the third Island makes it castable, with the artifacts paying
    // the X.
    let mut engine = the_table(3, 5, &[], &[]);
    tap_all_mana(&mut engine, kit::P0);
    cast_for(&mut engine, 3);
    let (options, _) =
        tap_to_pay_question(&engine).expect("{3} of the total cost is generic: tap artifacts");
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseTargets {
                objects: options.into_iter().take(3).collect(),
                players: vec![],
            },
        )
        .expect("three artifacts pay {3}");
    assert!(
        on_stack(&engine, whir_of_invention()).is_some(),
        "Whir is cast"
    );
}

/// Ruling (2017-02-09): "If an artifact you control has a mana ability with
/// {T} in the cost, activating that ability while casting a spell with
/// improvise will result in the artifact being tapped when you pay the spell's
/// costs. You won't be able to tap it again for improvise."
///
/// Three Islands, a Sol Ring and a Pendant, X = 3 (total `{3}{U}{U}{U}`):
/// everything with a mana ability is tapped first (`UUU` and `CC`, five mana),
/// and the Pendant improvises the sixth. The tap question offers only the
/// Pendant: the Sol Ring is already tapped and is not offered.
#[test]
fn whir_of_invention_cannot_improvise_an_artifact_that_was_tapped_for_mana() {
    let mut engine = the_table(3, 1, &[sol_ring()], &[]);
    let pendant = permanents_of(&engine, darksteel_pendant())[0];
    let ring = permanents_of(&engine, sol_ring())[0];
    tap_all_mana(&mut engine, kit::P0);
    assert!(
        is_tapped(&engine, ring),
        "the Sol Ring was tapped for {{C}}{{C}}"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 5, "UUU and CC");

    cast_for(&mut engine, 3);
    let (options, max) = tap_to_pay_question(&engine)
        .expect("one generic mana is left, and the Pendant could improvise it");
    assert_eq!(
        options,
        vec![pendant],
        "the tapped Sol Ring is not an artifact that can be tapped again"
    );
    assert_eq!(max, 1, "one generic mana of the total cost is unpaid");
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseTargets {
                objects: vec![pendant],
                players: vec![],
            },
        )
        .expect("the Pendant pays the sixth mana");
    assert!(
        on_stack(&engine, whir_of_invention()).is_some(),
        "Whir is cast"
    );
}

/// "Search your library for an artifact card with mana value X or less":
/// nothing above X is offered, and X itself is.
///
/// A Worn Powerstone (mana value 3) is put on top of a library of Sol Rings
/// (mana value 1). With X = 1 the search offers only the Sol Rings, not the
/// Powerstone; with X = 3 (six Islands, no improvise needed) it offers the
/// Powerstone as well, and choosing it puts it onto the battlefield.
#[test]
fn whir_of_invention_search_offers_mana_value_x_or_less() {
    // X = 1: the Powerstone, mana value 3, is above the ceiling.
    let mut engine = the_table(4, 0, &[], &[worn_powerstone_card()]);
    let stone = kit::stack_library(&mut engine, kit::P0, &[worn_powerstone_card()])[0];
    tap_all_mana(&mut engine, kit::P0);
    cast_for(&mut engine, 1);
    let offered = the_search_offers(&mut engine);
    assert!(
        !offered.is_empty() && !offered.contains(&stone),
        "Sol Rings are offered and the mana value 3 Powerstone is not: {offered:?}"
    );
    for id in &offered {
        assert_eq!(
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .map(|c| c.index),
            Some(quiet_artifact()),
            "only Sol Rings within X = 1"
        );
    }

    // X = 3: the Powerstone is exactly at the ceiling.
    let mut engine = the_table(6, 0, &[], &[worn_powerstone_card()]);
    let stone = kit::stack_library(&mut engine, kit::P0, &[worn_powerstone_card()])[0];
    tap_all_mana(&mut engine, kit::P0);
    cast_for(&mut engine, 3);
    let offered = the_search_offers(&mut engine);
    assert!(
        offered.contains(&stone),
        "mana value 3 is within X = 3: {offered:?}"
    );
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![stone],
            },
        )
        .expect("the Powerstone was offered");
    pass_until(&mut engine, |e| at_rest(e, kit::P0));
    assert!(
        on_battlefield(&engine, kit::P0, worn_powerstone_card()).is_some(),
        "the Powerstone arrived on the battlefield"
    );
}

/// "…then shuffle" does not depend on finding anything (CR 701.23b: the
/// searcher need not find a card; the shuffle is its own instruction).
///
/// X = 0 against a library of Sol Rings (mana value 1) finds nothing, puts
/// nothing onto the battlefield, and still shuffles the library.
#[test]
fn whir_of_invention_shuffles_even_when_it_finds_nothing() {
    let mut engine = the_table(3, 0, &[], &[]);
    tap_all_mana(&mut engine, kit::P0);
    let since = engine.state().journal.last_seq();
    cast_for(&mut engine, 0);

    kit::resolve_the_stack(&mut engine, |_, pending| kit::choose_cards(pending, &[]));
    assert!(
        permanents_of(&engine, quiet_artifact()).is_empty(),
        "no card has mana value 0, so nothing arrived"
    );
    assert!(
        shuffled_since(&engine, kit::P0, since),
        "the library is shuffled all the same"
    );
}
