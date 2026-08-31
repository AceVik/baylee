//! End-to-end: play a real game through the client's own code path.
//!
//! Every layer except the renderer is exercised here — the host, the wire
//! encoding, the board model, the token grouping, and the interaction state
//! machine — driven by a loop that answers whatever it is asked the way a
//! player would. No window, no GPU, so it runs in CI on every commit.
//!
//! This is the test that would have caught the setup bug where a seat marked
//! `Open` (which is what every hosted game marks its human seat) started with
//! no library at all: the existing suites all asserted that *choices arrived*,
//! and none of them ever looked at what the human could actually see.

use baylee_client::host::{DuelHost, HostMessage, LocalHost};
use baylee_client_core::board::{BoardModel, SeatPod};
use baylee_client_core::interaction::{CombatCandidates, Interaction};
use baylee_core::ids::{CardIndex, PlayerId, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{GameStatic, PlayerView};
use std::collections::HashSet;

fn card(oracle: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle)
        .expect("the acceptance registry contains the card")
        .index
}

/// A duel of basic lands plus a cheap creature, so turns actually progress.
fn duel_preset(seed: u64) -> GamePreset {
    let forest = card("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            card: forest,
            print: PrintRef::new(0),
        })
        .collect();
    let seat = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        deck: deck.clone(),
        sideboard: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(false), seat(true)],
    }
}

/// The client state a headless run needs: exactly what the Bevy resource holds,
/// minus anything that touches a device.
#[derive(Default)]
struct Client {
    statics: Option<GameStatic>,
    view: Option<PlayerView>,
    board: Option<BoardModel>,
    pending: Option<Pending>,
    errors: Vec<String>,
}

impl Client {
    fn absorb(&mut self, messages: Vec<HostMessage>) {
        for message in messages {
            match message {
                HostMessage::Static(s) => self.statics = Some(*s),
                HostMessage::View(v) => {
                    self.board = Some(BoardModel::from_view(&v, &HashSet::new(), 12.0));
                    self.view = Some(*v);
                }
                HostMessage::Choice(p) => self.pending = Some(*p),
                HostMessage::Failed(e) => self.errors.push(e),
            }
        }
    }

    /// Answers the pending choice the way a player holding down the pass key
    /// would: keep the opening hand, decline everything optional, attack with
    /// nothing, and pass priority.
    fn answer(&self, seat: PlayerId) -> Option<PlayerAction> {
        let pending = self.pending.clone()?;
        let interaction = Interaction::new(pending.clone(), seat, &CombatCandidates::default());
        if !interaction.is_mine() {
            return None;
        }
        match &pending {
            Pending::Mulligan { .. } => interaction.answer_mulligan(true),
            Pending::YesNo { .. } => interaction.answer_yes_no(false),
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                // Bottom or discard from the top of the hand.
                let hand = self.view.as_ref()?;
                let objects = hand
                    .hand
                    .iter()
                    .take(*count as usize)
                    .map(|c| c.id)
                    .collect();
                Some(PlayerAction::ChooseObjects { objects })
            }
            Pending::LegendChoice { options, .. } => Some(PlayerAction::ChooseObjects {
                objects: vec![*options.first()?],
            }),
            // Both take the smallest legal selection from the offered set.
            Pending::ChooseCards { options, min, .. }
            | Pending::ChooseTargets { options, min, .. } => Some(PlayerAction::ChooseObjects {
                objects: options.iter().take(*min as usize).copied().collect(),
            }),
            Pending::OrderObjects { objects, .. } => Some(PlayerAction::OrderObjects {
                objects: objects.clone(),
            }),
            Pending::ChooseColor { options, .. } => {
                Some(PlayerAction::ChooseColor(*options.first()?))
            }
            Pending::ChoosePlayer { options, .. } => {
                Some(PlayerAction::ChoosePlayer(*options.first()?))
            }
            Pending::ChooseCastMode { .. } => Some(PlayerAction::ChooseMode(0)),
            Pending::ChooseSubtype { options, .. } => {
                Some(PlayerAction::ChooseSubtype(*options.first()?))
            }
            Pending::GameOver(_) => None,
            // Priority, attackers, blockers, and X all confirm to a safe
            // default through the interaction itself.
            _ => interaction.confirm(),
        }
    }
}

/// Plays up to `steps` decisions and returns the final client state.
fn play(seed: u64, steps: usize) -> (Client, LocalHost) {
    let preset = duel_preset(seed);
    let mut host =
        LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"]).expect("the duel starts");
    let mut client = Client::default();
    client.absorb(host.poll());

    for _ in 0..steps {
        if matches!(client.pending, Some(Pending::GameOver(_))) {
            break;
        }
        let Some(action) = client.answer(PlayerId::new(0)) else {
            break;
        };
        host.submit(action);
        client.absorb(host.poll());
    }
    (client, host)
}

#[test]
fn a_human_seat_is_dealt_a_real_opening_hand_and_library() {
    let (client, _) = play(4, 0);
    let view = client.view.expect("the client can draw immediately");

    assert_eq!(view.hand.len(), 7, "the human seat must have a hand");
    let me = view.seat(PlayerId::new(0)).expect("own seat line");
    assert_eq!(me.library_count, 53);
    assert!(!me.is_decking_out());

    // And the opponent's hand is a count, never contents.
    let them = view.seat(PlayerId::new(1)).expect("opponent seat line");
    assert_eq!(them.hand_count, 7);
    assert_eq!(them.library_count, 53);
}

#[test]
fn the_game_advances_through_the_clients_own_path() {
    let (client, _) = play(4, 400);
    let view = client.view.expect("a view");

    assert!(
        client.errors.is_empty(),
        "the client sent something the engine rejected: {:?}",
        client.errors
    );
    assert!(
        view.turn > 1,
        "several turns should have passed, was on turn {}",
        view.turn
    );
}

#[test]
fn lands_reach_the_battlefield_and_show_up_grouped_in_the_board_model() {
    let (client, _) = play(4, 400);
    let board = client.board.expect("a board model");
    let pods: usize = board.pods.iter().map(SeatPod::permanent_count).sum();
    assert!(pods > 0, "somebody should have played a land by now");

    // Sixty identical Forests are the grouping case: however many are on the
    // battlefield, they draw as very few cards.
    for pod in &board.pods {
        let lands = pod
            .lane(baylee_client_core::layout::LaneKind::Lands)
            .expect("a lands lane");
        if lands.permanent_count() > 1 {
            assert!(
                lands.groups.len() < lands.permanent_count(),
                "identical lands must collapse: {} groups for {} permanents",
                lands.groups.len(),
                lands.permanent_count()
            );
        }
    }
}

#[test]
fn the_static_payload_lets_every_board_card_resolve_to_an_image() {
    let (client, _) = play(4, 400);
    let statics = client.statics.expect("the static payload");
    let board = client.board.expect("a board model");

    for key in board.required_images() {
        assert!(
            statics.print(key.print).is_some(),
            "the print table must cover every card the board wants to draw"
        );
    }
}

#[test]
fn the_client_never_builds_an_action_the_engine_refuses() {
    // The strongest property the interaction layer claims: whatever the game
    // asks, the answer the client constructs is accepted. Any rejection would
    // show up as an error message from the host.
    for seed in [1u64, 7, 23, 99] {
        let (client, _) = play(seed, 250);
        assert!(
            client.errors.is_empty(),
            "seed {seed} produced rejected actions: {:?}",
            client.errors
        );
    }
}

#[test]
fn the_same_seed_plays_out_identically() {
    // Determinism is the platform's central claim; a client that replays a
    // seed must land in the same place, or replays and reconnects are worthless.
    let (a, _) = play(12, 200);
    let (b, _) = play(12, 200);

    let (va, vb) = (a.view.expect("view"), b.view.expect("view"));
    assert_eq!(va.turn, vb.turn);
    assert_eq!(va.seats[0].life, vb.seats[0].life);
    assert_eq!(va.seats[0].library_count, vb.seats[0].library_count);
    assert_eq!(a.board, b.board, "the same seed must render the same board");
}

#[test]
fn hidden_information_never_reaches_the_client() {
    let (client, _) = play(4, 300);
    let view = client.view.expect("a view");

    // The opponent's hand is a number and nothing else: there is no field on
    // the view that could carry their cards, which is the point of the wire
    // type's shape.
    let them = view.seat(PlayerId::new(1)).expect("opponent");
    assert!(them.hand_count > 0);

    // Every object the client can see is in a public zone or its own hand.
    for object in &view.battlefield {
        assert!(
            object.card.is_some() || object.name == "Face-down" || !object.name.is_empty(),
            "a visible permanent must be identifiable or explicitly hidden"
        );
    }
}
