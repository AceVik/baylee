//! `seen`: a digest of the public table, the same on every build.
//!
//! The engine's [`snapshot_hash`](baylee_engine::engine::Engine::snapshot_hash)
//! checks every bit of a replay, but only on the build that wrote the record:
//! any change to the engine's state, even one no player could see, moves it.
//! A replay on a later build is checked against `seen` instead: what the
//! table showed after each input, written in numbers of our own that mean
//! the same on every build (`docs/protocol.md` §"The game record").
//!
//! FNV-1a 64 over little-endian bytes, in this order, never an enum's
//! discriminant or its `Debug` text:
//!
//! 1. the turn (`u32`), the active seat (`u8`), the step in CR 500.1's order
//!    (`u8`, [`step_number`]);
//! 2. per seat in seat order: in the game (`u8`), life (`i32`), poison,
//!    hand, library and exile counts (`u32` each), the graveyard's cards in
//!    zone order (a `u32` count, then each card's `u32`);
//! 3. the battlefield as a sorted list of (card, controller `u8`, tapped
//!    `u8`, face down `u8`), a `u32` count first;
//! 4. the stack's size (`u32`).
//!
//! A card is its [`CardIndex`](baylee_core::ids::CardIndex), the append-only
//! ledger, so it means the same on every build; an object with no card (a
//! token) is [`NO_CARD`], a face-down permanent [`FACE_DOWN`] whatever it is.
//!
//! **Never change this encoding.** A golden test pins its value for a fixed
//! table; the day it has to say more, that is a new field (`seen2`) beside
//! this one, and records carrying `seen` still check against this function.

use baylee_engine::object::{GameObject, Status};
use baylee_engine::state::GameState;
use baylee_engine::turn::{Phase, Step, TurnInfo};
use baylee_engine::zone::ZoneLocation;

/// What an object with no card (a token) is written as.
pub const NO_CARD: u32 = u32::MAX;
/// What a face-down permanent is written as, whatever card it is.
pub const FACE_DOWN: u32 = u32::MAX - 1;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64, fed little-endian numbers.
struct Fnv(u64);

impl Fnv {
    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }

    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }

    /// A count, saturated: no zone holds four billion objects.
    fn count(&mut self, n: usize) {
        self.u32(u32::try_from(n).unwrap_or(u32::MAX));
    }
}

/// The step as CR 500.1 orders a turn: untap 0, upkeep 1, draw 2, the
/// precombat main phase 3, beginning of combat 4, declare attackers 5,
/// declare blockers 6, combat damage 7 (first strike's too), end of combat 8,
/// the postcombat main phase 9, end 10, cleanup 11.
#[must_use]
pub const fn step_number(turn: &TurnInfo) -> u8 {
    match turn.step {
        Step::Untap => 0,
        Step::Upkeep => 1,
        Step::Draw => 2,
        Step::Main => match turn.phase {
            Phase::SecondMain | Phase::Ending => 9,
            Phase::Beginning | Phase::FirstMain | Phase::Combat => 3,
        },
        Step::CombatBegin => 4,
        Step::DeclareAttackers => 5,
        Step::DeclareBlockers => 6,
        Step::CombatDamageFirst | Step::CombatDamage => 7,
        Step::CombatEnd => 8,
        Step::End => 10,
        Step::Cleanup => 11,
    }
}

fn card_of(obj: &GameObject) -> u32 {
    obj.card.map_or(NO_CARD, |card| card.index.get())
}

/// The count a zone is written as, saturated: no zone holds four billion
/// objects.
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What one seat shows, in the order [`seen`] writes it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PublicSeat {
    /// `1` while the seat is in the game.
    pub in_game: u8,
    pub life: i32,
    pub poison: u32,
    pub hand: u32,
    pub library: u32,
    pub exile: u32,
    /// The graveyard's cards in zone order.
    pub graveyard: Vec<u32>,
}

/// The public table [`seen`] digests, as the numbers the module docs name,
/// read off a state ([`Self::of`]) and digested ([`Self::digest`]) apart.
///
/// Apart so the encoding can be pinned on a fixed table: the golden tests
/// digest tables captured once (`seen/table-*.json`), which no change to
/// the rules or to the house's play can move, where they used to replay a
/// recorded game on the current engine and lost their table the day a rule
/// was fixed (`seen/README.md`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PublicTable {
    pub turn: u32,
    pub active: u8,
    /// The step in CR 500.1's order ([`step_number`]).
    pub step: u8,
    pub seats: Vec<PublicSeat>,
    /// `(card, controller, tapped, face down)` of every permanent, sorted.
    pub battlefield: Vec<(u32, u8, u8, u8)>,
    pub stack: u32,
}

impl PublicTable {
    /// The public table of `state`.
    #[must_use]
    pub fn of(state: &GameState) -> Self {
        let seats = state
            .players
            .iter()
            .map(|player| {
                let seat = player.id;
                PublicSeat {
                    in_game: u8::from(!state.has_left(seat)),
                    life: player.life,
                    poison: u32::from(player.poison),
                    hand: count(state.zones.list(ZoneLocation::Hand(seat)).len()),
                    library: count(state.zones.list(ZoneLocation::Library(seat)).len()),
                    exile: count(state.zones.list(ZoneLocation::Exile(seat)).len()),
                    graveyard: state
                        .zones
                        .list(ZoneLocation::Graveyard(seat))
                        .iter()
                        .map(|&id| state.object(id).map_or(NO_CARD, card_of))
                        .collect(),
                }
            })
            .collect();
        let mut battlefield: Vec<(u32, u8, u8, u8)> = state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter_map(|&id| state.object(id))
            .map(|obj| {
                let face_down = obj.status.contains(Status::FACE_DOWN);
                (
                    if face_down { FACE_DOWN } else { card_of(obj) },
                    obj.controller.get(),
                    u8::from(obj.status.contains(Status::TAPPED)),
                    u8::from(face_down),
                )
            })
            .collect();
        battlefield.sort_unstable();
        Self {
            turn: state.turn.number,
            active: state.turn.active.get(),
            step: step_number(&state.turn),
            seats,
            battlefield,
            stack: count(state.zones.list(ZoneLocation::Stack).len()),
        }
    }

    /// Its digest, in the encoding the module docs fix.
    #[must_use]
    pub fn digest(&self) -> u64 {
        let mut h = Fnv(FNV_OFFSET);
        h.u32(self.turn);
        h.u8(self.active);
        h.u8(self.step);
        for seat in &self.seats {
            h.u8(seat.in_game);
            h.i32(seat.life);
            h.u32(seat.poison);
            h.u32(seat.hand);
            h.u32(seat.library);
            h.u32(seat.exile);
            h.count(seat.graveyard.len());
            for &card in &seat.graveyard {
                h.u32(card);
            }
        }
        h.count(self.battlefield.len());
        for &(card, controller, tapped, face_down) in &self.battlefield {
            h.u32(card);
            h.u8(controller);
            h.u8(tapped);
            h.u8(face_down);
        }
        h.u32(self.stack);
        h.0
    }
}

/// The public table's digest (see the module docs).
#[must_use]
pub fn seen(state: &GameState) -> u64 {
    PublicTable::of(state).digest()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a 64's published test vectors: the hash is the hash.
    #[test]
    fn fnv_is_fnv_1a_64() {
        let mut empty = Fnv(FNV_OFFSET);
        empty.bytes(b"");
        assert_eq!(empty.0, 0xcbf2_9ce4_8422_2325);
        let mut a = Fnv(FNV_OFFSET);
        a.bytes(b"a");
        assert_eq!(a.0, 0xaf63_dc4c_8601_ec8c);
        let mut foobar = Fnv(FNV_OFFSET);
        foobar.bytes(b"foobar");
        assert_eq!(foobar.0, 0x8594_4171_f739_67e8);
    }

    use super::super::tests::{encode, lines};
    use super::super::{Line, Replayed, hex, replay_exact};
    use baylee_core::ids::PlayerId;

    // The golden tables: what `seen` read after the 40th input of the
    // acceptance game at seed 3 and at its end, as plain numbers
    // (`PublicTable`), captured on the engine that played the original record
    // (origin/main 49c9a4d8f wrote it; captured on 5819631ae, which replayed
    // it exactly). Neither a fixed rule nor a change to the house's play can
    // move them, so the encoding they pin stands on its own
    // (`seen/README.md`). Both digests are the originals.
    const TABLE_40: &str = include_str!("seen/table-40.json");
    const TABLE_END: &str = include_str!("seen/table-end.json");

    fn table(json: &str) -> PublicTable {
        serde_json::from_str(json).expect("a captured table")
    }

    // A game for the tests that change a table and watch the digest move or
    // stay: the house playing seed 3 (`record::tests::played`), replayed
    // exactly, every engine hash checked (its header names the build `test`).
    // It pins no value, and is captured again
    // (`capture_acceptance_record`) when a fixed rule makes it diverge.
    const ACCEPTANCE_RECORD: &[u8] = include_bytes!("seen/acceptance-seed3.jsonl");

    /// The acceptance table at seed 3, played, and cut after its first 40
    /// inputs: a mid-game table with cards on the battlefield, in the
    /// graveyards and in the libraries. Returns the cut record, as the
    /// recorder wrote it, and the replay of it.
    fn forty_inputs() -> (Vec<Line>, Replayed) {
        let written = lines(ACCEPTANCE_RECORD);
        let mut inputs = 0;
        let mut cut = Vec::new();
        for line in written {
            let is_input = matches!(line, Line::Input { .. });
            cut.push(line);
            if is_input {
                inputs += 1;
                if inputs == 40 {
                    break;
                }
            }
        }
        assert_eq!(inputs, 40, "the game ran for fewer than 40 inputs");
        let replayed = replay_exact(&encode(&cut)).expect("the cut record replays");
        assert_eq!(replayed.inputs, 40);
        (cut, replayed)
    }

    /// `seen` of the table after the 40th input of the acceptance game at
    /// seed 3. If this is red, the encoding was edited. That is never the
    /// fix for it: records already written carry `seen` values made by the
    /// old encoding and are checked against this function on every later
    /// build. Add a new field (`seen2`) with the new encoding and leave
    /// this literal, and `seen`, as they are.
    const GOLDEN: u64 = 0x30b1_7ab8_dd2e_7087;

    #[test]
    fn the_digest_of_a_fixed_table_never_changes() {
        let fixed = table(TABLE_40);
        assert!(
            fixed.battlefield.len() >= 2,
            "a golden table without permanents pins nothing of the battlefield's encoding"
        );
        let seen_now = fixed.digest();
        assert_eq!(
            seen_now, GOLDEN,
            "seen moved to {seen_now:#018x}: see the note on GOLDEN, do not edit the literal"
        );
        // As the original record wrote it beside its 40th input.
        assert_eq!(hex(GOLDEN), "30b17ab8dd2e7087");
        assert_eq!(hex(GOLDEN), format!("{GOLDEN:016x}"));
    }

    /// `seen` of a state is the digest of the table read off it, and a
    /// table read back from JSON is the table written: what the golden
    /// tables stand for.
    #[test]
    fn a_state_s_digest_is_its_table_s() {
        for replayed in vec![forty_inputs().1, finished()].into_boxed_slice() {
            let state = replayed.engine.state();
            let read = PublicTable::of(state);
            assert_eq!(read.digest(), seen(state));
            let json = serde_json::to_string(&read).expect("a table writes");
            assert_eq!(table(&json), read);
        }
    }

    /// A seat's library has two different cards: their positions, to swap.
    fn two_different_cards(state: &GameState) -> (PlayerId, usize, usize) {
        for player in &state.players {
            let library = state.zones.list(ZoneLocation::Library(player.id));
            let card_at = |at: usize| {
                state
                    .object(library[at])
                    .and_then(|o| o.card)
                    .map(|c| c.index)
            };
            for i in 1..library.len() {
                if card_at(i) != card_at(0) {
                    return (player.id, 0, i);
                }
            }
        }
        panic!("no library holds two different cards");
    }

    /// What is in a library is not on the table: the order of it, or which
    /// card is where, moves the engine's hash and not `seen`.
    #[test]
    fn a_library_reshuffled_moves_the_hash_and_not_seen() {
        let (_, replayed) = forty_inputs();
        let state = replayed.engine.state();
        let (seat, i, j) = two_different_cards(state);
        let mut shuffled = state.clone();
        shuffled
            .zones
            .list_mut(ZoneLocation::Library(seat))
            .swap(i, j);
        assert_ne!(
            shuffled.snapshot_hash(),
            state.snapshot_hash(),
            "the swap changed nothing the engine hashes, so it proves nothing"
        );
        assert_eq!(seen(&shuffled), seen(state));
    }

    /// A cloned table is the same digest: the baseline every change below
    /// is measured from.
    #[test]
    fn a_clone_is_the_same_table() {
        let (_, replayed) = forty_inputs();
        let state = replayed.engine.state();
        assert_eq!(seen(&state.clone()), seen(state));
    }

    /// The acceptance game at seed 3 played to its end: graveyards full,
    /// a card in a graveyard.
    fn finished() -> Replayed {
        replay_exact(ACCEPTANCE_RECORD).expect("the acceptance record replays")
    }

    /// `seen` of the finished game's table, pinned as [`GOLDEN`] is, for a
    /// table with cards in the graveyards, which the 40th input's has not.
    const GOLDEN_END: u64 = 0xdeb4_9d0b_74b1_5d90;

    #[test]
    fn the_digest_of_a_finished_table_never_changes() {
        let fixed = table(TABLE_END);
        assert!(
            fixed.seats.iter().any(|seat| !seat.graveyard.is_empty()),
            "a golden table with an empty graveyard pins nothing of the graveyard's encoding"
        );
        let seen_now = fixed.digest();
        assert_eq!(
            seen_now, GOLDEN_END,
            "seen moved to {seen_now:#018x}: see the note on GOLDEN, do not edit the literal"
        );
    }

    /// Writes the acceptance record again from the house playing seed 3 on
    /// this engine, for when a fixed rule makes the old one diverge; the
    /// golden tables do not depend on it. By hand:
    /// `cargo test -p baylee-gamehost --lib capture_acceptance_record -- --ignored`,
    /// then the README's SHA256.
    #[test]
    #[ignore = "rewrites the fixture; run by hand when a rule fix makes it diverge"]
    fn capture_acceptance_record() {
        // Named the build `test`, as the fixture always was: no commit of
        // the build that happened to capture it is written into it.
        let mut written = lines(&super::super::tests::played(3).take_record());
        if let Some(Line::Header { build, .. }) = written.first_mut() {
            *build = "test".to_owned();
        }
        let record = encode(&written);
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/record/seen/acceptance-seed3.jsonl"
            ),
            record,
        )
        .expect("the fixture is written");
    }

    /// Everything a player can see moves it, on a table early in the game
    /// and on one at its end.
    #[test]
    fn what_the_table_shows_moves_seen() {
        for replayed in vec![forty_inputs().1, finished()].into_boxed_slice() {
            every_public_change_moves_seen(replayed.engine.state());
        }
    }

    fn every_public_change_moves_seen(state: &GameState) {
        let before = seen(state);
        let seat = 0;
        let changed = |name: &str, edit: &dyn Fn(&mut GameState)| {
            let mut other = state.clone();
            edit(&mut other);
            assert_ne!(seen(&other), before, "{name} left seen alone");
        };

        changed("a life point", &|s| s.players[seat].life += 1);
        changed("a life point lost", &|s| s.players[seat].life -= 1);
        changed("the other seat's life", &|s| s.players[1].life += 1);
        changed("a poison counter", &|s| s.players[seat].poison += 1);
        changed("the turn", &|s| s.turn.number += 1);
        changed("the active seat", &|s| {
            s.turn.active = PlayerId::new(1 - s.turn.active.get());
        });
        changed("the step", &|s| {
            s.turn.step = if s.turn.step == Step::End {
                Step::Upkeep
            } else {
                Step::End
            };
        });

        // A card from the library to each other zone of the seat's.
        let id = PlayerId::new(0);
        for to in [
            ZoneLocation::Hand(id),
            ZoneLocation::Graveyard(id),
            ZoneLocation::Exile(id),
        ] {
            changed(&format!("a card to {to:?}"), &|s| {
                let card = s
                    .zones
                    .list_mut(ZoneLocation::Library(id))
                    .pop()
                    .expect("a library with a card");
                s.zones.list_mut(to).push(card);
            });
        }

        // A permanent tapped or untapped, turned face up or down, handed
        // to the other seat, or gone.
        let permanent = *state
            .zones
            .list(ZoneLocation::Battlefield)
            .first()
            .expect("a permanent on the table");
        let flip = |s: &mut GameState, bit: Status| {
            let status = &mut s.object_mut(permanent).expect("on the table").status;
            if status.contains(bit) {
                status.remove(bit);
            } else {
                status.insert(bit);
            }
        };
        changed("a permanent tapped or untapped", &|s| {
            flip(s, Status::TAPPED);
        });
        changed("a permanent turned face down or up", &|s| {
            flip(s, Status::FACE_DOWN);
        });
        changed("a permanent changing controller", &|s| {
            let obj = s.object_mut(permanent).expect("on the table");
            obj.controller = PlayerId::new(1 - obj.controller.get());
        });
        changed("a permanent gone", &|s| {
            s.zones
                .list_mut(ZoneLocation::Battlefield)
                .retain(|&o| o != permanent);
        });
    }

    /// The battlefield is written sorted, not in the order the zone holds
    /// it: the same permanents are the same table.
    #[test]
    fn the_order_of_the_battlefield_is_not_the_table() {
        let (_, replayed) = forty_inputs();
        let state = replayed.engine.state();
        let on_table = state.zones.list(ZoneLocation::Battlefield);
        let tuples: std::collections::BTreeSet<_> = on_table
            .iter()
            .filter_map(|&id| state.object(id))
            .map(|o| {
                (
                    card_of(o),
                    o.controller.get(),
                    o.status.contains(Status::TAPPED),
                )
            })
            .collect();
        assert!(
            tuples.len() >= 2,
            "a battlefield of one kind proves nothing"
        );
        let mut reversed = state.clone();
        reversed.zones.list_mut(ZoneLocation::Battlefield).reverse();
        assert_ne!(
            reversed.zones.list(ZoneLocation::Battlefield),
            on_table,
            "the reversal moved nothing"
        );
        assert_eq!(seen(&reversed), seen(state));
    }

    /// Every step as the module docs number it, in every phase it can be
    /// reported in. The `match`es are exhaustive, so a step or a phase added
    /// tomorrow does not compile until it is numbered here.
    #[test]
    fn every_step_has_its_number() {
        const STEPS: [Step; 12] = [
            Step::Untap,
            Step::Upkeep,
            Step::Draw,
            Step::Main,
            Step::CombatBegin,
            Step::DeclareAttackers,
            Step::DeclareBlockers,
            Step::CombatDamageFirst,
            Step::CombatDamage,
            Step::CombatEnd,
            Step::End,
            Step::Cleanup,
        ];
        const PHASES: [Phase; 5] = [
            Phase::Beginning,
            Phase::FirstMain,
            Phase::Combat,
            Phase::SecondMain,
            Phase::Ending,
        ];
        // The number of a step in any phase but the main steps' two.
        const fn fixed(step: Step) -> Option<u8> {
            match step {
                Step::Untap => Some(0),
                Step::Upkeep => Some(1),
                Step::Draw => Some(2),
                Step::Main => None,
                Step::CombatBegin => Some(4),
                Step::DeclareAttackers => Some(5),
                Step::DeclareBlockers => Some(6),
                Step::CombatDamageFirst | Step::CombatDamage => Some(7),
                Step::CombatEnd => Some(8),
                Step::End => Some(10),
                Step::Cleanup => Some(11),
            }
        }
        const fn phase_is_listed(phase: Phase) -> bool {
            match phase {
                Phase::Beginning
                | Phase::FirstMain
                | Phase::Combat
                | Phase::SecondMain
                | Phase::Ending => true,
            }
        }
        for step in STEPS {
            for phase in PHASES {
                assert!(phase_is_listed(phase));
                let turn = TurnInfo {
                    number: 1,
                    active: PlayerId::new(0),
                    phase,
                    step,
                };
                let want = match (fixed(step), phase) {
                    (Some(n), _) => n,
                    (None, Phase::FirstMain | Phase::Beginning | Phase::Combat) => 3,
                    (None, Phase::SecondMain | Phase::Ending) => 9,
                };
                assert_eq!(step_number(&turn), want, "{step:?} in {phase:?}");
            }
        }
        // The two named in the module docs, spelled out.
        let main = |phase| TurnInfo {
            number: 1,
            active: PlayerId::new(0),
            phase,
            step: Step::Main,
        };
        assert_eq!(step_number(&main(Phase::FirstMain)), 3);
        assert_eq!(step_number(&main(Phase::SecondMain)), 9);
        // Twelve steps, eleven numbers: first strike and regular damage share 7.
        let mut numbers: Vec<u8> = STEPS
            .iter()
            .map(|&step| {
                step_number(&TurnInfo {
                    number: 1,
                    active: PlayerId::new(0),
                    phase: Phase::FirstMain,
                    step,
                })
            })
            .collect();
        numbers.dedup();
        assert_eq!(numbers, [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11]);
    }

    /// Rewrites the fixture's engine `hash` values after an intentional change
    /// to what the engine hash covers, and nothing else: the record is played
    /// again line by line, every public digest (`seen`) and the finish must
    /// hold, and in each line only the 16-hex `hash` is replaced, so actions,
    /// metadata and `seen` stay byte for byte (`seen/README.md`). By hand:
    /// `cargo test -p baylee-gamehost --lib refresh_fixture_hashes -- --ignored`,
    /// then update the README's SHA256.
    #[test]
    #[ignore = "rewrites the fixture; run by hand after a state-hash change"]
    fn refresh_fixture_hashes() {
        use baylee_engine::engine::Engine;
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/record/seen/acceptance-seed3.jsonl"
        );
        let text = std::fs::read_to_string(path).expect("the fixture");
        let mut engine: Option<Engine<crate::session::RegistryLookup>> = None;
        let mut out = String::with_capacity(text.len());
        for raw in text.lines() {
            let line: Line = serde_json::from_str(raw).expect("a record line");
            let (old, recorded_seen) = match line {
                Line::Header {
                    preset, hash, seen, ..
                } => {
                    engine = Some(
                        Engine::new(&preset, crate::session::RegistryLookup)
                            .expect("the preset builds"),
                    );
                    (Some(hash), seen)
                }
                Line::Input {
                    n,
                    seat,
                    action,
                    hash,
                    seen,
                    ..
                } => {
                    engine
                        .as_mut()
                        .expect("a header first")
                        .apply(PlayerId::new(seat), action)
                        .unwrap_or_else(|e| panic!("input {n} refused: {e:?}"));
                    (Some(hash), seen)
                }
                Line::End { .. } => {
                    assert!(matches!(
                        engine.as_ref().expect("a header first").pending(),
                        baylee_engine::choice::Pending::GameOver(_)
                    ));
                    (None, None)
                }
                Line::Chair { .. } | Line::DeclaredMind { .. } => (None, None),
            };
            let mut fresh = raw.to_string();
            if let Some(old) = old {
                let state = engine.as_ref().expect("a header first");
                if let Some(expected) = recorded_seen {
                    assert_eq!(hex(seen(state.state())), expected, "public digest of {raw}");
                }
                let from = format!("\"hash\":\"{old}\"");
                assert_eq!(raw.matches(&from).count(), 1, "one hash in {raw}");
                fresh = raw.replace(
                    &from,
                    &format!("\"hash\":\"{}\"", hex(state.snapshot_hash())),
                );
            }
            out.push_str(&fresh);
            out.push('\n');
        }
        std::fs::write(path, out).expect("the fixture is written");
    }
}
