//! The monarch at a table of four (CR 724), where the designation is played.
//!
//! A duel cannot tell "the monarch's end step" from "the end step of the
//! player who is not the active one", nor "the active player" from "the next
//! player after the leaver": with two seats each pair is one seat. Every test
//! here seats four. The crown is planted with `set_monarch` where the card
//! that made a monarch is not the subject; `palace_jailer`'s card tests play
//! the cards.

use super::testkit::*;
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, Defender};

const SEATS: usize = 4;

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn youthful_knight() -> CardIndex {
    card_index("ef2a24f5-ce5e-4054-843a-2cae0c66318a")
}

/// Four seats, `board` on each seat's battlefield by seat, every hand kept.
fn table(seed: u64, boards: [&[CardIndex]; SEATS]) -> Engine<RegistryLookup> {
    let mut duel = Duel::table(seed, basic_forest(), SEATS);
    for (n, board) in boards.iter().enumerate() {
        duel = duel.battlefield(n, board);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    engine
}

fn crown(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    engine
        .dev_state_mut(seat(0))
        .expect("the harness trusts itself")
        .set_monarch(player);
}

fn library(engine: &Engine<RegistryLookup>, player: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(player))
        .len()
}

/// Every sourceless "Monarch" ability on the stack, as its controller and
/// the creature it carries.
fn monarch_abilities(engine: &Engine<RegistryLookup>) -> Vec<(PlayerId, Option<ObjectId>)> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter_map(|&id| engine.state().object(id))
        .filter(|o| o.ability.is_some_and(|a| a.source == ObjectId::NO_SOURCE))
        .map(|o| (o.controller, o.event_object))
        .collect()
}

fn creatures_of(
    engine: &Engine<RegistryLookup>,
    player: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == player && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Answers one question the way a table that only does what the test says
/// would: `attack` names the attacks of the active seat, nobody blocks,
/// everything else passes.
fn answer(engine: &mut Engine<RegistryLookup>, attack: &[(ObjectId, Defender)]) {
    let (player, action) = match engine.pending().clone() {
        Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
        Pending::ChooseAttackers {
            player, attackers, ..
        } => {
            let attackers = attack
                .iter()
                .copied()
                .filter(|(a, _)| attackers.contains(a))
                .collect();
            (player, PlayerAction::DeclareAttackers { attackers })
        }
        Pending::ChooseBlockers { player, .. } => {
            (player, PlayerAction::DeclareBlockers { blockers: vec![] })
        }
        other => panic!("unexpected: {other:?}"),
    };
    engine.apply(player, action).unwrap();
}

/// Answers until `until` holds, attacking as `attack` says.
#[track_caller]
fn play_until(
    engine: &mut Engine<RegistryLookup>,
    attack: &[(ObjectId, Defender)],
    until: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..400 {
        if until(engine) {
            return;
        }
        answer(engine, attack);
    }
    panic!("never reached; stopped at {:?}", engine.pending());
}

fn in_step(step: crate::turn::Step) -> impl Fn(&Engine<RegistryLookup>) -> bool {
    move |e| e.state().turn.step == step
}

/// One full round from the turn under way, a turn for each seat still in
/// the game: every card drawn while an end step was under way, as (active
/// seat, drawing seat, cards).
///
/// A draw is read off the library, before and after each answer, and is
/// counted only when the answer began and ended in one end step: the draw
/// step's draw happens as a step begins and is never in one.
fn end_step_draws_over_a_round(
    engine: &mut Engine<RegistryLookup>,
) -> Vec<(PlayerId, PlayerId, usize)> {
    let first = engine.state().turn.number;
    let playing = (0..SEATS as u8)
        .filter(|&n| !engine.state().has_left(seat(n)))
        .count() as u32;
    let mut draws = Vec::new();
    for _ in 0..800 {
        if engine.state().turn.number >= first + playing {
            return draws;
        }
        let before: Vec<usize> = (0..SEATS as u8).map(|n| library(engine, seat(n))).collect();
        let (turn, step) = (engine.state().turn.number, engine.state().turn.step);
        answer(engine, &[]);
        let after = &engine.state().turn;
        if after.number == turn
            && step == crate::turn::Step::End
            && after.step == crate::turn::Step::End
        {
            for n in 0..SEATS as u8 {
                let drawn = before[usize::from(n)].saturating_sub(library(engine, seat(n)));
                if drawn > 0 {
                    draws.push((engine.state().turn.active, seat(n), drawn));
                }
            }
        }
    }
    panic!("the round never ended");
}

/// The draw is the monarch's, at the monarch's own end step, once a round:
/// not at the three other end steps, and not for anybody else.
#[test]
fn at_a_table_of_four_the_monarch_draws_once_a_round_at_their_own_end_step() {
    let mut engine = table(7240, [&[], &[], &[], &[]]);
    crown(&mut engine, seat(2));
    let draws = end_step_draws_over_a_round(&mut engine);
    assert_eq!(draws, [(seat(2), seat(2), 1)]);
    assert_eq!(engine.state().monarch, Some(seat(2)));
}

/// The draw waits on the stack, under the monarch, and is put there once.
#[test]
fn the_end_step_draw_is_one_ability_under_the_monarch() {
    let mut engine = table(7241, [&[], &[], &[], &[]]);
    crown(&mut engine, seat(1));
    play_until(&mut engine, &[], |e| {
        e.state().turn.active == seat(1)
            && e.state().turn.step == crate::turn::Step::End
            && !e.state().zones.stack_is_empty()
    });
    assert_eq!(monarch_abilities(&engine), [(seat(1), None)]);
}

/// Combat damage to a player who is not the monarch takes nothing, and
/// combat damage to the monarch in the same step takes the crown, once.
#[test]
fn only_the_damage_dealt_to_the_monarch_takes_the_crown() {
    let elf = quiet_creature();
    let mut engine = table(7242, [&[elf, elf], &[], &[], &[]]);
    crown(&mut engine, seat(2));
    let [a, b] = creatures_of(&engine, seat(0), elf)[..] else {
        panic!("two Elves")
    };
    let attack = [
        (a, Defender::Player(seat(2))),
        (b, Defender::Player(seat(3))),
    ];
    play_until(
        &mut engine,
        &attack,
        in_step(crate::turn::Step::CombatDamage),
    );
    play_until(&mut engine, &attack, |e| !e.state().zones.stack_is_empty());
    assert_eq!(
        monarch_abilities(&engine),
        [(seat(2), Some(a))],
        "one takeover, under the monarch it triggered against, carrying the Elf that hit them"
    );
    play_until(&mut engine, &attack, |e| e.state().zones.stack_is_empty());
    assert_eq!(engine.state().monarch, Some(seat(0)));
}

/// Two creatures hit the monarch at once: the ability triggers for each
/// (CR 603.2c), and both crown their controller.
#[test]
fn two_creatures_hitting_the_monarch_trigger_twice() {
    let elf = quiet_creature();
    let mut engine = table(7243, [&[elf, elf], &[], &[], &[]]);
    crown(&mut engine, seat(3));
    let [a, b] = creatures_of(&engine, seat(0), elf)[..] else {
        panic!("two Elves")
    };
    let attack = [
        (a, Defender::Player(seat(3))),
        (b, Defender::Player(seat(3))),
    ];
    play_until(&mut engine, &attack, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamage && !e.state().zones.stack_is_empty()
    });
    let mut on_stack = monarch_abilities(&engine);
    on_stack.sort_by_key(|&(_, c)| c);
    let mut expected = vec![(seat(3), Some(a)), (seat(3), Some(b))];
    expected.sort_by_key(|&(_, c)| c);
    assert_eq!(on_stack, expected);
    play_until(&mut engine, &attack, |e| e.state().zones.stack_is_empty());
    assert_eq!(engine.state().monarch, Some(seat(0)));
}

/// First strike takes the crown in the first combat damage step. In the
/// second, the player the rest of the attack hits is no longer the monarch,
/// and nothing triggers.
#[test]
fn first_strike_takes_the_crown_and_the_second_step_finds_no_monarch_to_hit() {
    let elf = quiet_creature();
    let knight = youthful_knight();
    let mut engine = table(7244, [&[knight, elf], &[], &[], &[]]);
    crown(&mut engine, seat(1));
    let [k] = creatures_of(&engine, seat(0), knight)[..] else {
        panic!("the Knight")
    };
    let [e] = creatures_of(&engine, seat(0), elf)[..] else {
        panic!("the Elf")
    };
    let attack = [
        (k, Defender::Player(seat(1))),
        (e, Defender::Player(seat(1))),
    ];
    play_until(&mut engine, &attack, |en| {
        en.state().turn.step == crate::turn::Step::CombatDamageFirst
            && !en.state().zones.stack_is_empty()
    });
    assert_eq!(monarch_abilities(&engine), [(seat(1), Some(k))]);
    play_until(&mut engine, &attack, |en| en.state().zones.stack_is_empty());
    assert_eq!(engine.state().monarch, Some(seat(0)));
    play_until(
        &mut engine,
        &attack,
        in_step(crate::turn::Step::CombatDamage),
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "both creatures connected, the 2/1 Knight in the first step and the Elf in the second"
    );
    assert!(
        monarch_abilities(&engine).is_empty(),
        "the Elf hit a player who no longer held the crown: {:?}",
        monarch_abilities(&engine)
    );
}

/// The crown taken in combat is the new monarch's for its own end step the
/// same turn, and the old monarch draws nothing for the rest of the round.
#[test]
fn a_crown_taken_in_combat_draws_at_the_new_monarchs_end_step() {
    let elf = quiet_creature();
    let mut engine = table(7245, [&[elf], &[], &[], &[]]);
    crown(&mut engine, seat(2));
    let [a] = creatures_of(&engine, seat(0), elf)[..] else {
        panic!("the Elf")
    };
    let attack = [(a, Defender::Player(seat(2)))];
    play_until(&mut engine, &attack, |e| e.state().monarch == Some(seat(0)));
    let mut draws = Vec::new();
    let first = engine.state().turn.number;
    while engine.state().turn.number < first + SEATS as u32 {
        let before: Vec<usize> = (0..SEATS as u8)
            .map(|n| library(&engine, seat(n)))
            .collect();
        let step = engine.state().turn.step;
        answer(&mut engine, &[]);
        if step == crate::turn::Step::End && engine.state().turn.step == crate::turn::Step::End {
            for n in 0..SEATS as u8 {
                let drawn = before[usize::from(n)] - library(&engine, seat(n));
                if drawn > 0 {
                    draws.push((engine.state().turn.active, seat(n), drawn));
                }
            }
        }
    }
    assert_eq!(draws, [(seat(0), seat(0), 1)]);
}

/// The monarch dies to the combat damage that would have taken the crown.
/// The crown passes to the active player as the monarch leaves (CR 724.4),
/// and the game goes on with the attacker crowned, once.
#[test]
fn a_monarch_killed_by_the_damage_crowns_the_attacker_as_they_leave() {
    let elf = quiet_creature();
    let mut engine = Duel::table(7246, basic_forest(), SEATS)
        .battlefield(0, &[elf])
        .life(2, 1)
        .start();
    keep_mulligans(&mut engine);
    crown(&mut engine, seat(2));
    let [a] = creatures_of(&engine, seat(0), elf)[..] else {
        panic!("the Elf")
    };
    let attack = [(a, Defender::Player(seat(2)))];
    play_until(&mut engine, &attack, |e| e.state().has_left(seat(2)));
    assert_eq!(engine.state().monarch, Some(seat(0)));
    play_until(&mut engine, &attack, |e| e.state().zones.stack_is_empty());
    assert_eq!(engine.state().monarch, Some(seat(0)));
    assert!(
        monarch_abilities(&engine).is_empty(),
        "nothing is left on the stack under the player who left"
    );
}

/// The monarch leaves on their own turn: the next player in turn order who
/// is still in the game takes the crown, skipping a seat that already left.
#[test]
fn an_active_monarch_who_leaves_crowns_the_next_seat_still_playing() {
    let mut engine = table(7247, [&[], &[], &[], &[]]);
    play_until(&mut engine, &[], |e| e.state().turn.active == seat(1));
    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    crown(&mut engine, seat(1));
    engine.apply(seat(1), PlayerAction::Concede).unwrap();
    assert_eq!(
        engine.state().monarch,
        Some(seat(3)),
        "seat 2, next after the leaver, had already left"
    );
}

/// The monarch leaves on another seat's turn: the active player, not the
/// seat after the leaver.
#[test]
fn a_monarch_who_leaves_on_another_turn_crowns_the_active_player() {
    let mut engine = table(7248, [&[], &[], &[], &[]]);
    play_until(&mut engine, &[], |e| e.state().turn.active == seat(1));
    crown(&mut engine, seat(2));
    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(engine.state().monarch, Some(seat(1)));
    let draws = end_step_draws_over_a_round(&mut engine);
    assert_eq!(
        draws,
        [(seat(1), seat(1), 1)],
        "the heir draws at their own end step, and the leaver's seat is skipped"
    );
}

/// "Becomes the monarch" is an event: the journal says who, once, and says
/// nothing when the monarch is told to become what they already are.
#[test]
fn becoming_the_monarch_is_journaled_once_per_change() {
    let mut engine = table(7249, [&[], &[], &[], &[]]);
    let from = engine.journal().entries().len();
    crown(&mut engine, seat(2));
    crown(&mut engine, seat(2));
    crown(&mut engine, seat(3));
    let crowned: Vec<PlayerId> = engine.journal().entries()[from..]
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::BecameMonarch { player } => Some(player),
            _ => None,
        })
        .collect();
    assert_eq!(crowned, [seat(2), seat(3)]);
}
