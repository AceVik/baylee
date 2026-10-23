#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// What a scripted spell is aimed at.
#[derive(Clone, Copy, Debug)]
enum At {
    /// Seat 0's permanent with this card.
    Mine(&'static str),
    /// Seat 1's permanent with this card.
    Theirs(&'static str),
    /// Seat 0.
    Me,
    /// Nothing: the spell has no target.
    Nothing,
}

/// A turn of seat 0's with spells on the stack (#226 B). Seat 0 is the
/// agent.
#[derive(Default)]
struct Stack<'a> {
    hand: &'a [&'a str],
    board: &'a [&'a str],
    their_board: &'a [&'a str],
    their_life: Option<i32>,
    /// Seat 0's spells, cast by script first, in order.
    mine: &'a [(&'a str, At)],
    /// Seat 1's spells, cast by script in response, in order.
    theirs: &'a [(&'a str, At)],
}

/// Casts `name` from `seat`'s hand, tapping the lands written down for it,
/// and aims it as `at` says.
fn cast(engine: &mut Engine<RegistryLookup>, seat: PlayerId, name: &str, at: At) {
    let lands: &[&str] = match name {
        "Path to Exile" => &["Plains"],
        "Lightning Bolt" => &["Mountain"],
        "Dualcaster Mage" => &["Mountain", "Mountain", "Mountain"],
        other => panic!("no lands written down for {other}"),
    };
    for land in lands {
        let view = asked_view(engine.state(), seat, 2, engine.pending());
        let source = view
            .battlefield_of(seat)
            .find(|o| o.name == *land && !o.status.contains(baylee_view::ObjectStatus::TAPPED))
            .expect("an untapped land of the script's")
            .id;
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let view = asked_view(engine.state(), seat, 2, engine.pending());
    let card = view
        .hand
        .iter()
        .find(|c| c.card.index == entry(name).card)
        .expect("the script's spell in hand")
        .id;
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the script's mana pays for it");
    if !matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        assert!(matches!(at, At::Nothing), "{name} asked no target");
        return;
    }
    let view = asked_view(engine.state(), seat, 2, engine.pending());
    let find = |side: u8, card: &str| {
        view.battlefield_of(PlayerId::new(side))
            .find(|o| o.card.is_some_and(|c| c.index == entry(card).card))
            .expect("the permanent the script names")
            .id
    };
    let (objects, players) = match at {
        At::Mine(card) => (vec![find(0, card)], vec![]),
        At::Theirs(card) => (vec![find(1, card)], vec![]),
        At::Me => (vec![], vec![PlayerId::new(0)]),
        At::Nothing => panic!("{name} asks for a target the script did not name"),
    };
    engine
        .apply(seat, PlayerAction::ChooseTargets { objects, players })
        .expect("the script's target is legal");
}

/// Plays `table` in seat 0's first main phase: seat 0 casts `mine`, seat 1
/// casts `theirs` in response and passes from then on, and the agent answers
/// everything else until the scripted stack has resolved, or, with nothing
/// scripted, until the main phase ends. Returns the view at the end and
/// every target question the agent answered, each beside the view it was
/// asked in.
fn on_the_stack(
    profile: AIProfile,
    table: &Stack<'_>,
) -> (PlayerView, Vec<(PlayerView, Pending, PlayerAction)>) {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = position(table.hand, table.board);
    let theirs: Vec<DeckEntry> = table.theirs.iter().map(|(name, _)| entry(name)).collect();
    preset.seats[1].starting_hand = Some(theirs);
    preset.seats[1].starting_battlefield = table.their_board.iter().map(|n| entry(n)).collect();
    preset.seats[1].starting_life = table.their_life;
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    settle(&mut engine, &[]);
    for &(name, at) in table.mine {
        cast(&mut engine, me, name, at);
    }
    if !table.theirs.is_empty() {
        engine.apply(me, PlayerAction::PassPriority).unwrap();
        for &(name, at) in table.theirs {
            cast(&mut engine, them, name, at);
        }
        engine.apply(them, PlayerAction::PassPriority).unwrap();
    }
    let scripted = !table.mine.is_empty() || !table.theirs.is_empty();

    let agent = HeuristicAgent::new(profile);
    let mut answered = Vec::new();
    for seq in 3..200 {
        let pending = engine.pending().clone();
        let Some(seat) = pending_player(&pending) else {
            break;
        };
        let view = asked_view(engine.state(), seat, seq, &pending);
        if view.phase != baylee_view::Phase::FirstMain || (scripted && view.stack.is_empty()) {
            break;
        }
        let action = match pending {
            Pending::Priority { .. } if seat == them => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, &pending, &engine.decision_context()),
        };
        if seat == me && matches!(pending, Pending::ChooseTargets { .. }) {
            answered.push((view.clone(), pending.clone(), action.clone()));
        }
        engine
            .apply(seat, action)
            .expect("every planned action is legal");
    }
    (
        asked_view(engine.state(), me, 999, engine.pending()),
        answered,
    )
}

/// The names of `seat`'s creatures.
fn creatures(view: &PlayerView, seat: u8) -> Vec<String> {
    view.battlefield_of(PlayerId::new(seat))
        .filter(|o| o.types.contains(baylee_core::types::TypeSet::CREATURE))
        .map(|o| o.name.clone())
        .collect()
}

/// The names of the cards in the agent's hand.
fn in_hand(view: &PlayerView) -> Vec<&'static str> {
    view.hand
        .iter()
        .filter_map(|c| baylee_cards::by_index(c.card.index))
        .map(baylee_cards_dsl::CardDef::name)
        .collect()
}

/// #226: Misdirection turned the agent's own Path to Exile from Serra Angel
/// onto an Ondu Cleric, and pitched a Brainstorm to do it. A redirect is
/// worth what it saves, and nothing of this seat's is under attack.
#[test]
fn a_misdirection_is_not_spent_on_this_seats_own_spell() {
    for profile in [
        AIProfile::CASUAL,
        AIProfile::STEADY,
        AIProfile::SHARP,
        AIProfile::EXPERT,
    ] {
        let (view, _) = on_the_stack(
            profile,
            &Stack {
                hand: &["Path to Exile", "Misdirection", "Brainstorm"],
                board: &["Plains", "Llanowar Elves"],
                their_board: &["Ondu Cleric", "Serra Angel"],
                ..Stack::default()
            },
        );
        assert_eq!(
            in_hand(&view),
            ["Misdirection", "Brainstorm"],
            "{profile:?}: Misdirection was cast, on the agent's own Path"
        );
        assert_eq!(
            creatures(&view, 1),
            ["Ondu Cleric"],
            "{profile:?}: the Path is on Serra Angel, and stays there"
        );
    }
}

/// #226: a turned Path to Exile went to the first creature across the table,
/// an Ondu Cleric, and Serra Angel stayed. The turned spell is removal, so
/// its new target is the best creature an opponent controls, under either
/// rule: Misdirection's "change the target" (CR 115.7a) and Deflecting
/// Swat's "choose new targets" (CR 115.7d).
#[test]
fn a_turned_removal_spell_goes_to_the_best_creature_across_the_table() {
    for (redirect, lands) in [
        ("Misdirection", &["Island"][..]),
        ("Deflecting Swat", &["Mountain", "Mountain", "Mountain"][..]),
    ] {
        let board: Vec<&str> = ["Llanowar Elves"].iter().chain(lands).copied().collect();
        for profile in [AIProfile::CASUAL, AIProfile::EXPERT] {
            let (view, answered) = on_the_stack(
                profile,
                &Stack {
                    hand: &[redirect, "Brainstorm"],
                    board: &board,
                    their_board: &["Plains", "Ondu Cleric", "Serra Angel"],
                    theirs: &[("Path to Exile", At::Mine("Llanowar Elves"))],
                    ..Stack::default()
                },
            );
            assert_eq!(
                answered.len(),
                2,
                "{profile:?} {redirect}: which spell, and what it becomes"
            );
            assert_eq!(
                creatures(&view, 0),
                ["Llanowar Elves"],
                "{profile:?} {redirect}: the Elves were not saved"
            );
            assert_eq!(
                creatures(&view, 1),
                ["Ondu Cleric"],
                "{profile:?} {redirect}: the Path did not take Serra Angel"
            );
        }
    }
}

/// A Lightning Bolt aimed at this seat is turned onto what it kills: the
/// opponent's face when that is the last 3 life, a creature it can kill
/// otherwise, and never Serra Angel, which it cannot.
#[test]
fn a_turned_bolt_goes_where_it_kills() {
    for (life, face) in [(3, true), (20, false)] {
        let (view, answered) = on_the_stack(
            AIProfile::STEADY,
            &Stack {
                hand: &["Misdirection", "Brainstorm"],
                board: &["Llanowar Elves", "Island"],
                their_board: &["Mountain", "Ondu Cleric", "Serra Angel"],
                their_life: Some(life),
                theirs: &[("Lightning Bolt", At::Me)],
                ..Stack::default()
            },
        );
        let (_, _, turned) = answered.last().expect("the Bolt was turned");
        let PlayerAction::ChooseTargets { objects, players } = turned else {
            panic!("a target answer, not {turned:?}");
        };
        if face {
            assert_eq!(
                (objects.len(), players.as_slice()),
                (0, &[PlayerId::new(1)][..]),
                "at {life} life the Bolt finishes its caster"
            );
            assert_eq!(view.seat(PlayerId::new(1)).map(|s| s.life), Some(0));
        } else {
            assert!(players.is_empty(), "at {life} life a creature dies instead");
            assert_eq!(
                creatures(&view, 1),
                ["Serra Angel"],
                "the Bolt killed the Cleric"
            );
        }
    }
}

/// A copy of an opponent's Path to Exile is this seat's removal: Dualcaster
/// Mage's copy starts aimed at this seat's own Elves (CR 707.10), and is
/// turned (CR 707.10c) onto the best creature across the table.
#[test]
fn a_copied_removal_spell_goes_to_the_best_creature_across_the_table() {
    for profile in [AIProfile::CASUAL, AIProfile::EXPERT] {
        let (view, answered) = on_the_stack(
            profile,
            &Stack {
                hand: &["Dualcaster Mage"],
                board: &["Llanowar Elves", "Mountain", "Mountain", "Mountain"],
                their_board: &["Plains", "Ondu Cleric", "Serra Angel"],
                theirs: &[("Path to Exile", At::Mine("Llanowar Elves"))],
                ..Stack::default()
            },
        );
        assert_eq!(answered.len(), 2, "{profile:?}: which spell, and where");
        assert_eq!(
            creatures(&view, 1),
            ["Ondu Cleric"],
            "{profile:?}: the copy did not take Serra Angel"
        );
    }
}

/// #226: Dualcaster Mage copied its controller's own Path to Exile onto the
/// Ondu Cleric the Path was already aimed at, and the original then had no
/// target. What the original hits is spent; the copy goes elsewhere.
#[test]
fn a_copy_of_this_seats_removal_is_not_aimed_where_the_original_is() {
    for profile in [AIProfile::CASUAL, AIProfile::EXPERT] {
        let (view, answered) = on_the_stack(
            profile,
            &Stack {
                hand: &["Path to Exile", "Dualcaster Mage"],
                board: &["Plains", "Mountain", "Mountain", "Mountain"],
                their_board: &["Ondu Cleric", "Serra Angel"],
                mine: &[
                    ("Path to Exile", At::Theirs("Ondu Cleric")),
                    ("Dualcaster Mage", At::Nothing),
                ],
                ..Stack::default()
            },
        );
        assert_eq!(answered.len(), 2, "{profile:?}: which spell, and where");
        assert!(
            creatures(&view, 1).is_empty(),
            "{profile:?}: one Path missed, leaving {:?}",
            creatures(&view, 1)
        );
    }
}

/// An opponent's Path at the agent's Elves, and the agent holding
/// Misdirection and a Path of its own: the Elves are saved and the two Paths
/// take both creatures across the table, in whichever order the agent casts
/// them. Measured before what this seat's own removal on the stack already
/// hits counted as spent: the expert profile cast its own Path at Serra
/// Angel first, then turned the opponent's onto the Angel too, and the
/// Cleric stayed.
#[test]
fn a_turned_removal_spell_is_not_aimed_where_this_seats_own_already_is() {
    for profile in [
        AIProfile::CASUAL,
        AIProfile::STEADY,
        AIProfile::SHARP,
        AIProfile::EXPERT,
    ] {
        let (view, _) = on_the_stack(
            profile,
            &Stack {
                hand: &["Misdirection", "Path to Exile", "Brainstorm"],
                board: &["Llanowar Elves", "Plains"],
                their_board: &["Plains", "Ondu Cleric", "Serra Angel"],
                theirs: &[("Path to Exile", At::Mine("Llanowar Elves"))],
                ..Stack::default()
            },
        );
        assert_eq!(
            (creatures(&view, 0), creatures(&view, 1)),
            (vec!["Llanowar Elves".to_owned()], vec![]),
            "{profile:?}"
        );
    }
}
