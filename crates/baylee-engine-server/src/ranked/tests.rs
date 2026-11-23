use super::*;
use baylee_core::preset::{FormatId, HouseRules, SeatController};
use baylee_engine::choice::PlayerAction;
use baylee_protocol::execution::{Identity, Variant};
use uuid::Uuid;

fn identity() -> Identity {
    Identity::native(
        Variant::StandardLoops,
        "ranked-test (0123456789)",
        "0123456789abcdef0123456789abcdef01234567",
        1,
    )
}

fn setup() -> v1::RankedEngineSetup {
    let forest =
        baylee_cards::ranked::registration("standard", &["60 Forest".into()], &[], &[]).unwrap();
    let mut preset = baylee_cards::decks::preset_for_all(41, &[&forest.deck, &forest.deck]);
    preset.format = FormatId::Freeform;
    preset.house_rules = HouseRules::ranked();
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
        seat.starting_life = Some(20);
    }
    v1::RankedEngineSetup {
        generation: 1,
        match_id: Uuid::now_v7().as_bytes().to_vec(),
        primary: true,
        setup: Some(v1::GameSetup {
            game_id: Uuid::now_v7().to_string(),
            seat_names: vec!["One".into(), "Two".into()],
            seat_accounts: vec![true; 2],
            seat_guests: vec![false; 2],
            execution: identity().to_metadata(),
            preset_json: serde_json::to_vec(&preset).unwrap(),
        }),
    }
}

fn vote(messages: &[Envelope]) -> v1::RankedEngineVote {
    let [
        Envelope {
            msg: Some(Msg::RankedEngineVote(vote)),
        },
    ] = messages
    else {
        panic!("a pending step must emit only a private vote");
    };
    vote.clone()
}

fn open(order: &v1::RankedEngineSetup) -> (Replica, v1::RankedEngineVote) {
    let (replica, first) = Replica::open_with_identity(order.clone(), &identity()).unwrap();
    (replica, vote(&first))
}

fn action(seat: u32, position: u64, which: &PlayerAction) -> v1::RankedEngineStep {
    let body = wrap(Msg::PlayerAction(v1::PlayerActionMsg {
        action: baylee_core::wire::encode(which),
        ..Default::default()
    }));
    v1::RankedEngineStep {
        generation: 1,
        position,
        command_id: Uuid::now_v7().as_bytes().to_vec(),
        seat,
        operation: body.encode_to_vec(),
        authority: false,
    }
}

fn commit(vote: &v1::RankedEngineVote) -> v1::RankedEngineCommit {
    v1::RankedEngineCommit {
        generation: vote.generation,
        position: vote.position,
        evidence: vote.evidence.clone(),
        chain: vec![7; 32],
        publish: true,
    }
}

fn ask(position: u64, generation: u64) -> v1::RankedEngineSnapshot {
    v1::RankedEngineSnapshot {
        position,
        generation,
        seat: 1,
    }
}

#[test]
fn certification_gates_hidden_frames_and_replays_across_generation() {
    let mut order = setup();
    let (mut lead, opening) = open(&order);
    order.primary = false;
    let (mut standby, shadow_opening) = open(&order);
    assert_eq!(opening, shadow_opening);
    assert_eq!(opening.awaiting, 3);
    assert!(lead.snapshot(&ask(0, 1)).is_err());
    let mut wrong = commit(&opening);
    wrong.evidence[0] ^= 1;
    assert!(lead.commit(&wrong).is_err());
    assert!(!lead.commit(&commit(&opening)).unwrap().is_empty());
    assert!(standby.commit(&commit(&opening)).unwrap().is_empty());
    assert!(standby.snapshot(&ask(0, 1)).is_err());

    let first = action(0, 1, &PlayerAction::MulliganKeep);
    let one = vote(&lead.step(first.clone()).unwrap());
    assert!(one.accepted);
    assert_eq!(vote(&standby.step(first.clone()).unwrap()), one);
    assert!(lead.snapshot(&ask(0, 1)).unwrap().is_empty());
    assert_eq!(vote(&lead.step(first.clone()).unwrap()), one);
    let frames = lead.commit(&commit(&one)).unwrap();
    assert!(!frames.is_empty());
    assert!(standby.commit(&commit(&one)).unwrap().is_empty());
    assert!(lead.commit(&commit(&one)).unwrap().is_empty());

    order.primary = true;
    order.generation = 2;
    let (mut promoted, reopened) = open(&order);
    assert_eq!(reopened.evidence, opening.evidence);
    let mut silent = commit(&reopened);
    silent.publish = false;
    assert!(promoted.commit(&silent).unwrap().is_empty());
    let mut historical = first;
    historical.generation = 2;
    let restored = vote(&promoted.step(historical).unwrap());
    assert_eq!(restored.evidence, one.evidence);
    silent = commit(&restored);
    silent.publish = false;
    assert!(promoted.commit(&silent).unwrap().is_empty());
    assert!(promoted.snapshot(&ask(1, 1)).is_err());
    let frames = promoted.snapshot(&ask(1, 2)).unwrap();
    assert!(!frames.is_empty());
    for frame in frames {
        let Some(Msg::SeatFrame(frame)) = frame.msg else {
            panic!("a seat frame")
        };
        assert_eq!(frame.seat, 1);
        let Some(Msg::RankedSeatProjection(projection)) =
            Envelope::decode(frame.envelope).unwrap().msg
        else {
            panic!("a protected projection")
        };
        assert_eq!((projection.position, projection.generation), (1, 2));
        assert_eq!(projection.chain, silent.chain);
    }
    let end = action(1, 2, &PlayerAction::Concede);
    let left = vote(&standby.step(end.clone()).unwrap());
    let right = vote(
        &promoted
            .step(v1::RankedEngineStep {
                generation: 2,
                ..end
            })
            .unwrap(),
    );
    assert_eq!((right.outcome, right.awaiting), (1, 0));
    assert_eq!(left.evidence, right.evidence);
    assert!(!promoted.commit(&commit(&right)).unwrap().is_empty());
}

#[test]
fn mismatched_setup_conflicts_and_a_forged_authority_event_are_refused() {
    let order = setup();
    let mut wrong = order.clone();
    wrong.setup.as_mut().unwrap().execution.clear();
    assert!(Replica::open_with_identity(wrong, &identity()).is_err());
    let (mut replica, opening) = open(&order);
    replica.commit(&commit(&opening)).unwrap();
    let mut step = action(0, 2, &PlayerAction::MulliganKeep);
    assert!(replica.step(step.clone()).is_err());
    step.position = 1;
    let authority = wrap(Msg::RankedAuthorityEvent(v1::RankedAuthorityEvent {
        seat: 0,
        decision: opening.asked_zero,
        reason: 1,
    }))
    .encode_to_vec();
    step.operation = authority.clone();
    let denied = vote(&replica.step(step.clone()).unwrap());
    assert!(!denied.accepted);
    assert_eq!((denied.awaiting, denied.outcome), (3, 0));
    step.authority = true;
    assert!(replica.step(step).is_err());
    replica.commit(&commit(&denied)).unwrap();
    let system = v1::RankedEngineStep {
        authority: true,
        operation: authority,
        ..action(0, 2, &PlayerAction::Concede)
    };
    let last = vote(&replica.step(system).unwrap());
    assert!(last.accepted);
    assert_eq!(last.outcome, 2);
}
