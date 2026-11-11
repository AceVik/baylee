//! What the Carbon seat was told wrong or not at all (chunks b and c,
//! 11.11.2026): an X announced on the stack, the sentence about paying at
//! plain priority, a gone token's trigger, and a granted `{0}` ability
//! labelled with the sentence that granted it.

use super::*;
use crate::wake::tests::played::position;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::Session;
use baylee_view::{LogTail, StackItem};

fn asked(session: &Session) -> Request {
    let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
    request(view, pending, LogTail::default())
}

fn told(request: &Request) -> String {
    Narrator::new(&request.context)
        .wake(request, &[], None)
        .text
}

/// Answers what casting asks of seat 0 (X, a player or a target, both P2)
/// until it has priority again.
fn finish_cast(session: &mut Session, x: u32) {
    for _ in 0..8 {
        let action = match session.pending().clone() {
            Pending::Priority { .. } => return,
            Pending::ChooseNumber { .. } => PlayerAction::ChooseNumber(x),
            Pending::ChoosePlayer { .. } => PlayerAction::ChoosePlayer(THEM),
            Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![THEM],
            },
            other => panic!("casting asked {other:?}"),
        };
        session.act_slice(ME, action, 0).expect("the cast's answer");
    }
    panic!("the cast never gave priority back");
}

/// Blaze cast with X = 2: the stack says so, as the player who cast it
/// announced it (CR 601.2b, 107.3a). The view carried `chosen_x` and the
/// seat never read it, so Gaze of Granite resolved over a model that could
/// not tell which of its permanents it destroyed (Carbon b3, hc-expert-b
/// game-0004 q314).
#[test]
fn a_spell_on_the_stack_says_its_x() {
    let (mut session, view, _) = position(&["Mountain", "Mountain", "Mountain"], &["Blaze"], &[]);
    for mountain in view
        .battlefield
        .iter()
        .filter(|o| o.controller == ME)
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        session
            .act_slice(
                ME,
                PlayerAction::ActivateManaAbility { source: mountain },
                0,
            )
            .expect("a Mountain's red");
    }
    let blaze = view.hand[0].id;
    session
        .act_slice(ME, PlayerAction::CastSpell { card: blaze }, 0)
        .expect("three red pay Blaze with X = 2");
    finish_cast(&mut session, 2);
    let request = asked(&session);
    assert_eq!(
        request.view.object(blaze).and_then(|o| o.chosen_x),
        Some(2),
        "the view carries the X"
    );
    let text = told(&request);
    let line = text
        .lines()
        .find(|line| line.contains(&format!("{} Blaze", tag(blaze))))
        .unwrap_or_else(|| panic!("Blaze on the stack: {text}"));
    assert!(line.contains("X is 2"), "{line}");
}

/// An ability with X on the stack says it too.
#[test]
fn an_ability_on_the_stack_says_its_x() {
    let (mut view, log) = board();
    let mut ability = token(70, 1, "", 0, 0);
    ability.controller = THEM;
    ability.stack_item = Some(StackItem::Ability {
        source: id(45),
        ability: None,
        text: None,
        rules: None,
        token: None,
    });
    ability.chosen_x = Some(3);
    view.stack = vec![ability];
    let text = told(&request(view, priority(&board().0), log));
    let line = text
        .lines()
        .find(|line| line.contains("#70 P2's ability of Serra Angel"))
        .unwrap_or_else(|| panic!("the ability on the stack: {text}"));
    assert!(line.contains("X is 3"), "{line}");
}

/// A Pest token's "When this token dies" on the stack after the Pest died:
/// its name is the token's (CR 111.4, 113.7a), not "an unnamed object"
/// (Carbon b2, hc-expert-b game-0004 q284).
#[test]
fn a_gone_token_s_trigger_names_the_token() {
    let (mut view, log) = board();
    let pest =
        baylee_cards::tokens::token_id(&baylee_cards::generated_tokens::PEST_1_1_BLACK_GREEN);
    let mut ability = token(123, 1, "", 0, 0);
    ability.controller = THEM;
    ability.stack_item = Some(StackItem::Ability {
        source: id(120),
        ability: None,
        text: None,
        rules: None,
        token: Some(baylee_view::TokenAbility {
            token: pest,
            index: 0,
        }),
    });
    view.stack = vec![ability];
    let request = request(view, priority(&board().0), log);
    let text = told(&request);
    assert!(!text.contains("an unnamed object"), "{text}");
    assert!(text.contains("#123 P2's ability of Pest #120"), "{text}");
    // The ability itself, wherever a line names it by its handle (the
    // held line's "woke because P2's … is on the stack"): Carbon d2
    // game-0005 q136 read "P2's an unnamed object #123".
    let table = Table::new(&request.view, &request.context);
    assert_eq!(table.named(id(123)), "Pest's ability #123");
}

/// At a priority the seat is woken for because the mana it made this step
/// still floats (`Verdict::Continue`), nothing is being paid for: the
/// question says what floats, never "in the middle of paying for
/// something" (Carbon b1 and b6: the pump of Shivan Dragon had resolved,
/// Skycloud Egg had resolved).
#[test]
fn a_priority_with_mana_floating_does_not_say_a_payment_is_under_way() {
    let (mut view, log) = board();
    view.seats[0].mana_pool.white = 1;
    view.seats[0].mana_pool.blue = 1;
    let mut request = request(view, priority(&board().0), log);
    request.continuing = true;
    let text = told(&request);
    assert!(!text.contains("middle of paying"), "{text}");
    assert!(text.contains("Mana is in your pool"), "{text}");
}

/// Wandering Fumarole animated has "{0}: Switch this creature's power and
/// toughness until end of turn": the option is that ability, in its own
/// words, never the `{2}{U}{R}` sentence that granted it (Carbon b7,
/// hc-expert-b game-0018 q271).
#[test]
fn a_granted_ability_is_labelled_with_its_own_words() {
    let (mut session, view, legal) = position(
        &[
            "Wandering Fumarole",
            "Island",
            "Island",
            "Mountain",
            "Mountain",
        ],
        &[],
        &[],
    );
    let fumarole = view
        .battlefield
        .iter()
        .find(|o| o.name.as_ref() == "Wandering Fumarole")
        .expect("the Fumarole")
        .id;
    for land in view
        .battlefield
        .iter()
        .filter(|o| o.controller == ME && o.id != fumarole)
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        session
            .act_slice(ME, PlayerAction::ActivateManaAbility { source: land }, 0)
            .expect("a land's mana");
    }
    assert!(
        legal
            .unpaid_abilities
            .iter()
            .any(|&(s, i, _)| s == fumarole && i == 1)
    );
    session
        .act_slice(
            ME,
            PlayerAction::ActivateAbility {
                source: fumarole,
                ability_index: 1,
            },
            0,
        )
        .expect("the animation, paid from the pool");
    // Both pass: the animation resolves, and priority comes back.
    for _ in 0..4 {
        let pending = session.pending().clone();
        if session.observe_view(ME).stack.is_empty() && pending.asked() == Some(ME) {
            break;
        }
        let player = pending.asked().expect("someone is asked");
        session
            .act_slice(player, PlayerAction::PassPriority, 0)
            .expect("a pass");
    }
    let request = asked(&session);
    let Pending::Priority { legal, .. } = &request.pending else {
        panic!("priority, not {:?}", request.pending);
    };
    let (_, granted) = *legal
        .abilities
        .iter()
        .find(|&&(s, i)| s == fumarole && baylee_engine::choice::granted_slot(i).is_some())
        .expect("the granted {0} is offered");
    let menu = Narrator::new(&request.context).menu(&request);
    let label = menu
        .offered()
        .iter()
        .find(|o| {
            o.text
                .starts_with(&format!("Activate Wandering Fumarole {}", tag(fumarole)))
        })
        .map(|o| o.text.clone())
        .unwrap_or_else(|| {
            panic!(
                "the granted ability's option ({granted}): {:?}",
                menu.offered()
            )
        });
    assert!(
        label.contains("{0}: Switch this creature's power and toughness until end of turn."),
        "{label}"
    );
    assert!(!label.contains("{2}{U}{R}"), "{label}");
}

/// Great Hall of the Biblioplex's black floats for instants and sorceries
/// only: the seat's line says which colour is restricted, as the view counts
/// it, beside the plain mana (Carbon b9: the line read "+1 restricted").
#[test]
fn restricted_mana_floating_is_told_by_colour() {
    let (mut view, log) = board();
    view.seats[0].mana_pool.white = 1;
    view.seats[0].mana_pool.restricted[baylee_core::mana::ManaColor::Black.index()] = 1;
    let text = told(&request(view.clone(), priority(&board().0), log.clone()));
    assert!(
        text.contains("pool {W} (+{B} restricted to some spells)"),
        "{text}"
    );
    view.seats[0].mana_pool.white = 0;
    let text = told(&request(view, priority(&board().0), log));
    assert!(
        text.contains("pool {B} (restricted to some spells)"),
        "{text}"
    );
}

/// Sphinx's Revelation cast for X = 2 from a payment window: the window
/// names the spell and its X, and says what passing short does, which the
/// engine then does: the cast is reversed (CR 601.2h, 734.1), the card
/// stays in the hand and the Island tapped for it untaps. It read "Pay it,
/// or pass and leave it unpaid (what it was for is lost)" with neither the
/// spell nor X (Carbon d1 game-0008 q164, d2 game-0007 q114).
#[test]
fn a_cast_s_payment_window_names_the_spell_and_what_passing_does() {
    let (mut session, view, legal) = position(
        &["Plains", "Island", "Island", "Island", "Island"],
        &["Sphinx's Revelation"],
        &[],
    );
    let revelation = view.hand[0].id;
    assert!(legal.payable.contains(&revelation), "{legal:?}");
    session
        .act_slice(ME, PlayerAction::CastSpell { card: revelation }, 0)
        .expect("the cast begins");
    finish_cast(&mut session, 2);
    let island = view
        .battlefield
        .iter()
        .find(|o| o.controller == ME && o.name.as_ref() == "Island")
        .expect("an Island")
        .id;
    session
        .act_slice(ME, PlayerAction::ActivateManaAbility { source: island }, 0)
        .expect("one Island toward the cost");
    // As a socket frame carries it: the choice context names the cast.
    let (pending, view_now) = session.view_for_with_context(ME).expect("seat 0 is asked");
    let request = request(view_now, pending, LogTail::default());
    assert_eq!(request.view.casting, Some(revelation));
    let text = told(&request);
    assert!(!text.contains("what it was for is lost"), "{text}");
    assert!(!text.contains("without paying the rest"), "{text}");
    let owed = text
        .lines()
        .find(|line| line.contains("owe {2}{W}{U}{U}"))
        .unwrap_or_else(|| panic!("the owed line: {text}"));
    assert!(
        owed.contains(&format!("Sphinx's Revelation {}", tag(revelation)))
            && owed.contains("X is 2"),
        "{owed}"
    );
    assert!(owed.contains("cancels the cast"), "{owed}");
    let pass = text
        .lines()
        .find(|line| line.trim_start().starts_with("p "))
        .unwrap_or_else(|| panic!("the pass option: {text}"));
    assert!(pass.contains("cancel"), "{pass}");
    // What the words say is what the table does.
    session
        .act_slice(ME, PlayerAction::PassPriority, 0)
        .expect("a pass settles the window");
    let after = session.observe_view(ME);
    assert!(
        after.hand.iter().any(|c| c.id == revelation),
        "the card is back"
    );
    assert!(after.stack.is_empty());
    assert!(
        after
            .battlefield
            .iter()
            .any(|o| o.id == island && !o.status.is_tapped()),
        "the Island untapped"
    );
}
