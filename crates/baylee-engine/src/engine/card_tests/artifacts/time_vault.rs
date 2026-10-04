//! Time Vault: "If you would begin your turn while this artifact is tapped,
//! you may skip that turn instead. If you do, untap this artifact."
//!
//! A skip is a replacement effect (CR 614.10), asked as the turn would
//! begin, and the untap is "the first thing that happens during the next
//! step, phase, or turn to actually occur" (CR 614.10b).
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::YesNoPrompt;
use crate::event::{Cause, GameEvent};
use baylee_core::generated::index;
use baylee_core::ids::AbilityRef;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);
const VAULT: CardIndex = index::TIME_VAULT;

/// Seats a game with `vaults` Time Vaults on `seat`'s battlefield, every
/// one of them tapped, and walks to p0's first main phase.
fn table(seats: usize, seat: usize, vaults: usize) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let mut e = Duel::table(6140, forest(), seats)
        .battlefield(seat, &vec![VAULT; vaults])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let owner = PlayerId::new(u8::try_from(seat).unwrap());
    let ids: Vec<ObjectId> = e
        .state()
        .battlefield_view()
        .into_iter()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.controller == owner && o.card.map(|c| c.index) == Some(VAULT))
        })
        .collect();
    assert_eq!(ids.len(), vaults, "every Vault is on the battlefield");
    for id in &ids {
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(*id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }
    e.refresh_offer();
    (e, ids)
}

fn skip_offer(e: &Engine<RegistryLookup>) -> Option<(PlayerId, ObjectId)> {
    match e.pending() {
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::SkipTurn { source },
            ..
        } => Some((*player, *source)),
        _ => None,
    }
}

fn to_skip_offer(e: &mut Engine<RegistryLookup>) -> (PlayerId, ObjectId) {
    pass_until(e, |e| skip_offer(e).is_some());
    skip_offer(e).unwrap()
}

fn untapped_by_effect(e: &Engine<RegistryLookup>, id: ObjectId, from: u64) -> Option<u64> {
    e.state()
        .journal
        .entries()
        .iter()
        .filter(|entry| entry.seq > from)
        .find(|entry| {
            matches!(
                entry.event,
                GameEvent::ObjectUntapped { object, cause: Cause::Effect } if object == id
            )
        })
        .map(|entry| entry.seq)
}

fn turn_started_seq(e: &Engine<RegistryLookup>, number: u32, from: u64) -> Option<u64> {
    e.state()
        .journal
        .entries()
        .iter()
        .filter(|entry| entry.seq > from)
        .find(
            |entry| matches!(entry.event, GameEvent::TurnStarted { number: n, .. } if n == number),
        )
        .map(|entry| entry.seq)
}

/// The activated half, with the skip it now meets: cast, the Vault enters
/// tapped; untapped by the harness, `{T}` gives p0 the next turn too. As
/// that extra turn would begin the Vault is tapped again, so p0 is asked,
/// and a no takes the extra turn with the Vault still tapped.
#[test]
fn time_vault_enters_tapped_and_takes_an_extra_turn() {
    let vault_card = card_index("99d4d99d-cf56-45aa-aa39-a250695612f2");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[vault_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, vault_card);
    pass_until(&mut engine, stack_is_empty);
    let vault = on_battlefield(&engine, P0, vault_card).expect("it resolved");
    assert!(is_tapped(&engine, vault), "it enters tapped");

    engine
        .dev_state_mut(P0)
        .expect("the harness may set boards up")
        .object_mut(vault)
        .expect("seated")
        .status
        .remove(Status::TAPPED);
    engine.refresh_offer();
    activate(&mut engine, P0, vault_card, 1);
    pass_until(&mut engine, stack_is_empty);
    let turn = engine.state().turn.number;
    assert_eq!(to_skip_offer(&mut engine), (P0, vault));
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == turn + 1 && e.state().turn.step == crate::turn::Step::Main
    });
    assert_eq!(engine.state().turn.active, P0, "the extra turn is p0's");
    assert!(is_tapped(&engine, vault), "and it did not untap");
}

/// Yes: p0's turn never happens. The turn number does not move for it, p1
/// takes the next turn, and the Vault untaps as that turn's first event,
/// not between the two (CR 614.10b).
#[test]
fn skipping_a_turn_untaps_the_vault_as_the_next_turn_begins() {
    let (mut e, vaults) = table(2, 0, 1);
    let vault = vaults[0];
    pass_until(&mut e, |e| e.state().turn.active == P1);
    let turn = e.state().turn.number;
    let offer = to_skip_offer(&mut e);
    assert_eq!(offer, (P0, vault), "p0 is asked as their turn would begin");
    let Pending::YesNo { source, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(
        *source,
        Some(AbilityRef::new(VAULT, 2)),
        "the prompt names the Vault's replacement clause"
    );
    assert_eq!(e.state().turn.active, P1, "no turn has begun yet");
    assert!(is_tapped(&e, vault));
    let before = e.state().journal.last_seq();
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();

    assert_eq!(e.state().turn.active, P1, "the skipped turn goes to nobody");
    assert_eq!(e.state().turn.number, turn + 1, "one turn began, not two");
    assert!(!is_tapped(&e, vault), "the Vault untapped");
    let started = turn_started_seq(&e, turn + 1, before).expect("p1's turn began");
    let untapped = untapped_by_effect(&e, vault, before).expect("the untap is journaled");
    assert!(
        started < untapped,
        "the untap is the first thing in the next turn, not between turns"
    );
    assert!(e.state().skip_followups.is_empty());

    // Untapped, it offers nothing at p0's next turn: p0 simply takes it.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 || skip_offer(e).is_some()
    });
    assert_eq!(skip_offer(&e), None, "an untapped Vault offers no skip");
    assert_eq!(e.state().turn.active, P0);
}

/// No: the turn begins as it would have, the Vault stays tapped, and the
/// offer comes again at p0's next turn — each turn is its own event.
#[test]
fn declining_takes_the_turn_and_the_offer_returns_next_turn() {
    let (mut e, vaults) = table(2, 0, 1);
    let vault = vaults[0];
    pass_until(&mut e, |e| e.state().turn.active == P1);
    let turn = e.state().turn.number;
    to_skip_offer(&mut e);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(e.state().turn.active, P0);
    assert_eq!(e.state().turn.number, turn + 1);
    assert!(is_tapped(&e, vault), "doesn't untap during its untap step");
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert_eq!(to_skip_offer(&mut e), (P0, vault), "asked again");
}

/// Two tapped Vaults: one skip per event (CR 614.5). Declining the first
/// offers the second; declining both begins the turn. Taking the second
/// untaps only it.
#[test]
fn each_vault_gets_one_opportunity_at_the_turn() {
    let (mut e, vaults) = table(2, 0, 2);
    pass_until(&mut e, |e| e.state().turn.active == P1);
    let (_, first) = to_skip_offer(&mut e);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    let (asked, second) = skip_offer(&e).expect("the other Vault is offered");
    assert_eq!(asked, P0);
    assert_ne!(first, second);
    assert!(vaults.contains(&first) && vaults.contains(&second));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(skip_offer(&e), None, "neither is offered a second time");
    assert_eq!(e.state().turn.active, P0, "the turn began");

    pass_until(&mut e, |e| e.state().turn.active == P1);
    let (_, first) = to_skip_offer(&mut e);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    let (_, second) = skip_offer(&e).unwrap();
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(e.state().turn.active, P1);
    assert!(is_tapped(&e, first), "the declined Vault stays tapped");
    assert!(!is_tapped(&e, second), "the one that skipped untaps");
}

/// At a three-seat table the order goes on from the skipped turn: p1 skips,
/// so p2 — not p0 again, and not p1 — takes the next turn.
#[test]
fn turn_order_goes_on_from_the_skipped_turn() {
    let (mut e, vaults) = table(3, 1, 1);
    let turn = e.state().turn.number;
    assert_eq!(to_skip_offer(&mut e), (P1, vaults[0]));
    assert_eq!(e.state().turn.active, P0);
    e.apply(P1, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(e.state().turn.active, P2);
    assert_eq!(e.state().turn.number, turn + 1);
    assert!(!is_tapped(&e, vaults[0]));
}

/// An extra turn is a turn: skipping it spends it (CR 614.10), and the
/// normal successor follows. The Vault untaps at the start of that turn.
#[test]
fn skipping_an_extra_turn_spends_it() {
    let (mut e, vaults) = table(2, 0, 1);
    e.dev_state_mut(P0).unwrap().extra_turns.push_back(P0);
    let turn = e.state().turn.number;
    assert_eq!(to_skip_offer(&mut e), (P0, vaults[0]));
    assert_eq!(e.state().turn.active, P0, "still p0's first turn");
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert!(e.state().extra_turns.is_empty(), "the extra turn is spent");
    assert_eq!(e.state().turn.active, P1);
    assert_eq!(e.state().turn.number, turn + 1);
    assert!(!is_tapped(&e, vaults[0]));
}

/// Skipping into another turn of the same player's: the untap waits for a
/// turn that actually occurs, so the still-tapped Vault may skip that one
/// too, and both untaps arrive at the first turn that begins.
#[test]
fn an_untap_waits_through_a_second_skipped_turn() {
    let (mut e, vaults) = table(2, 0, 1);
    e.dev_state_mut(P0).unwrap().extra_turns.push_back(P0);
    to_skip_offer(&mut e);
    // The extra turn is skipped; p1's turn would begin next, and p1 has no
    // Vault: it begins and p0's Vault untaps there.
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(e.state().turn.active, P1);
    assert!(!is_tapped(&e, vaults[0]));

    // With two extra turns queued, the first skip leaves the Vault tapped
    // as the second would begin, and it is offered again.
    let (mut e, vaults) = table(2, 0, 1);
    e.dev_state_mut(P0).unwrap().extra_turns.push_back(P0);
    e.dev_state_mut(P0).unwrap().extra_turns.push_back(P0);
    to_skip_offer(&mut e);
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        is_tapped(&e, vaults[0]),
        "no turn has occurred to untap it in"
    );
    assert_eq!(
        skip_offer(&e),
        Some((P0, vaults[0])),
        "the second extra turn"
    );
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(e.state().turn.active, P0, "the second extra turn is taken");
    assert!(!is_tapped(&e, vaults[0]), "and the skip's untap opens it");
    assert!(e.state().extra_turns.is_empty());
}

/// Only the Vault's controller, only for their own turn: p1's turn begins
/// without a question while p0's Vault sits tapped.
#[test]
fn the_offer_is_only_for_its_controllers_turn() {
    let (mut e, vaults) = table(2, 0, 1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 || skip_offer(e).is_some()
    });
    assert_eq!(skip_offer(&e), None);
    assert_eq!(e.state().turn.active, P1);
    assert!(is_tapped(&e, vaults[0]));
}

/// The question is refused, and changes nothing, when the wrong seat or the
/// wrong kind of answer arrives; the right answer is then still taken.
#[test]
fn a_wrong_or_mistaken_answer_to_the_skip_changes_nothing() {
    let (mut e, vaults) = table(2, 0, 1);
    pass_until(&mut e, |e| e.state().turn.active == P1);
    to_skip_offer(&mut e);
    let before = e.fingerprint();
    let hash = e.snapshot_hash();
    assert!(
        e.apply(P1, PlayerAction::YesNo(true)).is_err(),
        "not p1's question"
    );
    assert!(
        e.apply(P0, PlayerAction::PassPriority).is_err(),
        "not priority"
    );
    assert!(
        e.apply(P0, PlayerAction::ChooseMode(0)).is_err(),
        "not a menu"
    );
    assert_eq!(e.fingerprint(), before);
    assert_eq!(e.snapshot_hash(), hash);
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert!(!is_tapped(&e, vaults[0]));
}

/// The deciding player leaving the game drops the question; the turn order
/// goes on without them (CR 800.4a).
#[test]
fn a_skip_offer_goes_with_a_player_who_concedes() {
    let (mut e, _) = table(3, 1, 1);
    assert_eq!(to_skip_offer(&mut e).0, P1);
    e.apply(P1, PlayerAction::Concede).unwrap();
    assert_eq!(skip_offer(&e), None);
    assert_eq!(e.state().turn.active, P2, "p2 takes the next turn");
}

/// A stolen Vault answers to its new controller: "you" follows control
/// (CR 109.5), and p1, now controlling it, is asked as p1's turn would
/// begin — p0 no longer is.
#[test]
fn the_offer_follows_control_of_the_vault() {
    let mut e = Duel::new(6141, forest())
        .battlefield(0, &[VAULT])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[index::STEAL_ARTIFACT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let vault = on_battlefield(&e, P0, VAULT).unwrap();
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(vault)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    e.refresh_offer();
    reach_their_main_phase(&mut e, P1);
    for source in all_on_battlefield(&e, P1, island()) {
        e.apply(P1, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut e, P1, index::STEAL_ARTIFACT);
    e.apply(
        P1,
        PlayerAction::ChooseTargets {
            objects: vec![vault],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(vault).unwrap().controller, P1);

    // p0's own turn begins without a question: p0 no longer controls it.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 || skip_offer(e).is_some()
    });
    assert_eq!(skip_offer(&e), None, "p0 is not asked");
    assert_eq!(e.state().turn.active, P0);
    let turn = e.state().turn.number;
    assert_eq!(to_skip_offer(&mut e), (P1, vault), "the new controller is");
    e.apply(P1, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(e.state().turn.active, P0, "p1's turn was skipped");
    assert_eq!(e.state().turn.number, turn + 1);
    assert!(!is_tapped(&e, vault));
}
