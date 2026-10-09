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
            unpaid_abilities: vec![],
            payable: vec![],
            spell_increases: vec![],
            activation_increases: vec![],
            can_pass: true,
            lands: vec![],
            castable: vec![],
            mana_abilities: vec![],
            abilities: vec![],
            suspendable: vec![],
            granted_actions: vec![],
        }),
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "Your move"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Mine, None, false),
        "Du bist dran"
    );
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Theirs, None, false),
        "You may respond"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Theirs, None, false),
        "Du kannst reagieren"
    );
}

/// "As Phantasmal Terrain enters, choose a basic land type" is not headed
/// "Choose a creature type": the engine offers the five basic land types
/// (CR 205.3i) and the sentence names them. A list with a creature type in
/// it is still the creature question.
#[test]
fn a_choice_of_the_five_basic_land_types_is_headed_as_one() {
    use baylee_core::generated::subtypes::{creature, land};
    let basics = vec![
        land::PLAINS,
        land::ISLAND,
        land::SWAMP,
        land::MOUNTAIN,
        land::FOREST,
    ];
    let headline = |options: Vec<baylee_core::ids::SubtypeId>, lang| {
        interaction(Pending::ChooseSubtype {
            player: me(),
            options,
        })
        .prompt()
        .headline(lang, Turn::Mine, None, false)
    };
    assert_eq!(
        headline(basics.clone(), Lang::En),
        "Choose a basic land type"
    );
    assert_eq!(
        headline(basics.clone(), Lang::De),
        "Wähle einen Standardlandtyp"
    );
    let mut mixed = basics;
    mixed.push(creature::ELF);
    assert_eq!(headline(mixed, Lang::En), "Choose a creature type");
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
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "Choose up to 2 targets"
    );

    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 50,
        reason: baylee_engine::choice::NumberPrompt::X,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "Choose a number (0–50)"
    );

    // Fury's division: which target, of how many, and what is left.
    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 1,
        max: 2,
        reason: baylee_engine::choice::NumberPrompt::DivideDamage {
            target: obj(1),
            index: 0,
            of: 3,
            left: 4,
        },
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "Damage to target 1 of 3, 4 left to divide (1–2)"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Mine, None, false),
        "Schaden an Ziel 1 von 3, noch 4 zu verteilen (1–2)"
    );

    // Banding's division of a creature's combat damage says it is combat
    // damage, and which creature of how many the share is for.
    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 3,
        reason: baylee_engine::choice::NumberPrompt::CombatDamage {
            source: obj(1),
            recipient: obj(2),
            index: 0,
            of: 2,
            left: 3,
        },
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "Combat damage to creature 1 of 2, 3 left to divide (0–3)"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Mine, None, false),
        "Kampfschaden an Kreatur 1 von 2, noch 3 zu verteilen (0–3)"
    );

    // The band question counts attackers joining the band, not cards.
    let i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(2), obj(3)],
        min: 0,
        max: 2,
        prompt: baylee_engine::choice::ChoicePrompt::Band { with: obj(1) },
        total: None,
    });
    let line = i.prompt().headline(Lang::En, Turn::Mine, None, false);
    assert!(line.contains("attackers to join the band"), "{line}");

    // The same question counting replicate payments says so, and what each
    // one costs: "choose a number" over a Lose Focus did not.
    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 2,
        reason: baylee_engine::choice::NumberPrompt::Replicate {
            cost: baylee_core::mana::ManaCost::parse("{U}"),
        },
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "Replicate {U}: pay it how many times? (0–2)"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Mine, None, false),
        "Replikation {U}: wie oft zahlen? (0–2)"
    );

    let i = interaction(Pending::YesNo {
        player: me(),
        prompt: YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
        source: None,
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
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
        waiting.headline(Lang::De, Turn::Theirs, Some(&roster), false),
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
        offer.headline(Lang::En, Turn::Mine, Some(&roster), false),
        "AceVik offers a draw. Accept?"
    );
    assert_eq!(
        offer.headline(Lang::De, Turn::Mine, Some(&roster), false),
        "AceVik bietet ein Remis an. Annehmen?"
    );

    // A frame drawn before `GameStatic` arrives numbers the seat rather
    // than dropping it: the sentence is about a chair that exists either
    // way, and this is what both lines said before there was a roster to
    // ask. The seat the roster does not describe answers the same way.
    assert_eq!(
        waiting.headline(Lang::De, Turn::Theirs, None, false),
        "Warte auf Platz 1"
    );
    assert_eq!(
        offer.headline(Lang::En, Turn::Mine, None, false),
        "Seat 1 offers a draw. Accept?"
    );
}

/// A seat that has kept its opening hand while others still decide theirs
/// (#257) holds no question, and the bar says who it is waiting on.
///
/// Four views of one table of four, seat 0's: still deciding itself (its
/// own question is the sentence, so nothing here), kept with one other
/// seat left (named, as any other wait is), kept with two left (counted),
/// and turn 1 (nothing: `deciding` is empty and whatever is asked then
/// arrives as a question).
#[test]
fn a_seat_that_has_kept_is_told_who_is_still_deciding() {
    let mut roster = crate::test_support::statics(0);
    roster.seats.push(baylee_view::SeatIdentity {
        player: PlayerId::new(2),
        display_name: "AceVik".to_string(),
        is_ai: false,
        away: false,
        team: None,
    });
    let at = |deciding: &[u8]| {
        let mut view = crate::test_support::ViewBuilder::new(4).build();
        view.seat = me();
        view.awaiting = deciding.contains(&me().get()).then_some(me());
        view.deciding = deciding.iter().copied().map(PlayerId::new).collect();
        Prompt::after_keeping(&view)
    };

    assert!(
        at(&[0, 2]).is_none(),
        "this seat is still deciding, and its own question says so"
    );
    let one = at(&[2]).expect("seat 2 is still deciding");
    assert_eq!(
        one.headline(Lang::En, Turn::Mine, Some(&roster), false),
        "Waiting for AceVik"
    );
    assert_eq!(
        one.headline(Lang::De, Turn::Mine, Some(&roster), false),
        "Warte auf AceVik"
    );
    let two = at(&[1, 3]).expect("seats 1 and 3 are still deciding");
    assert_eq!(
        two.headline(Lang::En, Turn::Mine, Some(&roster), false),
        "Waiting for 2 players"
    );
    assert_eq!(
        two.headline(Lang::De, Turn::Mine, Some(&roster), false),
        "Warte auf 2 Spieler"
    );
    assert!(at(&[]).is_none(), "from turn 1 on nobody is deciding");
}

/// The word for a held chair, on the one line that needed it.
///
/// #81 gave a chair the house is holding a dashed rim on its mat and no
/// word anywhere in the interface. `Warte auf AceVik` over a table that is
/// not in fact waiting for that player — the house is answering for them —
/// is the question a player asks and the sentence that would not answer it.
///
/// Both counter-cases are here, because the flag has two neighbours it must
/// not be confused with: the same roster with `away` cleared goes back to
/// naming the player, and an AI chair is not a held one — that seat was
/// always the house and nobody is coming back to it.
#[test]
fn a_held_chair_says_the_house_is_answering_for_it() {
    let mut roster = crate::test_support::statics(0);
    roster.seats.push(baylee_view::SeatIdentity {
        player: PlayerId::new(1),
        display_name: "AceVik".to_string(),
        is_ai: false,
        away: true,
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
        waiting.headline(Lang::En, Turn::Theirs, Some(&roster), false),
        "Waiting for the house — AceVik is away"
    );
    assert_eq!(
        waiting.headline(Lang::De, Turn::Theirs, Some(&roster), false),
        "Warte auf das Haus — AceVik ist abwesend"
    );

    roster.seats[1].away = false;
    assert_eq!(
        waiting.headline(Lang::De, Turn::Theirs, Some(&roster), false),
        "Warte auf AceVik",
        "the line is not turned by the flag"
    );

    roster.seats[1].is_ai = true;
    assert_eq!(
        waiting.headline(Lang::En, Turn::Theirs, Some(&roster), false),
        "Waiting for AceVik",
        "an AI chair was read as a held one"
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
            total: None,
        })
        .prompt()
        .headline(lang, Turn::Mine, None, false)
    };

    assert_eq!(
        line(ChoicePrompt::SearchLibrary, 1, 1, Lang::En),
        "Choose 1 card from your library"
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
    // The two asking costs whose card survives being named, which is the
    // whole reason neither shares the sacrifice's sentence — and why they do
    // not share each other's: a tap wants an untapped permanent (CR 118.3)
    // and Quirion Ranger's Forest is usually tapped when it is returned.
    assert_eq!(
        line(ChoicePrompt::CostTap, 1, 1, Lang::En),
        "Choose 1 untapped permanent to tap"
    );
    assert_eq!(
        line(ChoicePrompt::CostTap, 1, 1, Lang::De),
        "Wähle 1 ungetappte bleibende Karte, die getappt wird"
    );
    assert_eq!(
        line(ChoicePrompt::CostReturn, 1, 1, Lang::En),
        "Choose 1 permanent to return to its owner's hand"
    );
    assert_eq!(
        line(ChoicePrompt::CostReturn, 1, 1, Lang::De),
        "Wähle 1 bleibende Karte, die auf die Hand ihres Besitzers zurückgenommen wird"
    );
    // And the pile the card leaves, which delve's plain noun never had to
    // say: Mines of Moria asks three times, one card at a time.
    assert_eq!(
        line(ChoicePrompt::CostExile, 1, 1, Lang::En),
        "Choose 1 card to exile from your graveyard"
    );
    assert_eq!(
        line(ChoicePrompt::CostExile, 1, 1, Lang::De),
        "Wähle 1 Karte aus deinem Friedhof, die ins Exil geschickt wird"
    );

    // Atraxa asks once per card type, and the type is the question.
    let creature = ChoicePrompt::OneOfType {
        card_type: baylee_core::types::TypeSet::CREATURE,
    };
    assert_eq!(
        line(creature, 0, 1, Lang::En),
        "Put up to one Creature card into your hand"
    );
    assert_eq!(
        line(creature, 0, 1, Lang::De),
        "Nimm bis zu eine Karte vom Typ Kreatur auf deine Hand"
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
            can_take: true,
        },
        Pending::MulliganBottom {
            player: me(),
            count: 1,
        },
        Pending::Priority {
            player: me(),
            legal: Box::new(LegalActions {
                unpaid_abilities: vec![],
                payable: vec![],
                spell_increases: vec![],
                activation_increases: vec![],
                can_pass: true,
                lands: vec![],
                castable: vec![],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
                granted_actions: vec![],
            }),
        },
        attack_choice(vec![obj(1)], vec![seat(1)]),
        Pending::ChooseBlockers {
            demands: Vec::new(),
            player: me(),
            attacker: PlayerId::new(1),
            blockers: vec![],
            capacity: Vec::new(),
            obeying: Vec::new(),
            bounds: Vec::new(),
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
            total: None,
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
            reason: baylee_engine::choice::NumberPrompt::X,
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
        put_back(vec![]),
        Pending::YesNo {
            player: me(),
            prompt: YesNoPrompt::Generic,
            source: None,
        },
    ];
    for pending in all {
        let i = interaction(pending);
        assert!(
            !i.prompt()
                .headline(Lang::En, Turn::Mine, None, false)
                .is_empty()
        );
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
        total: None,
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
        let target_line = targeting.prompt().headline(lang, Turn::Mine, None, false);
        for asking in [&convoke, &delve] {
            let line = asking.prompt().headline(lang, Turn::Mine, None, false);
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

/// The tap-to-help line names no card type, because one question asks it
/// for two keywords that tap different things.
///
/// Convoke taps creatures (CR 702.51a), a waterbend artifacts and creatures
/// (CR 701.67a), and both arrive as `TargetPrompt::Convoke` (#229). The line
/// used to say "creatures or artifacts", which was wrong for convoke once
/// the engine stopped offering it artifacts. And the German is "tappen",
/// the game's word, and never "tippen", which is a finger on the glass.
#[test]
fn the_tap_to_help_line_names_no_card_type() {
    let convoke = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: TargetPrompt::Convoke,
    });
    for lang in [Lang::En, Lang::De] {
        let line = convoke
            .prompt()
            .headline(lang, Turn::Mine, None, false)
            .to_lowercase();
        for kind in ["creature", "artifact", "kreatur", "artefakt"] {
            assert!(!line.contains(kind), "{lang:?} names {kind}: {line}");
        }
        assert!(!line.contains("tippe"), "{lang:?} says tippen: {line}");
    }
}

/// A payment window is a priority window and must not read as one.
///
/// The shape is the whole difficulty: a CR 605.3a window is an ordinary
/// `Pending::Priority` offering mana abilities and nothing else, which is what
/// lets every client draw it and every agent answer it without a new question
/// — and is exactly why neither could tell it from a quiet pass. The house
/// agent found nothing castable, passed, and its own spell was countered.
///
/// Both languages, and both turns: a payment window on an opponent's turn is
/// still a payment window, so `owing` has to beat `Turn` rather than sit
/// inside it.
#[test]
fn a_payment_window_says_what_it_is_instead_of_your_move() {
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            unpaid_abilities: vec![],
            payable: vec![],
            spell_increases: vec![],
            activation_increases: vec![],
            can_pass: true,
            lands: vec![],
            castable: vec![],
            mana_abilities: vec![],
            abilities: vec![],
            suspendable: vec![],
            granted_actions: vec![],
        }),
    });
    for turn in [Turn::Mine, Turn::Theirs] {
        assert_eq!(
            i.prompt().headline(Lang::En, turn, None, true),
            "You owe mana. Activate mana abilities to pay, or pass."
        );
        assert_eq!(
            i.prompt().headline(Lang::De, turn, None, true),
            "Du schuldest Mana. Nutze Manafähigkeiten zum Bezahlen, oder passe."
        );
    }
}

/// And it is the *priority* sentence it replaces, not every sentence.
///
/// `owing` is read off `PlayerView::owed`, which is a fact about the seat and
/// not about the question — so it is live while any prompt is on screen, and a
/// branch written one level too high would put the payment line over a
/// mulligan or a discard. The engine cannot open a payment window inside
/// those, so the only thing this can catch is the branch being in the wrong
/// place; that is the thing worth catching.
#[test]
fn owing_changes_the_priority_line_and_no_other() {
    for pending in [
        Pending::Mulligan {
            player: me(),
            taken: 1,
            next_is_free: false,
            can_take: true,
        },
        Pending::MulliganBottom {
            player: me(),
            count: 2,
        },
        Pending::ChooseNumber {
            player: me(),
            min: 0,
            max: 3,
            reason: baylee_engine::choice::NumberPrompt::X,
        },
    ] {
        let i = interaction(pending);
        let with = i.prompt().headline(Lang::En, Turn::Mine, None, true);
        let without = i.prompt().headline(Lang::En, Turn::Mine, None, false);
        assert_eq!(
            with, without,
            "owing moved a sentence that is not the priority one: {with:?}"
        );
        assert!(!with.is_empty(), "and it must still say something");
    }
}

/// A one-pile ordering says which end of the library the named order runs
/// from, in both languages.
///
/// The cards are named in turn and the first one named is the first listed,
/// so on top it is the new top card and on the bottom it is the card just
/// under what the library already held — the last one named is the bottom
/// card. "Put these in order" alone left a player to guess which of the two
/// the numbers beside the cards meant.
#[test]
fn a_one_pile_ordering_says_which_end_the_first_card_is() {
    let bottom = Pending::Arrange {
        player: me(),
        cards: vec![obj(1), obj(2)],
        piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, 2)],
        prompt: ArrangePrompt::Order,
    };
    let graveyard = Pending::Arrange {
        player: me(),
        cards: vec![obj(1), obj(2)],
        piles: vec![ArrangePile::all_of(ArrangePlace::Graveyard, 2)],
        prompt: ArrangePrompt::Order,
    };
    for (pending, en, de) in [
        (
            put_back(vec![obj(1), obj(2)]),
            "new top card",
            "neue oberste Karte",
        ),
        (bottom, "the bottom card", "unterste Karte"),
        (graveyard, "first card on top", "erste Karte oben"),
    ] {
        let i = interaction(pending);
        let english = i.prompt().headline(Lang::En, Turn::Mine, None, false);
        let german = i.prompt().headline(Lang::De, Turn::Mine, None, false);
        assert!(english.contains(en), "{english}");
        assert!(german.contains(de), "{german}");
    }
}

/// A scry and a surveil name themselves, in the game's own words for them,
/// and each says where its second pile goes — the bottom is still the
/// library, a graveyard is a zone every player reads.
#[test]
fn a_scry_and_a_surveil_say_what_they_are_and_where_the_rest_goes() {
    let look = |prompt, away| Pending::Arrange {
        player: me(),
        cards: vec![obj(1), obj(2)],
        piles: vec![
            ArrangePile::up_to(ArrangePlace::LibraryTop, 2),
            ArrangePile::up_to(away, 2),
        ],
        prompt,
    };
    for (pending, en, de) in [
        (
            look(ArrangePrompt::Scry, ArrangePlace::LibraryBottom),
            ["Scry", "bottom"],
            ["Hellsicht", "unter die Bibliothek"],
        ),
        (
            look(ArrangePrompt::Surveil, ArrangePlace::Graveyard),
            ["Surveil", "graveyard"],
            ["Überwachen", "Friedhof"],
        ),
    ] {
        let i = interaction(pending);
        let english = i.prompt().headline(Lang::En, Turn::Mine, None, false);
        let german = i.prompt().headline(Lang::De, Turn::Mine, None, false);
        assert!(en.iter().all(|w| english.contains(w)), "{english}");
        assert!(de.iter().all(|w| german.contains(w)), "{german}");
    }
}

#[test]
fn life_ward_is_a_localized_explicit_payment_and_only_the_payer_may_answer() {
    let payer = PlayerId::new(1);
    let pending = Pending::YesNo {
        player: payer,
        prompt: YesNoPrompt::PayLife { amount: 7 },
        source: None,
    };
    // Use the wire decoder so the new enum must survive serialization too.
    let pending: Pending = serde_json::from_slice(&serde_json::to_vec(&pending).unwrap()).unwrap();
    let mine = Interaction::new(pending.clone(), payer);
    assert_eq!(mine.answer_yes_no(true), Some(PlayerAction::YesNo(true)));
    assert_eq!(mine.answer_yes_no(false), Some(PlayerAction::YesNo(false)));
    let other = Interaction::new(pending, PlayerId::new(0));
    assert_eq!(other.answer_yes_no(true), None);
    let prompt = Prompt::YesNo {
        question: YesNoPrompt::PayLife { amount: 7 },
    };
    assert_eq!(
        prompt.headline(Lang::En, Turn::Theirs, None, false),
        "Pay 7 life?"
    );
    assert_eq!(
        prompt.headline(Lang::De, Turn::Theirs, None, false),
        "7 Lebenspunkte bezahlen?"
    );
    assert!(!YesNoPrompt::PayLife { amount: 7 }.automatable());
}

#[test]
fn skipping_a_turn_names_the_cost_and_only_the_turn_owner_may_answer() {
    let owner = PlayerId::new(1);
    let question = YesNoPrompt::SkipTurn { source: obj(7) };
    let pending = Pending::YesNo {
        player: owner,
        prompt: question,
        source: None,
    };
    let pending: Pending = serde_json::from_slice(&serde_json::to_vec(&pending).unwrap()).unwrap();
    let mine = Interaction::new(pending.clone(), owner);
    assert_eq!(mine.answer_yes_no(true), Some(PlayerAction::YesNo(true)));
    assert_eq!(mine.answer_yes_no(false), Some(PlayerAction::YesNo(false)));
    let other = Interaction::new(pending, PlayerId::new(0));
    assert_eq!(other.answer_yes_no(true), None);
    assert_eq!(other.answer_yes_no(false), None);
    assert_eq!(
        mine.prompt().headline(Lang::En, Turn::Mine, None, false),
        "Skip this turn to untap this permanent?"
    );
    assert_eq!(
        mine.prompt().headline(Lang::De, Turn::Mine, None, false),
        "Diesen Zug überspringen, um diese bleibende Karte zu enttappen?"
    );
    assert!(!question.automatable());
}

#[test]
fn pact_question_names_the_colored_cost_and_warns_about_losing() {
    let i = interaction(Pending::YesNo {
        player: me(),
        prompt: baylee_engine::choice::YesNoPrompt::PayPact {
            cost: baylee_core::mana::ManaCost::parse("{3}{U}{U}"),
        },
        source: None,
    });
    for (lang, warning) in [
        (Lang::En, "lose the game"),
        (Lang::De, "verlierst du das Spiel"),
    ] {
        let text = i.prompt().headline(lang, Turn::Mine, None, false);
        assert!(text.contains("{3}{U}{U}"), "{text}");
        assert!(text.contains(warning), "{text}");
    }
}

/// Conduit of Worlds' offer: the line says the card is cast paying its mana
/// cost and that the mana is made first, because a yes opens a payment
/// window rather than casting at once.
/// Crew (CR 702.122a) is answered with any number of creatures, and what
/// decides is their total power — so the line names the total, not a count.
#[test]
fn crew_question_names_the_total_power() {
    let i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2), obj(3)],
        min: 1,
        max: 3,
        prompt: ChoicePrompt::CostCrew { power: 2 },
        total: None,
    });
    for (lang, crew, total) in [
        (Lang::En, "Crew 2", "total power 2 or more"),
        (Lang::De, "Besatzung 2", "Gesamtstärke von 2 oder mehr"),
    ] {
        let text = i.prompt().headline(lang, Turn::Mine, None, false);
        assert!(text.contains(crew), "{text}");
        assert!(text.contains(total), "{text}");
    }
}

#[test]
fn cast_paying_question_says_the_mana_is_made_first() {
    let i = interaction(Pending::YesNo {
        player: me(),
        prompt: baylee_engine::choice::YesNoPrompt::CastPaying { card: obj(7) },
        source: None,
    });
    for (lang, cost, first) in [
        (Lang::En, "paying its mana cost", "make the mana first"),
        (Lang::De, "Manakosten bezahlen", "zuerst das Mana"),
    ] {
        let text = i.prompt().headline(lang, Turn::Mine, None, false);
        assert!(text.contains(cost), "{text}");
        assert!(text.contains(first), "{text}");
    }
}

/// The band and combat damage questions name their creatures where the
/// renderer can: a double block by two Bears is two shares (CR 510.1c), and
/// "creature 1 of 2" does not say which Bear is first.
#[test]
fn division_and_band_questions_name_their_creatures() {
    let i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(2), obj(3)],
        min: 0,
        max: 2,
        prompt: baylee_engine::choice::ChoicePrompt::Band { with: obj(1) },
        total: None,
    });
    let names = |id: ObjectId| {
        [(obj(1), "Craw Wurm"), (obj(2), "Grizzly Bears")]
            .into_iter()
            .find(|(o, _)| *o == id)
            .map(|(_, n)| n.to_string())
    };
    let line = i
        .prompt()
        .headline_naming(Lang::En, Turn::Mine, None, false, &names);
    assert_eq!(line, "Choose up to 2 attackers to band with Craw Wurm");
    let line = i
        .prompt()
        .headline_naming(Lang::De, Turn::Mine, None, false, &names);
    assert_eq!(
        line,
        "Wähle bis zu 2 Angreifer, die mit Craw Wurm eine Gruppe bilden"
    );
    let i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 6,
        reason: baylee_engine::choice::NumberPrompt::CombatDamage {
            source: obj(1),
            recipient: obj(2),
            index: 0,
            of: 2,
            left: 6,
        },
    });
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::En, Turn::Mine, None, false, &names),
        "Combat damage from Craw Wurm to Grizzly Bears (1 of 2), 6 left to divide (0–6)"
    );
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::De, Turn::Mine, None, false, &names),
        "Kampfschaden von Craw Wurm an Grizzly Bears (1 von 2), noch 6 zu verteilen (0–6)"
    );
    // A creature the renderer cannot name keeps the count.
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::En, Turn::Mine, None, false, &|_| None),
        "Combat damage to creature 1 of 2, 6 left to divide (0–6)"
    );
}

#[test]
fn counter_amount_names_the_counter_and_recipient_in_both_languages() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 4,
        reason: baylee_engine::choice::NumberPrompt::Counters {
            target: obj(1),
            kind: baylee_engine::object::CounterKind::Plus {
                power: 1,
                toughness: 0,
            },
        },
    });
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "How many +1/+0 counters? (0–4)"
    );
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::En, Turn::Mine, None, false, &|_| Some(
                "Clockwork Beast".into()
            )),
        "How many +1/+0 counters on Clockwork Beast? (0–4)"
    );
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::De, Turn::Mine, None, false, &|_| Some(
                "Uhrwerkbestie".into()
            )),
        "Wie viele +1/+0-Marken auf Uhrwerkbestie? (0–4)"
    );
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(0)));
    assert_eq!(i.set_number(u32::MAX), 4);
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(4)));
}

#[test]
fn retarget_explains_when_an_empty_answer_preserves_the_existing_target() {
    for min in [0, 1] {
        let mut i = interaction(Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1)],
            player_options: vec![],
            min,
            max: 1,
            reason: TargetPrompt::Retarget {
                current: crate::test_support::target(obj(9)),
                index: 1,
                of: 3,
            },
        });
        for lang in [Lang::En, Lang::De] {
            let phrase = if min == 0 {
                Phrase::ChooseNewTargetOrKeep
            } else {
                Phrase::ChooseNewTarget
            };
            assert_eq!(
                i.prompt().headline_naming_targets(
                    lang,
                    Turn::Mine,
                    None,
                    false,
                    &|_| None,
                    &|_| Some("Goblin".into())
                ),
                format!(
                    "{} {}",
                    if lang == Lang::En {
                        "Target 2 of 3: Goblin."
                    } else {
                        "Ziel 2 von 3: Goblin."
                    },
                    phrase.text(lang)
                )
            );
        }
        assert_eq!(i.can_confirm(), min == 0);
        if min == 0 {
            assert_eq!(
                i.confirm(),
                Some(PlayerAction::ChooseObjects { objects: vec![] })
            );
        }
        i.toggle(obj(1));
        assert_eq!(
            i.confirm(),
            Some(PlayerAction::ChooseObjects {
                objects: vec![obj(1)],
            })
        );
    }
}

#[test]
fn retarget_names_the_current_player_and_sends_a_selected_player() {
    let other = PlayerId::new(1);
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![other],
        min: 0,
        max: 1,
        reason: TargetPrompt::Retarget {
            current: baylee_engine::choice::TargetRef::Player(me()),
            index: 0,
            of: 1,
        },
    });
    for lang in [Lang::En, Lang::De] {
        let expected = format!(
            "{}: {}.",
            if lang == Lang::En {
                "Target 1 of 1"
            } else {
                "Ziel 1 von 1"
            },
            crate::i18n::seat_name(lang, None, me()),
        );
        assert!(
            i.prompt()
                .headline(lang, Turn::Mine, None, false)
                .starts_with(&expected)
        );
    }
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] })
    );
    i.toggle_player(other);
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![other],
        })
    );
}

#[test]
fn optional_payment_keeps_zero_and_overpayment_reachable() {
    let mut i = interaction(Pending::ChooseNumber {
        player: me(),
        min: 0,
        max: 80,
        reason: baylee_engine::choice::NumberPrompt::ManaPayment {
            preventable_damage: 2,
        },
    });
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(0)));
    assert_eq!(i.set_number(80), 80);
    assert_eq!(i.confirm(), Some(PlayerAction::ChooseNumber(80)));
    assert_eq!(
        i.prompt().headline(Lang::En, Turn::Mine, None, false),
        "How much mana will you pay? (0–80; prevent up to 2 damage)"
    );
    assert_eq!(
        i.prompt().headline(Lang::De, Turn::Mine, None, false),
        "Wie viel Mana zahlen? (0–80; bis zu 2 Schaden verhindern)"
    );
}

#[test]
fn an_effects_sacrifice_choice_names_the_player_who_loses_the_cards() {
    let i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1)],
        min: 1,
        max: 1,
        prompt: ChoicePrompt::SacrificeFor {
            player: PlayerId::new(1),
        },
        total: None,
    });
    assert!(
        i.prompt()
            .headline(Lang::En, Turn::Mine, None, false)
            .starts_with("Seat 1 sacrifices the chosen permanents.")
    );
    assert!(
        i.prompt()
            .headline(Lang::De, Turn::Mine, None, false)
            .starts_with("Platz 1 opfert die gewählten bleibenden Karten.")
    );
}

#[test]
fn large_target_counts_survive_selection_and_the_display_boundary() {
    let options: Vec<_> = (1..=300).map(obj).collect();
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: options.clone(),
        player_options: vec![],
        min: 300,
        max: 300,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(i.bounds(), Some((300, 300)));
    assert!(
        i.prompt()
            .headline(Lang::En, Turn::Mine, None, false)
            .contains("300")
    );
    for &object in &options[..299] {
        i.toggle(object);
    }
    assert!(!i.can_confirm());
    i.toggle(options[299]);
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects { objects: options })
    );
    let retarget = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: TargetPrompt::Retarget {
            current: crate::test_support::target(obj(2)),
            index: 69_999,
            of: 70_000,
        },
    });
    assert!(
        retarget
            .prompt()
            .headline(Lang::En, Turn::Mine, None, false)
            .starts_with("Target 70000 of 70000:")
    );
}

#[test]
fn retarget_identity_and_names_include_the_original_incarnation() {
    let question = |version| Pending::ChooseTargets {
        player: me(),
        options: vec![obj(9)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: TargetPrompt::Retarget {
            current: baylee_engine::choice::TargetRef::Object(baylee_core::ids::DamageSourceRef {
                object: obj(9),
                version,
            }),
            index: 0,
            of: 1,
        },
    };
    let mut first = interaction(question(1));
    let name = first.prompt().headline_naming_targets(
        Lang::En,
        Turn::Mine,
        None,
        false,
        &|_| panic!("current-object names must not label historical targets"),
        &|source| {
            assert_eq!(source.version, 1);
            Some("Old Bears (earlier incarnation)".into())
        },
    );
    assert!(name.contains("Old Bears (earlier incarnation)"));
    let missing = first
        .prompt()
        .headline_naming(Lang::En, Turn::Mine, None, false, &|_| {
            Some("New secret identity".into())
        });
    assert!(!missing.contains("secret"));
    first.toggle(obj(9));
    let next = Interaction::new_keeping(question(3), me(), Some(&first));
    assert_ne!(first.decision_id(), next.decision_id());
    assert_eq!(
        next.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] })
    );
}

/// Raging River asks the attacking player once per attacker, and two of the
/// questions are word for word the same unless the line says which attacker
/// it is about — played live, with two Grizzly Bears on the table the
/// unnamed line read as a question about the pointer's card. Named in both
/// languages; a renderer that cannot name it keeps the old line.
#[test]
fn a_river_label_names_the_attacker_it_is_for() {
    let i = interaction(Pending::ChoosePile {
        player: me(),
        piles: vec![vec![obj(5)], vec![obj(6), obj(7)]],
        label: Some(obj(2)),
    });
    let names = |id: ObjectId| (id == obj(2)).then(|| "Hill Giant".to_string());
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::En, Turn::Mine, None, false, &names),
        "Hill Giant: choose \"left\" or \"right\"; only that pile and fliers may block it"
    );
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::De, Turn::Mine, None, false, &names),
        "Hill Giant: wähle \"links\" oder \"rechts\"; nur dieser Stapel und Flieger dürfen blocken"
    );
    assert_eq!(
        i.prompt()
            .headline_naming(Lang::En, Turn::Mine, None, false, &|_| None),
        "Choose \"left\" (first) or \"right\": only that pile and fliers may block it"
    );
    // Fact or Fiction's pile is not a label, and says nothing of blocking.
    let i = interaction(Pending::ChoosePile {
        player: me(),
        piles: vec![vec![obj(5)], vec![obj(6)]],
        label: None,
    });
    assert!(
        !i.prompt()
            .headline_naming(Lang::En, Turn::Mine, None, false, &names)
            .contains("Hill Giant")
    );
}
