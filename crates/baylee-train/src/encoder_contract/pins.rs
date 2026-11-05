//! Older datasets stay valid: every output of the four encoders on two
//! scripted games, hashed and pinned.
//!
//! Each game is played once per encoder. What convert4 and the runtime read
//! from an encoder — each seat's passive state when its memory changes, and
//! per decision the state, the candidates, the teacher target of the scripted
//! answer and its decoding (and for Encoders 7 to 9 the announce-before-payment
//! menu) — goes into one SHA-256 as `serde_json` bytes, which print an `f32`
//! exactly and the same way on every platform.
//!
//! The digests were taken at 6934c1c2c, before the encoders learnt PR #410's
//! foretell action; neither game asks anything #410 added. A change that means
//! to alter an older encoding is a new encoder version, not a new digest here.
//! If a digest moves only because an engine or card change altered the game
//! itself, take the new one from the parent of your change, never from your
//! own build. `BAYLEE_PRINT_ENCODER_PINS=1` prints them.

use std::fmt::Write as _;

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_view::{Phase, Step};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{Encoder, P0, P1, Seat, Table};
use crate::features4::{SeatKnowledge4, decode_action4, encode_teacher_action4};
use crate::features7::encode_candidates7;

/// The answer to each question of a scripted game, and `None` once it has
/// gone far enough.
type Script = fn(&Table, &Pending) -> Option<PlayerAction>;

fn feed(hasher: &mut Sha256, value: &impl Serialize) {
    hasher.update(serde_json::to_vec(value).expect("encodings serialize"));
    hasher.update(b"\n");
}

/// Plays `script` on a fresh `table()` and hashes everything `encoder` writes
/// on the way.
fn digest(encoder: Encoder, table: fn() -> Table, script: Script) -> (String, usize) {
    let mut table = table();
    let mut knowledge = [P0, P1].map(|seat| SeatKnowledge4::from_preset(&table.preset, seat));
    let mut hasher = Sha256::new();
    let mut decisions = 0;
    while let Some(action) = script(&table, table.session.pending()) {
        let seat = table.session.pending().asked().expect("a question is open");
        let (pending, view) = encoder.view_for(&table.session, seat);
        // Every seat observes the table before the answer, the deciding seat
        // through its own deciding view, as convert4 replays a game.
        for (other, memory) in [P0, P1].into_iter().zip(&mut knowledge) {
            let observed = if other == seat {
                view.clone()
            } else {
                encoder.observe(&table.session, other)
            };
            if encoder.remember(memory, &observed) {
                feed(&mut hasher, &encoder.state(&observed, None, Some(memory)));
            }
        }
        let memory = &knowledge[usize::from(seat.get())];
        let (state, candidates) = encoder.encode(&view, &pending, Some(memory));
        let target = encode_teacher_action4(&candidates, &pending, &action);
        let decoded = target
            .as_ref()
            .and_then(|target| decode_action4(&candidates, &view, &pending, target));
        feed(&mut hasher, &(&state, &candidates, &target, &decoded));
        if encoder != Encoder::V4 {
            let announced =
                encode_candidates7(&state, &view, &pending).map_err(|limit| limit.required);
            feed(&mut hasher, &announced);
        }
        table
            .session
            .act(seat, action)
            .expect("the script answers legally");
        decisions += 1;
    }
    let hex = hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("write to a String");
            hex
        });
    (hex, decisions)
}

/// Taps `seat`'s lands one at a time, then casts `spell`; `None` once it has
/// nothing to do. Mana abilities a land has by its basic type come first,
/// then printed ones (Wastes).
fn tap_then_cast(
    table: &Table,
    pending: &Pending,
    seat: PlayerId,
    spell: &str,
) -> Option<PlayerAction> {
    let Pending::Priority { player, legal } = pending else {
        return None;
    };
    if *player != seat {
        return None;
    }
    let card = table
        .session
        .observe_view(seat)
        .hand
        .iter()
        .find(|card| card.name == spell)
        .map(|card| card.id)?;
    if legal.castable.contains(&card) {
        return Some(PlayerAction::CastSpell { card });
    }
    if let Some(&source) = legal.mana_abilities.first() {
        return Some(PlayerAction::ActivateManaAbility { source });
    }
    legal
        .abilities
        .first()
        .map(|&(source, ability_index)| PlayerAction::ActivateAbility {
            source,
            ability_index,
        })
}

/// Fireball for X = 1 at the other player on its caster's first main phase,
/// then everything else answered by default to the end of turn 1.
fn fireball(table: &Table, pending: &Pending) -> Option<PlayerAction> {
    if table.session.state().turn.number > 1 {
        return None;
    }
    let view = table.session.observe_view(P0);
    if view.phase == Phase::FirstMain
        && view.stack.is_empty()
        && let Some(action) = tap_then_cast(table, pending, P0, "Fireball")
    {
        return Some(action);
    }
    match pending {
        Pending::ChooseCastMode { .. } => Some(PlayerAction::ChooseMode(0)),
        Pending::ChooseNumber { .. } => Some(PlayerAction::ChooseNumber(1)),
        Pending::ChooseTargets { .. } => Some(PlayerAction::ChooseTargets {
            objects: Vec::new(),
            players: vec![P1],
        }),
        _ => timeout_answer(pending),
    }
}

fn fireball_table() -> Table {
    Table::new(
        12,
        "Forest",
        [
            Seat {
                hand: &["Fireball"],
                battlefield: &["Mountain", "Wastes", "Wastes"],
            },
            Seat {
                hand: &[],
                battlefield: &[],
            },
        ],
    )
}

/// Every creature attacks the other player, every blocker blocks the next
/// attacker, and the attacker's Giant Growth goes on its first attacker once
/// blocks are declared; everything else answered by default to the end of
/// turn 3.
fn combat(table: &Table, pending: &Pending) -> Option<PlayerAction> {
    if table.session.state().turn.number > 3 {
        return None;
    }
    match pending {
        Pending::ChooseAttackers {
            attackers,
            defenders,
            ..
        } => {
            let defender = *defenders.first()?;
            Some(PlayerAction::DeclareAttackers {
                attackers: attackers.iter().map(|&id| (id, defender)).collect(),
            })
        }
        Pending::ChooseBlockers { blockers, .. } => {
            let mut declared: Vec<(_, _)> = Vec::new();
            for option in blockers {
                if let Some(&attacker) = option
                    .attackers
                    .iter()
                    .find(|attacker| !declared.iter().any(|(_, taken)| taken == *attacker))
                {
                    declared.push((option.blocker, attacker));
                }
            }
            Some(PlayerAction::DeclareBlockers { blockers: declared })
        }
        Pending::ChooseTargets { options, .. } => Some(PlayerAction::ChooseTargets {
            objects: options.first().copied().into_iter().collect(),
            players: Vec::new(),
        }),
        Pending::Priority { player, .. }
            if *player == P0
                && table.session.observe_view(P0).step == Step::DeclareBlockers
                && table.session.observe_view(P0).stack.is_empty() =>
        {
            tap_then_cast(table, pending, P0, "Giant Growth").or_else(|| timeout_answer(pending))
        }
        _ => timeout_answer(pending),
    }
}

fn combat_table() -> Table {
    Table::new(
        7,
        "Forest",
        [
            Seat {
                hand: &["Giant Growth"],
                battlefield: &["Grizzly Bears", "Grizzly Bears", "Forest"],
            },
            Seat {
                hand: &["Giant Growth"],
                battlefield: &["Grizzly Bears", "Llanowar Elves", "Forest"],
            },
        ],
    )
}

/// A scripted game: the table it starts from and its answers.
struct Game {
    name: &'static str,
    table: fn() -> Table,
    script: Script,
}

const GAMES: [Game; 2] = [
    Game {
        name: "fireball",
        table: fireball_table,
        script: fireball,
    },
    Game {
        name: "combat",
        table: combat_table,
        script: combat,
    },
];

/// Per encoder, the digest of each of [`GAMES`], from 6934c1c2c.
const PINNED: [(Encoder, [&str; 2]); 4] = [
    (
        Encoder::V4,
        [
            "e2682abe4b3eb56347f2f41eb35aa2010ede2b15022b2752e8989b436fc4bea1",
            "b9fe402b7bfda9831257ae444acac5188f3e33fd3717622dc4c72440b6a62a81",
        ],
    ),
    (
        Encoder::V7,
        [
            "a496379eff93693a5711c07c378c767cec7ef3dd446a28ba2a05b4ac93ba7bb3",
            "b5288f526f78104faed5ebe6b2fb75dfab07357330c4a40126836235842b5da9",
        ],
    ),
    (
        Encoder::V8,
        [
            "a37580430ea0c3398c804d1e19a3a5cc887e5c0bddfac39baedd8649b3058648",
            "2c8588f82241f1c175ba57d88074705f5fe6a15c8947f64de040c39f9a58b5f6",
        ],
    ),
    (
        Encoder::V9,
        [
            "f4ec25377879cc3f0858f5991212327f293d8be7556003600f7477e246b27548",
            "a931690c161fdc7d7fa9f18c1db25df86d50b999d1bde5f216197042457a8923",
        ],
    ),
];

#[test]
fn every_encoder_writes_the_scripted_games_as_it_did_before_410() {
    assert_eq!(
        PINNED.map(|(encoder, _)| encoder),
        Encoder::ALL,
        "every encoder is pinned"
    );
    let print = std::env::var_os("BAYLEE_PRINT_ENCODER_PINS").is_some();
    let mut moved = Vec::new();
    for (encoder, pins) in PINNED {
        for (
            Game {
                name: game,
                table,
                script,
            },
            pin,
        ) in GAMES.into_iter().zip(pins)
        {
            let (actual, decisions) = digest(encoder, table, script);
            if print {
                eprintln!("{encoder:?} {game}: {actual} over {decisions} decisions");
            }
            assert!(
                decisions > 10,
                "{encoder:?} {game}: only {decisions} decisions"
            );
            if actual != pin {
                moved.push(format!("{encoder:?} {game}: {actual}"));
            }
        }
    }
    assert!(moved.is_empty(), "encodings moved: {moved:#?}");
}
