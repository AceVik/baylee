//! Day and night (CR 731), and the untap step's second turn-based action
//! (CR 502.2) that decides when the designation flips.
//!
//! The designation itself is tested here without any card: the check reads
//! how many spells the *previous* turn's active player cast, and that count
//! can be planted on `PerTurn` mid-turn, so a whole turn boundary is
//! exercised — the snapshot in `begin_turn` and the check in `untap_step`
//! together — without a werewolf to do it with. Daybound and nightbound,
//! which need a card that turns over, live in `werewolf_tests`.

use super::testkit::*;
use super::*;
use crate::turn::DayNight;

fn plains() -> baylee_core::ids::CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

/// Sets the designation the way a card would, and says how many spells the
/// active player has cast this turn — the two inputs CR 502.2 reads.
fn set_up(engine: &mut Engine<RegistryLookup>, now: Option<DayNight>, spells_this_turn: u32) {
    let seat = PlayerId::new(0);
    let state = engine
        .dev_state_mut(seat)
        .expect("the harness trusts itself");
    state.day_night = now;
    let active = state.turn.active.get() as usize;
    state.per_turn.spells_cast[active] = spells_this_turn;
}

/// Plays on until the given turn has begun — by which point that turn's
/// untap step, and with it the CR 502.2 check, has already run.
fn play_into_turn(engine: &mut Engine<RegistryLookup>, number: u32) {
    pass_until(engine, |e| e.state().turn.number >= number);
}

fn duel(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, plains()).start();
    keep_mulligans(&mut engine);
    engine
}

/// CR 730.2c / 502.2: a game with neither designation skips the check and
/// keeps having neither. This is every game in the pool with no daybound
/// card in it, so it is the case that must cost nothing and change nothing.
#[test]
fn a_game_with_neither_designation_never_gains_one_by_itself() {
    let mut engine = duel(901);
    play_into_turn(&mut engine, 4);
    assert_eq!(
        engine.state().day_night,
        None,
        "three quiet turns gave a game a designation nothing asked for"
    );
}

/// CR 502.2, first half: day becomes night when the previous turn's active
/// player cast no spells.
#[test]
fn a_turn_with_no_spells_turns_day_into_night() {
    let mut engine = duel(902);
    set_up(&mut engine, Some(DayNight::Day), 0);
    play_into_turn(&mut engine, 2);
    assert_eq!(engine.state().day_night, Some(DayNight::Night));
}

/// CR 502.2, second half: night becomes day when they cast two or more.
#[test]
fn a_turn_with_two_spells_turns_night_into_day() {
    let mut engine = duel(903);
    set_up(&mut engine, Some(DayNight::Night), 2);
    play_into_turn(&mut engine, 2);
    assert_eq!(engine.state().day_night, Some(DayNight::Day));
}

/// The gap between the two halves, and the reason neither is written as
/// "did they cast fewer/more than one". One spell is not none and not two,
/// so it holds the designation exactly where it is — in both directions,
/// which is what the pair of assertions below is for: a check written with
/// a single comparison would pass one of them and fail the other.
#[test]
fn a_single_spell_holds_the_designation_in_both_directions() {
    let mut day = duel(904);
    set_up(&mut day, Some(DayNight::Day), 1);
    play_into_turn(&mut day, 2);
    assert_eq!(
        day.state().day_night,
        Some(DayNight::Day),
        "one spell made it night; only *no* spells do that"
    );

    let mut night = duel(905);
    set_up(&mut night, Some(DayNight::Night), 1);
    play_into_turn(&mut night, 2);
    assert_eq!(
        night.state().day_night,
        Some(DayNight::Night),
        "one spell made it day; that takes two"
    );
}

/// The count the check reads is the *previous* turn's, and it is gone by the
/// time the check could recount it: `PerTurn::reset()` runs at the top of
/// every turn, before that turn's untap step. This is the test that fails if
/// the snapshot in `begin_turn` is moved below the reset or below the swap
/// that changes the active player.
#[test]
fn the_check_reads_the_turn_that_just_ended_and_not_this_one() {
    let mut engine = duel(906);
    set_up(&mut engine, Some(DayNight::Night), 2);
    let first = engine.state().turn.active;
    play_into_turn(&mut engine, 2);
    let previous = engine.state().previous_turn.expect("turn 2 has a turn 1");
    assert_eq!(previous.active, first, "the snapshot named the wrong seat");
    assert_eq!(
        previous.spells_cast, 2,
        "the snapshot was taken after the reset"
    );
    assert_eq!(engine.state().day_night, Some(DayNight::Day));

    // And the turn after that: seat 1 cast nothing, so the same designation
    // walks straight back to night. Two flips in a row also prove the
    // journal entry is written each time rather than only on the first.
    play_into_turn(&mut engine, 3);
    assert_eq!(engine.state().day_night, Some(DayNight::Night));
    let flips = engine
        .journal()
        .entries()
        .iter()
        .filter(|e| matches!(e.event, GameEvent::DayNightChanged { .. }))
        .count();
    assert_eq!(flips, 2, "each change is one journal entry");
}

/// The first turn of the game has no previous turn to ask, so nothing is
/// snapshotted and the check has nothing to read. Without the guard the
/// snapshot would name turn 1's own active player with a count of zero, and
/// a game that started at day would be night before anyone had a turn.
#[test]
fn the_first_turn_has_no_previous_turn_and_flips_nothing() {
    let mut engine = duel(907);
    assert_eq!(engine.state().previous_turn, None);
    let seat = PlayerId::new(0);
    engine
        .dev_state_mut(seat)
        .expect("the harness trusts itself")
        .day_night = Some(DayNight::Day);
    // Still turn 1 — the untap step that ran before this had no previous
    // turn, and nothing since has begun a new one.
    assert_eq!(engine.state().turn.number, 1);
    assert_eq!(engine.state().day_night, Some(DayNight::Day));
}

/// CR 730.1: once the game has a designation it has exactly one from that
/// point forward. The field is an `Option` for the "neither" that a game
/// starts in, not for a state it can return to, and the doors are what
/// enforce that — nothing clears them.
#[test]
fn a_designation_that_is_already_set_records_nothing_and_never_clears() {
    let mut engine = duel(908);
    let seat = PlayerId::new(0);
    let state = engine
        .dev_state_mut(seat)
        .expect("the harness trusts itself");
    state.become_day();
    let after_first = state.journal.last_seq();
    state.become_day();
    assert_eq!(
        state.journal.last_seq(),
        after_first,
        "becoming the designation the game already has is not an event"
    );
    state.become_night();
    assert_eq!(state.day_night, Some(DayNight::Night));
    state.become_day();
    assert_eq!(state.day_night, Some(DayNight::Day));
}

/// The designation and the previous turn are both rules-visible inputs to
/// the next untap step, so two states that differ in either are not the same
/// state. Brent's algorithm compares `loop_signature`s, and a flip it could
/// not see would let a game that is genuinely alternating day and night be
/// reported as an endless loop.
#[test]
fn the_loop_signature_can_tell_day_from_night() {
    let mut engine = duel(909);
    let seat = PlayerId::new(0);
    let before = engine.state().loop_signature();
    engine
        .dev_state_mut(seat)
        .expect("the harness trusts itself")
        .become_day();
    let day = engine.state().loop_signature();
    engine
        .dev_state_mut(seat)
        .expect("the harness trusts itself")
        .become_night();
    let night = engine.state().loop_signature();
    assert_ne!(before, day, "neither and day hash the same");
    assert_ne!(day, night, "day and night hash the same");
}
