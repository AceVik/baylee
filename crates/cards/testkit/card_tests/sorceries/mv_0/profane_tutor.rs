//! `cards/sorceries/mv_0/profane_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Suspend is a card in exile with time counters on it (CR 702.62a,
/// 702.62b), and what the countdown casts is that card. Profane Tutor is
/// suspended, cast as its last counter comes off, and resolves into the
/// graveyard. Bojuka Bog then exiles that graveyard. The Tutor in exile now
/// was exiled by the Bog and is not suspended, so no upkeep casts it.
///
/// The suspend mark rode along with the card through the stack and the
/// graveyard, and the countdown asks only for the mark: with no time counter
/// left it counted the Tutor down from nothing and cast it for free again
/// at the next upkeep (CR 400.7).
#[test]
#[allow(clippy::too_many_lines)] // a suspend, its countdown, a Bog and one more upkeep, told in order
fn a_suspended_card_that_resolved_and_was_exiled_again_is_not_cast_again() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(71, island())
        .hand(0, &[profane_tutor(), bojuka_bog()])
        .battlefield(0, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tutor = in_hand(&engine, p0, profane_tutor()).expect("in hand");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.suspendable.contains(&tutor),
        "{:?}",
        legal.suspendable
    );
    engine
        .apply(p0, PlayerAction::Suspend { card: tutor })
        .expect("the suspend cost is paid");
    assert_eq!(
        engine.state().object(tutor).map(|o| o.zone),
        Some(Zone::Exile)
    );

    // Two of p0's upkeeps take the two counters off, and the last one casts
    // it: the search is the Tutor resolving.
    let searching = |e: &Engine<RegistryLookup>| {
        matches!(e.pending(), Pending::ChooseCards { player, prompt, .. }
            if *player == p0 && *prompt != ChoicePrompt::LeaveTapped)
    };
    for _ in 0..6 {
        if searching(&engine) {
            break;
        }
        pass_until(&mut engine, |e| {
            searching(e) || e.state().turn.step == Step::Cleanup
        });
        pass_until(&mut engine, |e| {
            searching(e) || e.state().turn.step != Step::Cleanup
        });
    }
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the Tutor was never cast: {:?}", engine.pending())
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(tutor).map(|o| o.zone),
        Some(Zone::Graveyard),
        "cast from suspend and resolved"
    );

    // Bojuka Bog, at p0's own graveyard.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let bog = in_hand(&engine, p0, bojuka_bog()).expect("the Bog is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: bog })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let exiled = engine.state().object(tutor).expect("the Tutor");
    assert_eq!(exiled.zone, Zone::Exile, "exiled by the Bog");
    assert_eq!(
        exiled.counters.get(CounterKind::Time),
        0,
        "with no time counter on it"
    );

    // p0's next turn begins, upkeep and all, and nothing is cast.
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        searching(e)
            || (matches!(e.state().turn.phase, Phase::FirstMain)
                && e.state().turn.active == p0
                && e.state().turn.number > turn)
    });
    assert!(
        !searching(&engine),
        "the Tutor the Bog exiled was cast for free at the next upkeep"
    );
    assert_eq!(
        engine.state().object(tutor).map(|o| o.zone),
        Some(Zone::Exile)
    );
}

/// A card whose mana cost does not carry its colour has it from its colour
/// indicator (CR 202.2e), in every zone: Profane Tutor (black), Ancestral
/// Vision (blue), Pact of Negation (blue, cost {0}) and Dryad Arbor (green)
/// were colourless in hand until the field was written on their faces.
#[test]
fn cards_coloured_beyond_their_cost_have_that_colour_in_hand() {
    use baylee_core::color::{Color, ColorSet};
    let p0 = PlayerId::new(0);
    let rows = [
        (profane_tutor(), Color::Black),
        (ancestral_vision_card(), Color::Blue),
        (
            card_index("f3e213a4-ba5a-468a-93b3-c0a34e1bd725"),
            Color::Blue,
        ),
        (
            card_index("e996cd67-739c-40f4-b276-0042acf26c71"),
            Color::Green,
        ),
    ];
    let hand: Vec<CardIndex> = rows.iter().map(|(card, _)| *card).collect();
    let engine = Duel::new(4402, island()).hand(0, &hand).start();
    for (card, color) in rows {
        let id = in_hand(&engine, p0, card).expect("in hand");
        assert_eq!(
            engine.state().object(id).unwrap().characteristics().colors,
            ColorSet::from_slice(&[color]),
            "{card:?}"
        );
    }
}
