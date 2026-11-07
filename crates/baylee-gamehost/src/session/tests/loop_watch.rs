//! What a seat cannot turn into an endless loop by repeating itself.
//!
//! The engine calls a situation that keeps coming back a loop, and a loop is
//! broken or drawn. A socket can say the same thing thousands of times inside
//! one decision clock, so everything a player may repeat at will has to be
//! left out of that count: a setting, a cast taken back, a draw offered and
//! declined. None of them is the game repeating itself (CR 733.1 takes a cast
//! back as if it never happened), and none may end a game or cost anybody a
//! trigger.

use super::*;
use baylee_engine::choice::PriorityHold;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// More frames than the watch took to draw a game before these were left out
/// of it (9 211 with one spell on the stack).
const FRAMES: u32 = 12_000;

fn card(name: &str) -> CardIndex {
    baylee_cards::generated::ALL
        .iter()
        .find(|(_, card)| card.name() == name)
        .unwrap_or_else(|| panic!("the pool has {name}"))
        .1
        .index
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

/// Two players on seat 0's first main phase, each holding `hand` and with
/// `board` in play.
fn table(hand: &[&str], board: &[&str]) -> Session {
    let of = |names: &[&str]| {
        names
            .iter()
            .map(|name| entry(card(name)))
            .collect::<Vec<_>>()
    };
    let mut preset = two_humans();
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.starting_hand = Some(of(hand));
        seat.starting_battlefield = of(board);
    }
    let mut session = Session::new(&preset).expect("session builds");
    session.pump();
    for seat in [ME, THEM] {
        session.act(seat, PlayerAction::MulliganKeep).expect("kept");
    }
    for _ in 0..200 {
        let main = format!("{:?}", session.state().turn.step) == "Main";
        match session.pending().clone() {
            Pending::Priority { player, .. } if main && player == ME => return session,
            Pending::Priority { player, .. } => {
                session
                    .act(player, PlayerAction::PassPriority)
                    .expect("passed");
            }
            other => panic!("on the way to the main phase: {other:?}"),
        }
    }
    panic!("never reached the main phase");
}

/// Taps a land of seat 0's, which holds priority.
fn tap_a_land(session: &mut Session) {
    let Pending::Priority { legal, .. } = session.pending().clone() else {
        panic!("{:?}", session.pending());
    };
    session
        .act(
            ME,
            PlayerAction::ActivateManaAbility {
                source: legal.mana_abilities[0],
            },
        )
        .expect("tapped");
}

/// Seat 0 casts the first card it can at seat 1, as far as the target.
fn cast_at_them(session: &mut Session) {
    tap_a_land(session);
    let Pending::Priority { legal, .. } = session.pending().clone() else {
        panic!("{:?}", session.pending());
    };
    session
        .act(
            ME,
            PlayerAction::CastSpell {
                card: legal.castable[0],
            },
        )
        .expect("cast");
    session
        .act(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![THEM],
            },
        )
        .expect("targeted");
}

/// Neither seat has ended the game, and no loop was seen.
fn no_loop(session: &Session, by: &str) {
    assert!(
        !matches!(session.pending(), Pending::GameOver(_)),
        "{by} ended the game: {:?}",
        session.pending()
    );
    assert_eq!(session.engine.loops_broken(), 0, "{by} was called a loop");
}

/// The Bolt still on the stack resolves when both pass: nothing was lost.
fn the_spell_resolves(session: &mut Session) {
    let before = session.state().players[1].life;
    for seat in [ME, THEM] {
        session
            .act(seat, PlayerAction::PassPriority)
            .expect("passed");
    }
    assert_eq!(session.state().players[1].life, before - 3);
}

/// A setting is taken from any seat at any time and moves no clock, so it is
/// the cheapest thing to repeat; the seat that holds nothing up included.
#[test]
fn a_setting_said_again_and_again_is_no_loop() {
    for seat in [ME, THEM] {
        let mut session = table(&["Lightning Bolt"], &["Mountain"]);
        cast_at_them(&mut session);
        for _ in 0..FRAMES {
            session
                .act(seat, PlayerAction::SetPriorityHold(PriorityHold::Always))
                .expect("a setting is taken");
        }
        no_loop(&session, "a setting");
        the_spell_resolves(&mut session);
    }
}

/// A cast taken back leaves the game as it was (CR 733.1), so a seat with
/// priority can do it for as long as it likes without anything happening.
#[test]
fn a_cast_taken_back_again_and_again_is_no_loop() {
    let mut session = table(
        &["Lightning Bolt", "Lightning Bolt"],
        &["Mountain", "Mountain"],
    );
    cast_at_them(&mut session);
    tap_a_land(&mut session);
    for _ in 0..FRAMES / 2 {
        let Pending::Priority { legal, .. } = session.pending().clone() else {
            panic!("{:?}", session.pending());
        };
        session
            .act(
                ME,
                PlayerAction::CastSpell {
                    card: legal.castable[0],
                },
            )
            .expect("cast");
        // Taken back at its target: past that it is on the stack.
        session
            .act(ME, PlayerAction::CancelCast)
            .expect("taken back");
        no_loop(&session, "a cast taken back");
    }
    the_spell_resolves(&mut session);
}

/// A draw declined is a draw not agreed. The answer is the other player's
/// (at a house seat, the house's), so this must not be how one is forced.
#[test]
fn a_draw_declined_again_and_again_is_no_draw() {
    let mut session = table(&["Lightning Bolt"], &["Mountain"]);
    cast_at_them(&mut session);
    for _ in 0..FRAMES / 2 {
        session.act(ME, PlayerAction::OfferDraw).expect("offered");
        session
            .act(THEM, PlayerAction::YesNo(false))
            .expect("declined");
        no_loop(&session, "a draw declined");
    }
    the_spell_resolves(&mut session);
}

/// A setting that says what already stands sends nobody anything and leaves
/// no line in the record, or one seat's small frames would become a view for
/// every seat at the table, hundreds of times a second.
#[test]
fn a_setting_that_changes_nothing_sends_nobody_anything() {
    let mut session = table(&["Lightning Bolt"], &["Mountain"]);
    cast_at_them(&mut session);
    let hold = PlayerAction::SetPriorityHold(PriorityHold::Always);
    session
        .act(THEM, hold.clone())
        .expect("the first says something");
    let seq = session.seq();
    let frames = session.act(THEM, hold).expect("taken");
    assert!(frames.is_empty(), "{} frames", frames.len());
    assert_eq!(session.seq(), seq, "and nothing moved");
}

/// A setting that changes something is a line in the record, so a seat has
/// only so many of them while nothing happens. Past that it is told so, the
/// game goes on, the other seat has its own, and what happens earns more.
#[test]
fn a_seat_has_only_so_many_settings_while_nothing_happens() {
    let mut session = table(&["Lightning Bolt"], &["Mountain"]);
    cast_at_them(&mut session);
    let holds = [PriorityHold::Always, PriorityHold::PassWhenNothingToDo];
    let setting = |n: u64| PlayerAction::SetPriorityHold(holds[(n % 2) as usize]);
    let allowed = session.settings_allowed();
    assert!(allowed < answers::MOST_SETTINGS, "{allowed}");
    for n in 0..allowed {
        session.act(THEM, setting(n)).expect("within its allowance");
    }
    let seq = session.seq();
    let refused = session.act(THEM, setting(allowed));
    assert!(refused.is_err(), "one past its allowance");
    assert_eq!(session.seq(), seq, "and nothing moved");
    session.act(ME, setting(0)).expect("the other seat's own");
    the_spell_resolves(&mut session);
    assert!(session.settings_allowed() > allowed, "the game moved on");
    session
        .act(THEM, setting(allowed))
        .expect("and earned more");

    // And a game going on has an end to them too.
    session.settings_made[THEM.get() as usize] = answers::MOST_SETTINGS;
    assert!(session.settings_allowed() <= answers::MOST_SETTINGS);
    assert!(session.act(THEM, setting(allowed + 1)).is_err());
}

/// A setting that changes something changes one seat's table, so nobody
/// else is sent a view of it.
#[test]
fn a_changed_setting_is_told_only_to_its_seat() {
    let mut session = table(&["Lightning Bolt"], &["Mountain"]);
    cast_at_them(&mut session);
    for hold in [PriorityHold::Always, PriorityHold::PassWhenNothingToDo] {
        let frames = session
            .act(THEM, PlayerAction::SetPriorityHold(hold))
            .expect("taken");
        assert!(!frames.is_empty(), "its own seat hears of it");
        assert!(
            frames.iter().all(|(seat, _)| *seat == THEM),
            "another seat was sent a view"
        );
    }
}

/// The handles in a setting are the seat's to name, so what it may hold of
/// them is bounded, and what it already holds it can still take back.
#[test]
fn a_seat_holds_only_so_many_yields() {
    let mut session = table(&["Lightning Bolt"], &["Mountain"]);
    let most = baylee_engine::choice::SeatAutomation::MAX_ENTRIES;
    let ability = |index: u32| baylee_core::ids::AbilityRef {
        card: island(),
        index,
    };
    for index in 0..u32::try_from(most).expect("fits") + 16 {
        // What is under test is what a seat holds, not how fast it asks.
        session.settings_made.fill(0);
        session
            .act(
                ME,
                PlayerAction::SetAbilityYield {
                    ability: ability(index),
                    enabled: true,
                },
            )
            .expect("taken");
    }
    let held = session.engine.automation(ME).yielded_abilities().count();
    assert_eq!(held, most);
    session
        .act(
            ME,
            PlayerAction::SetAbilityYield {
                ability: ability(0),
                enabled: false,
            },
        )
        .expect("taken");
    assert_eq!(
        session.engine.automation(ME).yielded_abilities().count(),
        most - 1
    );
}

/// A question stays answerable however often somebody repeats a setting
/// beside it, a modal trigger's above all: its plan stands on the queued
/// trigger, and a chair whose answers are all refused stalls the table.
#[test]
fn a_modal_trigger_is_still_answered_after_settings_said_beside_it() {
    let mut session = table(
        &["Aether Channeler"],
        &["Island", "Island", "Island", "Island"],
    );
    for _ in 0..4 {
        tap_a_land(&mut session);
    }
    let Pending::Priority { legal, .. } = session.pending().clone() else {
        panic!("{:?}", session.pending());
    };
    session
        .act(
            ME,
            PlayerAction::CastSpell {
                card: legal.castable[0],
            },
        )
        .expect("cast");
    for _ in 0..6 {
        let Pending::Priority { player, .. } = session.pending().clone() else {
            break;
        };
        session
            .act(player, PlayerAction::PassPriority)
            .expect("passed");
    }
    assert!(
        matches!(session.pending(), Pending::ChooseCastMode { .. }),
        "{:?}",
        session.pending()
    );
    for _ in 0..FRAMES {
        session
            .act(ME, PlayerAction::SetPriorityHold(PriorityHold::Always))
            .expect("a setting is taken");
    }
    session
        .act(ME, PlayerAction::ChooseMode(0))
        .expect("the question was still open");
}
