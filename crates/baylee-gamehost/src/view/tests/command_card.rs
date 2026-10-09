use super::*;
use crate::session::Session;
use baylee_protocol::v1::{self, Envelope};

// ---- Word of Command shows its caster the target's hand, once (CR 722.4)
//
// `zones::looking_at` has one deliberate exception to "a card another seat
// shows is not looked at": the `CommandCard` question. While Word asks its
// caster to choose, the caster is shown the whole of the target's hand, and
// nobody else is. These tests play it on a three-seat table through the
// session, which is the road a client's frames and print table travel.

const CASTER: PlayerId = PlayerId::new(0);
const TARGET: PlayerId = PlayerId::new(1);
const BYSTANDER: PlayerId = PlayerId::new(2);

/// The target's printing: the only deck, hand and board that uses it, so a
/// seat that holds its entry has been shown a card of the target's.
const TARGETS_PRINT: u16 = 1;

fn entry_of(name: &str, print: u16) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::generated::ALL
            .iter()
            .find(|(_, card)| card.name() == name)
            .unwrap_or_else(|| panic!("{name} is in the pool"))
            .1
            .index,
        print: PrintRef::new(print),
    }
}

fn word_of_command() -> CardIndex {
    by_oracle_id("e8ad3a77-b293-4d69-b080-27ca9f95d443")
        .expect("Word of Command is in the pool")
        .index
}

/// Seat 0 holds Word of Command with two Swamps; seat 1 holds three
/// different cards; seat 2 holds an Island. Each seat's deck, hand and board
/// use a printing of their own (0, 1, 2), so the print table tells whose
/// cards a seat has been shown. Everyone is a human who keeps.
fn a_command_table() -> GamePreset {
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    preset.prints = (0..3).map(|_| print_info("EN", Finish::Normal)).collect();
    let mut seat =
        |at: usize, card: &str, print: u16, hand: Vec<DeckEntry>, board: Vec<DeckEntry>| {
            let spec = &mut preset.seats[at];
            spec.controller = SeatController::Open;
            spec.deck = (0..60).map(|_| entry_of(card, print)).collect();
            spec.starting_hand = Some(hand);
            spec.starting_battlefield = board;
        };
    seat(
        0,
        "Swamp",
        0,
        vec![DeckEntry {
            card: word_of_command(),
            print: PrintRef::new(0),
        }],
        vec![entry_of("Swamp", 0), entry_of("Swamp", 0)],
    );
    seat(
        1,
        "Forest",
        TARGETS_PRINT,
        vec![
            entry_of("Forest", TARGETS_PRINT),
            entry_of("Lightning Bolt", TARGETS_PRINT),
            entry_of("Sol Ring", TARGETS_PRINT),
        ],
        vec![],
    );
    seat(2, "Island", 2, vec![entry_of("Island", 2)], vec![]);
    preset
}

/// The latest view out of what `routed` sent `seat`.
fn latest_view(routed: &[(PlayerId, Envelope)], seat: PlayerId) -> PlayerView {
    routed
        .iter()
        .rev()
        .filter(|(to, _)| *to == seat)
        .find_map(|(_, env)| match &env.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                Some(serde_json::from_slice(&delta.view_json).expect("the view decodes"))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{seat:?} was sent no view"))
}

/// The view `seat` would be sent now, out of a snapshot.
fn snapshot_view(session: &Session, seat: PlayerId) -> PlayerView {
    let routed: Vec<_> = session
        .snapshot(seat)
        .into_iter()
        .map(|env| (seat, env))
        .collect();
    latest_view(&routed, seat)
}

/// Every object id that appears anywhere in a view, a looked-at card and a
/// controlled or shared hand included.
fn ids_in(view: &PlayerView) -> Vec<ObjectId> {
    let mut ids: Vec<ObjectId> = view.hand.iter().map(|c| c.id).collect();
    for zone in [&view.battlefield, &view.stack, &view.looking_at] {
        ids.extend(zone.iter().map(|o| o.id));
    }
    for per_seat in [&view.graveyards, &view.exile, &view.command] {
        for zone in per_seat {
            ids.extend(zone.iter().map(|o| o.id));
        }
    }
    ids.extend(view.library_tops.iter().map(|o| o.id));
    for shared in view.controlled_hands.iter().chain(&view.shared_hands) {
        ids.extend(shared.cards.iter().map(|o| o.id));
    }
    ids
}

/// A table with Word of Command cast at seat 1 and resolving up to its
/// question, and what the table was sent on the way.
struct AtTheChoice {
    session: Session,
    /// What the last action sent each seat.
    routed: Vec<(PlayerId, Envelope)>,
    /// The target's hand.
    hand: Vec<ObjectId>,
}

fn at_the_choice() -> AtTheChoice {
    let mut session = Session::new(&a_command_table()).expect("a three-seat game");
    session.pump();
    for seat in [CASTER, TARGET, BYSTANDER] {
        session
            .act(seat, PlayerAction::MulliganKeep)
            .expect("a keep");
    }
    // Seat 0 in its own first main phase, holding priority over an empty
    // stack. It is then not the target's turn, so its land drop is not its
    // own (CR 305.3), which lets Word finish without playing anything.
    let mut view = snapshot_view(&session, CASTER);
    for _ in 0..300 {
        match session.pending().clone() {
            Pending::Priority { player, .. } => {
                view = snapshot_view(&session, CASTER);
                if player == CASTER
                    && view.active == CASTER
                    && view.phase == Phase::FirstMain
                    && view.stack.is_empty()
                {
                    break;
                }
                session.act(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                session
                    .act(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected question on the way to the main phase: {other:?}"),
        }
    }
    assert_eq!(view.active, CASTER, "seat 0 reached its first main phase");
    let swamps: Vec<ObjectId> = view
        .battlefield
        .iter()
        .filter(|o| o.controller == CASTER)
        .map(|o| o.id)
        .collect();
    assert_eq!(swamps.len(), 2);
    for source in swamps {
        session
            .act(CASTER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let word = view
        .hand
        .iter()
        .find(|o| o.name == "Word of Command")
        .expect("seat 0 holds Word")
        .id;
    session
        .act(CASTER, PlayerAction::CastSpell { card: word })
        .unwrap();
    // Aim it at seat 1, then let it resolve up to its question.
    let mut routed = Vec::new();
    for _ in 0..30 {
        let (player, action) = match session.pending().clone() {
            Pending::ChoosePlayer { player, .. } => (player, PlayerAction::ChoosePlayer(TARGET)),
            Pending::ChooseTargets { player, .. } => (
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![TARGET],
                },
            ),
            Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
            Pending::ChooseCards { .. } => break,
            other => panic!("unexpected question while Word resolves: {other:?}"),
        };
        routed = session.act(player, action).unwrap();
    }
    let Pending::ChooseCards {
        player,
        options,
        prompt: baylee_engine::choice::ChoicePrompt::CommandCard,
        ..
    } = session.pending().clone()
    else {
        panic!(
            "Word asks its caster to choose, got {:?}",
            session.pending()
        );
    };
    assert_eq!(player, CASTER, "the caster is the one asked");
    let hand = session
        .state()
        .zones
        .list(ZoneLocation::Hand(TARGET))
        .clone();
    assert_eq!(hand.len(), 3, "the target holds its three cards");
    let mut offered = options;
    offered.sort();
    let mut sorted = hand.clone();
    sorted.sort();
    assert_eq!(offered, sorted, "the options are the target's whole hand");
    AtTheChoice {
        session,
        routed,
        hand,
    }
}

/// While Word asks, its caster is shown every card of the target's hand,
/// each named and with its card (the exception to `shown_elsewhere`).
#[test]
fn word_of_commands_caster_is_shown_the_whole_hand_it_chooses_from() {
    let table = at_the_choice();
    let view = latest_view(&table.routed, CASTER);
    let mut shown: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
    shown.sort();
    let mut hand = table.hand.clone();
    hand.sort();
    assert_eq!(shown, hand, "every card of the hand, and nothing else");
    let mut names: Vec<&str> = view.looking_at.iter().map(|o| o.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["Forest", "Lightning Bolt", "Sol Ring"]);
    assert!(
        view.looking_at.iter().all(|o| o.card.is_some()),
        "named, with a card to draw"
    );
    assert_eq!(
        view.seat(TARGET).map(|s| s.hand_count),
        Some(3),
        "the hand is still a count on the seat"
    );
}

/// The same moment, the third seat: a count and nothing more. No object id
/// of the hand, none of its names, nothing looked at.
#[test]
fn word_of_commands_hand_is_not_shown_to_a_bystander() {
    let table = at_the_choice();
    let view = latest_view(&table.routed, BYSTANDER);
    assert!(view.looking_at.is_empty(), "nothing looked at");
    assert!(view.controlled_hands.is_empty());
    assert_eq!(
        view.seat(TARGET).map(|s| s.hand_count),
        Some(3),
        "a count is all there is"
    );
    let visible = ids_in(&view);
    for id in &table.hand {
        assert!(
            !visible.contains(id),
            "a card of the commanded hand reached the bystander: {id:?}"
        );
    }
    let wire = serde_json::to_string(&view).expect("a view serialises");
    for name in ["Lightning Bolt", "Sol Ring"] {
        assert!(!wire.contains(name), "{name} is in the bystander's view");
    }
}

/// The commanded player's own view is the one it always had: its hand, its
/// cards, nothing looked at, no one else's hand.
#[test]
fn word_of_commands_target_sees_what_it_always_saw() {
    let table = at_the_choice();
    let view = snapshot_view(&table.session, TARGET);
    assert!(view.looking_at.is_empty(), "it is not the one choosing");
    assert!(view.controlled_hands.is_empty());
    let mut own: Vec<ObjectId> = view.hand.iter().map(|c| c.id).collect();
    own.sort();
    let mut hand = table.hand.clone();
    hand.sort();
    assert_eq!(own, hand, "its own hand, whole");
    let mut names: Vec<&str> = view.hand.iter().map(|c| c.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["Forest", "Lightning Bolt", "Sol Ring"]);
    let casters = table
        .session
        .state()
        .zones
        .list(ZoneLocation::Hand(CASTER))
        .clone();
    assert_eq!(
        view.seat(CASTER).map(|s| s.hand_count),
        Some(casters.len() as u32),
        "and still only a count of the caster's hand"
    );
    let visible = ids_in(&view);
    for id in &casters {
        assert!(!visible.contains(id), "the caster's card {id:?} is shown");
    }
}

/// The look is the question, not a memory: once the caster has chosen and
/// Word is done, the caster's view names none of what remains in the hand.
#[test]
fn word_of_commands_hand_is_hidden_again_once_the_choice_is_made() {
    let mut table = at_the_choice();
    let forest = table.session.state().zones.list(ZoneLocation::Hand(TARGET))[0];
    let chosen = table
        .hand
        .iter()
        .copied()
        .find(|&id| {
            table.session.state().object(id).is_some_and(|o| {
                o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
            })
        })
        .unwrap_or(forest);
    table
        .session
        .act(
            CASTER,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    // Word has the target play the Forest "if able": it is not its turn, so
    // nothing is played. Pass until Word is done.
    for _ in 0..30 {
        match table.session.pending().clone() {
            Pending::Priority { player, .. }
                if !table.session.state().zones.stack_is_empty() || player != CASTER =>
            {
                table
                    .session
                    .act(player, PlayerAction::PassPriority)
                    .unwrap();
            }
            _ => break,
        }
    }
    assert!(
        table.session.state().zones.stack_is_empty(),
        "Word is done resolving"
    );
    let remaining = table
        .session
        .state()
        .zones
        .list(ZoneLocation::Hand(TARGET))
        .clone();
    assert_eq!(remaining.len(), 3, "the target still holds its cards");
    for seat in [CASTER, BYSTANDER] {
        let view = snapshot_view(&table.session, seat);
        assert!(view.looking_at.is_empty(), "{seat:?}: nothing looked at");
        assert!(view.controlled_hands.is_empty(), "{seat:?}");
        let visible = ids_in(&view);
        for id in &remaining {
            assert!(!visible.contains(id), "{seat:?} still sees {id:?}");
        }
        let wire = serde_json::to_string(&view).expect("a view serialises");
        for name in ["Lightning Bolt", "Sol Ring"] {
            assert!(!wire.contains(name), "{seat:?}'s view still names {name}");
        }
    }
}

/// The print table: the caster earns the target's printing by being shown the
/// hand; the bystander never does, in a view, a frame or the table itself.
#[test]
fn word_of_commands_printing_reaches_only_its_caster() {
    let table = at_the_choice();
    let print = PrintRef::new(TARGETS_PRINT);
    assert!(
        table.session.game_static(CASTER).print(print).is_some(),
        "the caster was shown cards of that printing"
    );
    assert!(
        table.session.game_static(BYSTANDER).print(print).is_none(),
        "the bystander was not, and must not learn the target's printings"
    );
    let bystander = latest_view(&table.routed, BYSTANDER);
    assert!(
        bystander.prints().all(|p| p != print),
        "no object in the bystander's view points at the target's printing"
    );
    assert!(
        table
            .routed
            .iter()
            .filter(|(to, _)| *to == BYSTANDER)
            .all(|(_, env)| !matches!(env.msg, Some(v1::envelope::Msg::GameStatic(_)))),
        "nothing re-sent the bystander its table at the choice"
    );
}

/// The engine already withholds the question from a seat it is not
/// addressed to (`information_pending_for`), and the view does not lean on
/// that alone: handed the caster's question by mistake, as
/// `controlled_choice_projection_keeps_private_zones_separate` does for a
/// face-down cast, the target and the bystander are still shown nothing,
/// while the caster, the one it is addressed to, is shown the hand.
#[test]
fn word_of_commands_question_shown_by_mistake_still_shows_the_wrong_seat_nothing() {
    let table = at_the_choice();
    let pending = table.session.pending().clone();
    let state = table.session.state();
    let ctx = SeatContext {
        awaiting: Some(CASTER),
        decision_player: Some(CASTER),
        ..SeatContext::default()
    };
    for seat in [TARGET, BYSTANDER] {
        let view = player_view(state, seat, 0, Some(&pending), &ctx, &[]);
        assert!(
            view.looking_at.is_empty(),
            "{seat:?} was handed the caster's question and is shown the hand"
        );
    }
    let view = player_view(state, CASTER, 0, Some(&pending), &ctx, &[]);
    assert_eq!(view.looking_at.len(), table.hand.len());
}
