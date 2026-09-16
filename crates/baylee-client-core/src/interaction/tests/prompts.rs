//! The sentence a player reads over the buttons, in every language: `Prompt::headline`, the `Turn` it takes off the active seat, and the two lines about another chair that have to name a seat instead of printing a number. A priority window on somebody else's turn must not say it is your turn, four card choices read as four different decisions rather than four copies of "Choose 1 card(s)", helping to pay a cost is never worded as targeting, and every `Pending` variant produces a non-empty headline - the guard that makes a new engine choice fail here instead of at the table. What the player may then do about that sentence is asserted in the other files.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A priority window on somebody else's turn does not say it is yours.
///
/// The same `Pending`, the same buttons under it, two sentences — and
/// the German is where it was worst: "Du bist dran" over Pass and Skip
/// says *it is your turn* in a way "Your move" only implies. Both
/// languages are asserted because a phrase that reads right in one and
/// wrong in the other is exactly what `Phrase` exists to stop.
#[test]
fn the_bar_says_whose_turn_it_is_over_the_same_two_buttons() {
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![],
            castable: vec![],
            mana_abilities: vec![],
            abilities: vec![],
            suspendable: vec![],
        }),
    });
    assert_eq!(i.prompt().headline(Lang::En, Turn::Mine, None), "Your move");
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Mine, None),
        "Du bist dran"
    );
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Theirs, None),
        "You may respond"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Theirs, None),
        "Du kannst reagieren"
    );
}

/// And `Turn` is read off the seat, not guessed at.
#[test]
fn a_turn_belongs_to_the_seat_that_is_active() {
    assert_eq!(Turn::of(me(), me()), Turn::Mine);
    assert_eq!(Turn::of(PlayerId::new(1), me()), Turn::Theirs);
}

#[test]
fn prompt_headlines_are_written_for_a_player_not_a_developer() {
    let i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 0,
        max: 2,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None),
        "Choose up to 2 targets"
    );

    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 50,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None),
        "Choose a number (0–50)"
    );

    let i = interaction(Pending::YesNo {
        player: me(),
        prompt: YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
        source: None,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None),
        "Pay 2 life? Otherwise it enters tapped"
    );
}

/// Two lines in this `match` are about **another chair**, and both had
/// the seat in hand and printed a number. A draw offer is the one that
/// matters: at a table of four, "a draw was offered" is not a question
/// anybody can answer.
#[test]
fn the_two_lines_about_another_seat_say_whose_seat_it_is() {
    let mut roster = crate::test_support::statics(0);
    roster.seats.push(baylee_view::SeatIdentity {
        player: PlayerId::new(1),
        display_name: "AceVik".to_string(),
        is_ai: false,
        away: false,
        team: None,
    });

    let waiting = interaction(Pending::ChooseTargets {
        player: PlayerId::new(1),
        options: vec![obj(1)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    })
    .prompt();
    assert_eq!(
        waiting.headline(Lang::De, Turn::Theirs, Some(&roster)),
        "Warte auf AceVik"
    );

    let offer = interaction(Pending::YesNo {
        player: me(),
        prompt: YesNoPrompt::DrawOffer {
            proposer: PlayerId::new(1),
        },
        source: None,
    })
    .prompt();
    assert_eq!(
        offer.headline(Lang::En, Turn::Mine, Some(&roster)),
        "AceVik offers a draw. Accept?"
    );
    assert_eq!(
        offer.headline(Lang::De, Turn::Mine, Some(&roster)),
        "AceVik bietet ein Remis an. Annehmen?"
    );

    // A frame drawn before `GameStatic` arrives numbers the seat rather
    // than dropping it: the sentence is about a chair that exists either
    // way, and this is what both lines said before there was a roster to
    // ask. The seat the roster does not describe answers the same way.
    assert_eq!(
        waiting.headline(Lang::De, Turn::Theirs, None),
        "Warte auf Platz 1"
    );
    assert_eq!(
        offer.headline(Lang::En, Turn::Mine, None),
        "Seat 1 offers a draw. Accept?"
    );
}

/// The backlog's own check, as a test: four card choices, and each line
/// has to be placeable without knowing which card asked it. Before this,
/// all four read "Choose 1 card(s)".
#[test]
fn four_card_choices_read_as_four_different_decisions() {
    let line = |reason, min, max, lang| {
        interaction(Pending::ChooseCards {
            player: me(),
            options: vec![obj(1), obj(2), obj(3)],
            min,
            max,
            prompt: reason,
        })
        .prompt()
        .headline(lang, Turn::Mine, None)
    };

    assert_eq!(
        line(ChoicePrompt::SearchLibrary, 1, 1, Lang::En),
        "Choose 1 card from your library"
    );
    assert_eq!(
        line(ChoicePrompt::ScryBottom, 0, 2, Lang::De),
        "Wähle bis zu 2 Karten, die nach unten gehen"
    );
    assert_eq!(
        line(ChoicePrompt::PutBackOnTop, 1, 1, Lang::De),
        "Wähle 1 Karte, die oben auf deine Bibliothek kommt"
    );
    assert_eq!(
        line(ChoicePrompt::Wish, 1, 3, Lang::En),
        "Choose 1–3 cards from outside the game"
    );
    // Delve is answered a line earlier: it is part of a cost, not a
    // selection, and that arm must not be shadowed by the reason noun.
    assert_eq!(
        line(ChoicePrompt::Delve, 0, 4, Lang::En),
        "Exile cards from your graveyard to help pay — each pays for one"
    );
    // The other two costs are the opposite case: one named card, and the
    // player has to be told what happens to it. Both languages, because the
    // German is a relative clause and agrees with the number.
    assert_eq!(
        line(ChoicePrompt::CostSacrifice, 1, 1, Lang::En),
        "Choose 1 permanent to sacrifice"
    );
    assert_eq!(
        line(ChoicePrompt::CostSacrifice, 1, 1, Lang::De),
        "Wähle 1 bleibende Karte, die geopfert wird"
    );
    assert_eq!(
        line(ChoicePrompt::CostDiscard, 1, 1, Lang::En),
        "Choose 1 card to discard"
    );
    assert_eq!(
        line(ChoicePrompt::CostDiscard, 2, 2, Lang::De),
        "Wähle 2 Karten, die abgeworfen werden"
    );

    // And the whole of AS's second half: one card is never "card(s)".
    for lang in Lang::ALL {
        let one = line(ChoicePrompt::Generic, 1, 1, lang);
        assert!(!one.contains("(s)") && !one.contains("(n)"), "{one}");
    }
    assert_eq!(line(ChoicePrompt::Generic, 1, 1, Lang::De), "Wähle 1 Karte");
    assert_eq!(
        line(ChoicePrompt::Generic, 2, 2, Lang::De),
        "Wähle 2 Karten"
    );
}

#[test]
fn every_pending_variant_produces_a_prompt_without_panicking() {
    // A completeness guard: adding a choice to the engine without teaching
    // the client about it should fail here rather than at the table.
    let all = vec![
        Pending::Mulligan {
            player: me(),
            taken: 0,
            next_is_free: true,
        },
        Pending::MulliganBottom {
            player: me(),
            count: 1,
        },
        Pending::Priority {
            player: me(),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![],
                castable: vec![],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
            }),
        },
        attack_choice(vec![obj(1)], vec![seat(1)]),
        Pending::ChooseBlockers {
            player: me(),
            attacker: PlayerId::new(1),
            blockers: vec![],
        },
        Pending::DiscardChoice {
            player: me(),
            count: 1,
        },
        Pending::LegendChoice {
            player: me(),
            options: vec![obj(1), obj(2)],
        },
        Pending::ChooseCards {
            player: me(),
            options: vec![],
            min: 0,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        Pending::ChooseTargets {
            player: me(),
            options: vec![],
            player_options: vec![],
            min: 0,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        Pending::ChooseSubtype {
            player: me(),
            options: vec![],
        },
        Pending::ChooseColor {
            player: me(),
            options: vec![ManaColor::White],
        },
        Pending::ChooseNumber {
            player: me(),
            min: 0,
            max: 1,
        },
        Pending::ChoosePlayer {
            player: me(),
            options: vec![me()],
        },
        Pending::ChooseCastMode {
            player: me(),
            object: obj(1),
            options: vec![],
        },
        Pending::OrderObjects {
            player: me(),
            objects: vec![],
        },
        Pending::YesNo {
            player: me(),
            prompt: YesNoPrompt::Generic,
            source: None,
        },
    ];
    for pending in all {
        let i = interaction(pending);
        assert!(!i.prompt().headline(Lang::En, Turn::Mine, None).is_empty());
    }
}

/// Convoke and delve say what they are, in both languages.
///
/// Reported from a live game: a waterbend spell "wollte von mir 99
/// Targets". Both halves of that were true — the engine published the
/// question in the targeting variant with a sentinel bound — and both are
/// fixed at the source. What is pinned here is the half a player reads:
/// the line must not be the "choose N targets" one, because tapping your
/// own creatures to help pay is not choosing a target for anything.
#[test]
fn helping_to_pay_is_not_asked_for_as_targeting() {
    let convoke = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 0,
        max: 2,
        reason: TargetPrompt::Convoke,
    });
    let delve = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1)],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::Delve,
    });
    let targeting = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 0,
        max: 2,
        reason: TargetPrompt::Targets,
    });
    for lang in [Lang::En, Lang::De] {
        let target_line = targeting.prompt().headline(lang, Turn::Mine, None);
        for asking in [&convoke, &delve] {
            let line = asking.prompt().headline(lang, Turn::Mine, None);
            assert!(!line.is_empty());
            assert_ne!(
                line, target_line,
                "helping to pay was asked for as targeting in {lang:?}"
            );
        }
    }
    // And the selection itself is unchanged: it is still a bounded pick
    // over the offered permanents, answerable with none.
    assert!(convoke.confirm().is_some(), "convoke may be declined");
}
