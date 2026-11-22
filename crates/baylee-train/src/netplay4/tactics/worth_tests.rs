//! What the layer reads as worth casting, on real tables: a pump that ends
//! with the turn once no fight is left in it, a counter with only the seat's
//! own spells to name, and a counter the `React` reading does not see.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::{AIProfile, FormatId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{Phase, PlayerView, Step};

use super::{guard, is_modal_mode_worth_casting, reading};

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// A table with these hands and boards, `first` taking the first turn.
fn setup(
    mine: (&[&str], &[&str]),
    theirs: (&[&str], &[&str]),
    first: PlayerId,
) -> baylee_gamehost::Session {
    setup_at(mine, theirs, first, None)
}

/// [`setup`], the seat starting at `life` where given.
fn setup_at(
    mine: (&[&str], &[&str]),
    theirs: (&[&str], &[&str]),
    first: PlayerId,
    life: Option<i32>,
) -> baylee_gamehost::Session {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(915, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    preset.house_rules.starting_player = Some(first.get());
    let card = |name: &str| {
        let mut entry = preset.seats[0].deck[0];
        entry.card = baylee_cards::decks::by_name(name).unwrap();
        entry.rule_print = None;
        entry.choices = baylee_core::card_metadata::DeckChoices::default();
        entry
    };
    let sides = [mine, theirs].map(|(hand, board)| {
        (
            hand.iter().map(|name| card(name)).collect::<Vec<_>>(),
            board.iter().map(|name| card(name)).collect::<Vec<_>>(),
        )
    });
    for (spec, (hand, board)) in preset.seats.iter_mut().zip(sides) {
        spec.commanders.clear();
        spec.starting_hand = Some(hand);
        spec.starting_battlefield = board;
    }
    if let Some(life) = life {
        preset.seats[0].starting_life = Some(life);
    }
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    session.take_over(ME);
    session.take_over(THEM);
    session
}

/// Passes until `at` holds for the seat's own priority, and returns its view.
fn until(
    session: &mut baylee_gamehost::Session,
    at: impl Fn(&PlayerView) -> bool,
) -> (Pending, PlayerView) {
    for _ in 0..96 {
        if let Pending::Priority { player, .. } = session.pending()
            && *player == ME
        {
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            if at(&view) {
                return (pending, view);
            }
        }
        let player = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            other => panic!("unexpected setup decision {other:?}"),
        };
        session.act(player, action).unwrap();
    }
    panic!("the moment was not reached");
}

fn in_hand(view: &PlayerView, name: &str) -> ObjectId {
    view.hand.iter().find(|card| card.name == name).unwrap().id
}

#[test]
fn a_pump_that_ends_with_the_turn_waits_once_their_combat_is_over() {
    let mut session = setup(
        (&["Breach"], &["Swamp", "Swamp", "Swamp", "Grizzly Bears"]),
        (&[], &["Plains"]),
        THEM,
    );
    // Before their combat a pump may still turn a block.
    let (_, view) = until(&mut session, |view| {
        view.active == THEM && view.step == Step::Upkeep
    });
    assert!(reading::worth_casting(&view, in_hand(&view, "Breach")));
    // At their end step it would end before it could do anything.
    let (pending, view) = until(&mut session, |view| {
        view.active == THEM && view.step == Step::End && view.stack.is_empty()
    });
    let breach = in_hand(&view, "Breach");
    assert!(!reading::worth_casting(&view, breach));
    let Pending::Priority { legal, .. } = &pending else {
        panic!("not a priority question");
    };
    assert!(
        reading::react_window(&view, legal).is_none_or(|(_, spells)| !spells.contains(&breach)),
        "React would cast the pump into their end step"
    );
}

/// Izzet Charm's mode that counters, behind its payment.
fn charm_counter_mode() -> &'static baylee_cards_dsl::SpellMode {
    let charm =
        baylee_cards::by_index(baylee_cards::decks::by_name("Izzet Charm").unwrap()).unwrap();
    charm
        .abilities_for_face(0)
        .iter()
        .find_map(|ability| match ability {
            baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. }
            | baylee_cards_dsl::AbilityDef::Modal { modes, .. } => modes.first(),
            _ => None,
        })
        .unwrap()
}

#[test]
fn a_counter_mode_with_only_the_seats_own_spell_to_name_does_nothing() {
    let mut session = setup(
        (
            &["Divination", "Izzet Charm"],
            &["Island", "Island", "Island", "Island", "Mountain"],
        ),
        (&[], &["Plains"]),
        ME,
    );
    let (_, view) = until(&mut session, |view| {
        view.active == ME && view.phase == Phase::FirstMain
    });
    let islands: Vec<ObjectId> = view
        .battlefield
        .iter()
        .filter(|object| object.name == "Island")
        .map(|object| object.id)
        .take(3)
        .collect();
    for source in islands {
        session
            .act(ME, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&view, "Divination");
    session.act(ME, PlayerAction::CastSpell { card }).unwrap();
    let (_, view) = until(&mut session, |view| !view.stack.is_empty());
    assert!(view.stack.iter().all(|item| item.controller == ME));
    assert!(!is_modal_mode_worth_casting(
        &view,
        ME,
        charm_counter_mode()
    ));
}

/// Their Ornithopter on the stack in their main phase, the seat to answer.
fn their_spell_on_the_stack(mine: (&[&str], &[&str])) -> (Pending, PlayerView) {
    let mut session = setup(mine, (&["Ornithopter"], &["Plains"]), THEM);
    for _ in 0..96 {
        let view = session.observe_view(THEM);
        if view.active == THEM
            && view.phase == Phase::FirstMain
            && matches!(session.pending(), Pending::Priority { player, .. } if *player == THEM)
        {
            let card = in_hand(&view, "Ornithopter");
            session.act(THEM, PlayerAction::CastSpell { card }).unwrap();
            return until(&mut session, |view| !view.stack.is_empty());
        }
        let player = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected setup decision {other:?}"),
        };
        session.act(player, action).unwrap();
    }
    panic!("their main phase not reached");
}

#[test]
fn a_counter_behind_a_payment_is_cast_at_their_spell_by_the_counter_group_alone() {
    let (pending, view) = their_spell_on_the_stack((&["Mana Leak"], &["Island", "Island"]));
    let Pending::Priority { legal, .. } = &pending else {
        panic!("not a priority question");
    };
    let leak = in_hand(&view, "Mana Leak");
    // The React reading sees one effect list: the counter inside the payment
    // is not in it.
    assert!(reading::react_window(&view, legal).is_none_or(|(_, spells)| !spells.contains(&leak)));
    let state = crate::features7::encode_state7(&view, Some(&pending), None, &[]);
    let cands = crate::features7::encode_candidates7_grouped(&state, &view, &pending);
    let logits = vec![0.0; cands.candidates.len()];
    let chosen = guard::counter(&cands, &logits, &view, legal).expect("the counter group answers");
    let action = cands.candidates[chosen].action.as_ref().unwrap();
    assert!(
        *action == PlayerAction::CastSpell { card: leak } || super::is_mana_action(&view, action),
        "{action:?}"
    );
    assert!(is_modal_mode_worth_casting(&view, ME, charm_counter_mode()));
}

#[test]
fn the_counter_group_waits_without_a_counter_in_hand() {
    let (pending, view) = their_spell_on_the_stack((&["Grizzly Bears"], &["Forest", "Forest"]));
    let Pending::Priority { legal, .. } = &pending else {
        panic!("not a priority question");
    };
    let state = crate::features7::encode_state7(&view, Some(&pending), None, &[]);
    let cands = crate::features7::encode_candidates7_grouped(&state, &view, &pending);
    let logits = vec![0.0; cands.candidates.len()];
    assert!(guard::counter(&cands, &logits, &view, legal).is_none());
}

#[test]
fn a_modal_spells_target_is_left_to_the_net_where_its_modes_disagree() {
    // At 1 life Reverent Howl's "draw two, lose two" would kill the seat; its
    // other mode pumps the seat's own creature. Which mode is being aimed the
    // view does not say, so the layer does not force the seat's own side.
    let mut session = setup_at(
        (
            &["Reverent Howl"],
            &["Swamp", "Swamp", "Swamp", "Grizzly Bears"],
        ),
        (&[], &["Plains"]),
        ME,
        Some(1),
    );
    let (_, view) = until(&mut session, |view| {
        view.active == ME && view.phase == Phase::FirstMain
    });
    let swamps: Vec<ObjectId> = view
        .battlefield
        .iter()
        .filter(|object| object.name == "Swamp")
        .map(|object| object.id)
        .collect();
    for source in swamps {
        session
            .act(ME, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let howl = in_hand(&view, "Reverent Howl");
    session
        .act(ME, PlayerAction::CastSpell { card: howl })
        .unwrap();
    session.act(ME, PlayerAction::ChooseMode(0)).unwrap();
    let (pending, view) = session.view_for_with_context(ME).unwrap();
    let state = crate::features7::encode_state7(&view, Some(&pending), None, &[]);
    let cands = crate::features7::encode_candidates7_grouped(&state, &view, &pending);
    let theirs = |action: &PlayerAction| match action {
        PlayerAction::ChoosePlayer(player) => *player == THEM,
        PlayerAction::ChooseTargets { players, .. } => players == &[THEM],
        _ => false,
    };
    let them = cands
        .candidates
        .iter()
        .position(|cand| cand.action.as_ref().is_some_and(theirs))
        .expect("the other player is a candidate");
    let mut logits = vec![0.0; cands.candidates.len()];
    logits[them] = 4.0;
    let mut rng = crate::deckgen::Rng::new(3);
    assert_eq!(view.seat(ME).map(|seat| seat.life), Some(1));
    let (chosen, _) = super::select_single_choice(
        &cands,
        &logits,
        &view,
        &pending,
        Some((howl, None)),
        0.0,
        &mut rng,
        false,
    );
    assert_eq!(chosen, them, "the net's aim at the other player stands");
}
