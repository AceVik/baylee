//! The layer's numbers that are prices (`tactics::number`), asked at real
//! tables: an X paid in life is sized by what it buys and never paid into
//! the other side's swing; an X of mana whose effect the layer does not read
//! stays whole.
use super::*;
use baylee_core::ids::PrintRef;
use baylee_core::preset::{AIProfile, DeckEntry, FormatId};
use baylee_view::Phase;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

/// One side of a table: life, hand and battlefield.
struct Side<'a> {
    life: i32,
    hand: &'a [&'a str],
    board: &'a [&'a str],
}

/// A table in the seat's first main phase.
fn table(me: &Side<'_>, them: &Side<'_>) -> baylee_gamehost::Session {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    for (seat, side) in preset.seats.iter_mut().zip([me, them]) {
        seat.deck = vec![entry("Swamp"); 32];
        seat.starting_life = Some(side.life);
        seat.starting_hand = Some(side.hand.iter().map(|name| entry(name)).collect());
        seat.starting_battlefield = side.board.iter().map(|name| entry(name)).collect();
        seat.commanders.clear();
        seat.sideboard.clear();
    }
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    session.take_over(ME);
    session.take_over(THEM);
    for _ in 0..64 {
        let view = session.observe_view(ME);
        if view.phase == Phase::FirstMain && view.active == ME {
            return session;
        }
        let player = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        session.act(player, action).unwrap();
    }
    panic!("the seat's main phase not reached");
}

/// Casts `spell` from the seat's hand up to its X question.
fn cast_to_x(session: &mut baylee_gamehost::Session, spell: &str) -> (Pending, PlayerView) {
    let card = session
        .observe_view(ME)
        .hand
        .iter()
        .find(|card| card.name == spell)
        .unwrap()
        .id;
    session.act(ME, PlayerAction::CastSpell { card }).unwrap();
    for _ in 0..4 {
        let (pending, view) = session.view_for_with_context(ME).unwrap();
        match &pending {
            Pending::ChooseNumber {
                reason: baylee_engine::choice::NumberPrompt::X,
                ..
            } => return (pending, view),
            Pending::ChooseTargets { options, .. } => {
                let objects = vec![options[0]];
                session
                    .act(
                        ME,
                        PlayerAction::ChooseTargets {
                            objects,
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            other => panic!("{spell} asks {other:?} before X"),
        }
    }
    panic!("{spell} never asks X");
}

/// What the seat answers to the X question, as `--choice-guard
/// tactical-scalar` plays it, when the net's own number is `scalar`.
fn layer_x(pending: &Pending, view: &PlayerView, scalar: f32) -> u32 {
    let state = encode_state4(view, Some(pending), None, &[]);
    let cands = encode_candidates4(&state, view, pending);
    let mut guard = crate::netplay4::ChoiceGuard4::default();
    guard.tactical.scalar = true;
    let mut rng = Rng::new(7);
    let (action, ..) = PolicySampler4 {
        tactician: None,
        distill: None,
        mode: PolicyMode4::Raw,
        temperature: 0.0,
        rng: &mut rng,
        active_source: None,
        guard: &mut guard,
        rules: &mut crate::netplay4::TacticRules4::default(),
    }
    .select(
        &cands,
        view,
        pending,
        PolicyOutputs4 {
            embeddings: None,
            cand_logits: &[],
            pair_logits: &[],
            scalar_pred: scalar,
            value: 0.5,
        },
    )
    .unwrap();
    match action {
        PlayerAction::ChooseNumber(x) => x,
        other => panic!("a number, not {other:?}"),
    }
}

const SWAMPS: [&str; 4] = ["Swamp", "Swamp", "Swamp", "Swamp"];

#[test]
fn a_life_x_sweep_pays_what_ends_their_creatures_and_no_more() {
    let mut session = table(
        &Side {
            life: 20,
            hand: &["Toxic Deluge"],
            board: &SWAMPS,
        },
        &Side {
            life: 20,
            hand: &[],
            board: &["Hill Giant", "Hill Giant"],
        },
    );
    let (pending, view) = cast_to_x(&mut session, "Toxic Deluge");
    // The old layer paid all but one life, 19, whatever the net said.
    assert_eq!(layer_x(&pending, &view, 1.0), 3);
}

#[test]
fn a_life_x_sweep_into_an_empty_board_pays_nothing() {
    let mut session = table(
        &Side {
            life: 20,
            hand: &["Toxic Deluge"],
            board: &["Swamp", "Swamp", "Swamp", "Grizzly Bears"],
        },
        &Side {
            life: 20,
            hand: &[],
            board: &[],
        },
    );
    let (pending, view) = cast_to_x(&mut session, "Toxic Deluge");
    assert_eq!(layer_x(&pending, &view, 1.0), 0);
}

#[test]
fn a_life_x_sweep_spares_the_seats_bigger_creature() {
    // X = 2 ends both their Bears and keeps the seat's Giant; X = 3 would
    // also end the Giant, for a life more.
    let mut session = table(
        &Side {
            life: 20,
            hand: &["Toxic Deluge"],
            board: &["Swamp", "Swamp", "Swamp", "Hill Giant"],
        },
        &Side {
            life: 20,
            hand: &[],
            board: &["Grizzly Bears", "Grizzly Bears"],
        },
    );
    let (pending, view) = cast_to_x(&mut session, "Toxic Deluge");
    assert_eq!(layer_x(&pending, &view, 1.0), 2);
}

#[test]
fn a_life_x_is_never_paid_into_their_swing() {
    // At 6 life against a 6/4 and a 2/2: X = 2 leaves 4 life to a Wurm's 6;
    // X = 4 ends both and leaves 2 life to nothing.
    let mut session = table(
        &Side {
            life: 6,
            hand: &["Toxic Deluge"],
            board: &SWAMPS,
        },
        &Side {
            life: 20,
            hand: &[],
            board: &["Craw Wurm", "Grizzly Bears"],
        },
    );
    let (pending, view) = cast_to_x(&mut session, "Toxic Deluge");
    assert_eq!(layer_x(&pending, &view, 0.0), 4);
}

#[test]
fn a_life_x_the_layer_cannot_read_takes_the_nets_number_within_the_safe_life() {
    // Hatred's +X/+0 is not a sweep: the net's number stands, capped by the
    // life the seat can spare against their Giant's swing, with nothing to
    // block it (10 − 3 − 1 = 6).
    let hatred = || Side {
        life: 10,
        hand: &["Hatred"],
        board: &["Swamp", "Swamp", "Swamp", "Swamp", "Swamp"],
    };
    let giant = Side {
        life: 20,
        hand: &[],
        board: &["Hill Giant"],
    };
    let mut session = table(&hatred(), &giant);
    let (pending, view) = cast_to_x(&mut session, "Hatred");
    assert_eq!(layer_x(&pending, &view, 1.0), 6);
    let mut session = table(&hatred(), &giant);
    let (pending, view) = cast_to_x(&mut session, "Hatred");
    assert_eq!(layer_x(&pending, &view, 0.0), 0);
}

#[test]
fn a_mana_x_sweep_that_hits_the_seat_stops_short_of_killing_it() {
    // Earthquake at 3 life: X = 2 ends their Bears and leaves the seat at 1;
    // the whole X (4) would kill the seat.
    let mut session = table(
        &Side {
            life: 3,
            hand: &["Earthquake"],
            board: &["Mountain", "Mountain", "Mountain", "Mountain", "Mountain"],
        },
        &Side {
            life: 20,
            hand: &[],
            board: &["Grizzly Bears"],
        },
    );
    let (pending, view) = cast_to_x(&mut session, "Earthquake");
    assert_eq!(layer_x(&pending, &view, 1.0), 2);
}

#[test]
fn a_mana_x_the_layer_does_not_read_stays_whole() {
    let mut session = table(
        &Side {
            life: 20,
            hand: &["Fireball"],
            board: &["Mountain", "Mountain", "Mountain", "Mountain", "Mountain"],
        },
        &Side {
            life: 20,
            hand: &[],
            board: &["Grizzly Bears"],
        },
    );
    let (pending, view) = cast_to_x(&mut session, "Fireball");
    assert_eq!(layer_x(&pending, &view, 0.0), 4);
}

#[test]
fn mana_spent_on_prevention_stops_at_the_damage_it_prevents() {
    let session = table(
        &Side {
            life: 20,
            hand: &[],
            board: &SWAMPS,
        },
        &Side {
            life: 20,
            hand: &[],
            board: &[],
        },
    );
    let view = session.observe_view_with_context(ME);
    let pending = Pending::ChooseNumber {
        player: ME,
        min: 0,
        max: 5,
        reason: baylee_engine::choice::NumberPrompt::ManaPayment {
            preventable_damage: 2,
        },
    };
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let cands = encode_candidates4(&state, &view, &pending);
    assert_eq!(layer_number(&cands, &view, &pending, 1.0), 2);
}
