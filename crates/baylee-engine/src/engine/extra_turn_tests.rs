//! Extra turns and the turn order around them (CR 500.7): "Some effects can
//! give a player extra turns. They do this by adding the turns directly
//! after the specified turn. … The most recently created turn will be taken
//! first." The normal order then resumes from the turn the extra one was
//! added after, not from the extra turn itself.
//!
//! Driven with Time Vault ("{T}: Take an extra turn after this one", and a
//! skip offer while it is tapped) and Time Walk; every skip offer on the way
//! is answered explicitly, so no turn is lost to a question nobody saw.

use super::testkit::*;
use super::*;
use crate::choice::YesNoPrompt;
use crate::event::GameEvent;
use baylee_core::generated::index;
use baylee_core::ids::CardIndex;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);
const VAULT: CardIndex = index::TIME_VAULT;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn time_walk() -> CardIndex {
    card_index("d0209d3f-3f7e-4fd5-bce5-10bce6f29c86")
}

/// The skip offer standing, if any: who is asked.
fn skip_offer(e: &Engine<RegistryLookup>) -> Option<PlayerId> {
    match e.pending() {
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::SkipTurn { .. },
            ..
        } => Some(*player),
        _ => None,
    }
}

fn vault_of(e: &Engine<RegistryLookup>, seat: PlayerId) -> ObjectId {
    on_battlefield(e, seat, VAULT).expect("the Vault is on the battlefield")
}

fn vault_tapped(e: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    let id = vault_of(e, seat);
    e.state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

/// A table with an untapped Time Vault for each seat in `vaults`, mulligans
/// kept, at p0's first main phase.
fn table(seats: usize, vaults: &[usize], p0_hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut duel = Duel::table(5007, basic_forest(), seats).hand(0, p0_hand);
    for &seat in vaults {
        duel = duel.battlefield(seat, &[VAULT]);
    }
    if !p0_hand.is_empty() {
        duel = duel.battlefield(0, &[island(), island()]);
    }
    let mut e = duel.start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for &seat in vaults {
        let id = vault_of(&e, PlayerId::new(u8::try_from(seat).unwrap()));
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .remove(Status::TAPPED);
    }
    e.refresh_offer();
    e
}

/// Passes until `seat` holds priority during `active`'s turn.
fn priority_in_turn_of(e: &mut Engine<RegistryLookup>, seat: PlayerId, active: PlayerId) {
    pass_until(e, |e| {
        e.state().turn.active == active
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    });
}

/// `seat` activates its Vault's `{T}` and the stack resolves.
fn activate_vault(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let source = vault_of(e, seat);
    e.apply(
        seat,
        PlayerAction::ActivateAbility {
            source,
            ability_index: 1,
        },
    )
    .expect("{T}: take an extra turn is offered");
    pass_until(e, stack_is_empty);
}

/// Who took each turn begun after turn `from`, in order.
fn turns_after(e: &Engine<RegistryLookup>, from: u32) -> Vec<PlayerId> {
    e.state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::TurnStarted { number, active } if number > from => Some(active),
            _ => None,
        })
        .collect()
}

/// Plays on until `count` more turns have begun, answering every skip offer
/// with `skip(asked)`. Returns who was asked, offer by offer.
fn play_turns(
    e: &mut Engine<RegistryLookup>,
    count: u32,
    mut skip: impl FnMut(PlayerId) -> bool,
) -> Vec<PlayerId> {
    let goal = e.state().turn.number + count;
    let mut asked = Vec::new();
    while e.state().turn.number < goal {
        let now = e.state().turn.number;
        pass_until(e, |e| {
            e.state().turn.number > now || skip_offer(e).is_some()
        });
        if let Some(player) = skip_offer(e) {
            asked.push(player);
            e.apply(player, PlayerAction::YesNo(skip(player))).unwrap();
        }
    }
    asked
}

/// The live report: p0 taps the Vault in p1's turn. The extra turn comes
/// right after p1's, and then p0's own normal turn, the one that follows
/// p1's in the order (CR 500.7) — before the fix the order went on from the
/// extra turn and gave p1 the next turn, so p0's normal turn was lost.
/// Every skip offer is declined; p0 is asked before both of its turns.
#[test]
fn an_extra_turn_taken_in_the_opponents_turn_is_followed_by_your_own() {
    let mut e = table(2, &[0], &[]);
    priority_in_turn_of(&mut e, P0, P1);
    let from = e.state().turn.number;
    activate_vault(&mut e, P0);
    assert_eq!(e.state().extra_turns, [P0]);

    let asked = play_turns(&mut e, 3, |_| false);
    assert_eq!(turns_after(&e, from), [P0, P0, P1], "extra, normal, p1");
    assert_eq!(asked, [P0, P0], "the tapped Vault asks before each of p0's");
    assert!(e.state().resume_after.is_none());
}

/// Time Walk in p0's own turn: the extra turn is p0's, then p1's.
#[test]
fn an_extra_turn_taken_in_your_own_turn_is_followed_by_the_next_player() {
    let mut e = table(2, &[], &[time_walk()]);
    let from = e.state().turn.number;
    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, time_walk());
    pass_until(&mut e, stack_is_empty);

    play_turns(&mut e, 3, |_| false);
    assert_eq!(turns_after(&e, from), [P0, P1, P0]);
}

/// Two extra turns made in one turn: the most recently created is taken
/// first (CR 500.7). p1 taps its Vault in p0's turn, then p0 casts Time
/// Walk: p0's extra turn, p1's extra turn, then the order resumes after
/// p0's turn with p1's normal turn.
#[test]
fn the_most_recently_created_extra_turn_is_taken_first() {
    let mut e = table(2, &[1], &[time_walk()]);
    let from = e.state().turn.number;
    priority_in_turn_of(&mut e, P1, P0);
    activate_vault(&mut e, P1);
    assert_eq!(e.state().turn.active, P0, "still p0's turn");
    assert!(
        matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
        "p0 holds priority in its main phase again"
    );
    tap_all_mana(&mut e, P0);
    cast_with_floating(&mut e, P0, time_walk());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().extra_turns, [P0, P1], "the newest at the front");

    play_turns(&mut e, 4, |_| false);
    assert_eq!(turns_after(&e, from), [P0, P1, P1, P0]);
}

/// Three seats: p0 taps the Vault in p1's turn. p0's extra turn comes after
/// p1's, then the order resumes after p1's turn: p2, p0, p1.
#[test]
fn at_three_seats_the_order_resumes_after_the_turn_that_was_current() {
    let mut e = table(3, &[0], &[]);
    priority_in_turn_of(&mut e, P0, P1);
    let from = e.state().turn.number;
    activate_vault(&mut e, P0);

    play_turns(&mut e, 4, |_| false);
    assert_eq!(turns_after(&e, from), [P0, P2, P0, P1]);
}

/// The Vault's skip while the extra turn would begin: yes spends the extra
/// turn (CR 614.10), and the order still resumes after p1's turn, so p0's
/// normal turn comes next, asked about again (the untap waits for a turn
/// that occurs, CR 614.10b). A no there takes it, and the Vault untaps.
#[test]
fn skipping_the_extra_turn_keeps_your_normal_turn() {
    let mut e = table(2, &[0], &[]);
    priority_in_turn_of(&mut e, P0, P1);
    let from = e.state().turn.number;
    activate_vault(&mut e, P0);

    let mut answers = [true, false].into_iter();
    let asked = play_turns(&mut e, 2, |_| answers.next().unwrap());
    assert_eq!(asked, [P0, P0], "the extra turn, then the normal one");
    assert_eq!(turns_after(&e, from), [P0, P1], "p0's normal turn, then p1");
    assert_eq!(
        e.state().turn.number,
        from + 2,
        "the skipped turn is no turn"
    );
    assert!(!vault_tapped(&e, P0), "the skip's untap opened the Vault");
}

/// Declining the extra turn's skip and accepting the normal turn's: p0
/// takes the extra turn, skips its normal one, and p1 goes on from there.
#[test]
fn skipping_the_normal_turn_after_an_extra_one_passes_to_the_next_player() {
    let mut e = table(2, &[0], &[]);
    priority_in_turn_of(&mut e, P0, P1);
    let from = e.state().turn.number;
    activate_vault(&mut e, P0);

    let mut answers = [false, true].into_iter();
    let asked = play_turns(&mut e, 2, |_| answers.next().unwrap());
    assert_eq!(asked, [P0, P0]);
    assert_eq!(turns_after(&e, from), [P0, P1], "the extra turn, then p1");
    assert!(!vault_tapped(&e, P0), "untapped as p1's turn began");
}
