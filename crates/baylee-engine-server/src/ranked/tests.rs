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

/// Why a setup with `change` made to it does not open.
fn refused(change: impl FnOnce(&mut v1::RankedEngineSetup)) -> Option<&'static str> {
    let mut order = setup();
    change(&mut order);
    Replica::open_with_identity(order, &identity()).err()
}

fn inner(order: &mut v1::RankedEngineSetup) -> &mut v1::GameSetup {
    order.setup.as_mut().expect("a setup")
}

#[test]
fn a_ranked_setup_names_its_match_its_game_and_two_people() {
    type Change = fn(&mut v1::RankedEngineSetup);
    assert_eq!(refused(|_| ()), None);
    let changes: [Change; 14] = [
        |o| o.setup = None,
        |o| o.generation = 0,
        |o| o.match_id.clear(),
        |o| o.match_id = vec![0; 16],
        |o| o.match_id.push(0),
        |o| inner(o).game_id.clear(),
        |o| inner(o).game_id = Uuid::nil().to_string(),
        |o| inner(o).game_id = "f47ac10b-58cc-4372-a567-0e02b2c3d479".into(),
        |o| inner(o).seat_names.truncate(1),
        |o| inner(o).seat_names.push("Three".into()),
        |o| inner(o).seat_names[1] = "x".repeat(257),
        |o| inner(o).seat_guests = vec![false, true],
        |o| inner(o).seat_accounts = vec![true, false],
        |o| inner(o).seat_accounts = vec![true; 3],
    ];
    for (at, change) in changes.into_iter().enumerate() {
        assert!(refused(change).is_some(), "change {at}");
    }
    // What may be left out may, and a name may be as long as it may.
    assert_eq!(refused(|o| inner(o).seat_guests.clear()), None);
    assert_eq!(refused(|o| inner(o).seat_accounts.clear()), None);
    assert_eq!(refused(|o| inner(o).seat_names[1] = "x".repeat(256)), None);
}

#[test]
fn a_ranked_setup_is_this_executable_s_own_and_bounded() {
    let other = Identity::native(
        Variant::StandardLoops,
        "ranked-test (0123456789)",
        "0123456789abcdef0123456789abcdef01234567",
        2,
    );
    assert!(refused(|o| inner(o).execution = other.to_metadata()).is_some());
    let unranked = Identity::native(Variant::Standard, "b", "r", 1);
    let mut order = setup();
    inner(&mut order).execution = unranked.to_metadata();
    assert!(Replica::open_with_identity(order, &unranked).is_err());
    for bad in [&b""[..], b"{}", b"\xff", b"[[[["] {
        assert!(refused(|o| inner(o).preset_json = bad.to_vec()).is_some());
    }
    assert!(
        refused(|o| {
            let json = &mut inner(o).preset_json;
            json.resize(1024 * 1024 + 1, b' ');
        })
        .is_some(),
        "over its bound"
    );
    assert_eq!(
        refused(|o| {
            let json = &mut inner(o).preset_json;
            json.resize(1024 * 1024, b' ');
        }),
        None
    );
}

/// No advantage can be written into a ranked game's rules: whatever differs
/// from the one duel there is, is refused.
#[test]
fn a_ranked_game_has_no_rule_of_its_own() {
    type Change = fn(&mut baylee_core::preset::GamePreset);
    let ruled = |change: Change| {
        refused(|o| {
            let json = &mut inner(o).preset_json;
            let mut preset = serde_json::from_slice(json).unwrap();
            change(&mut preset);
            *json = serde_json::to_vec(&preset).unwrap();
        })
    };
    assert_eq!(ruled(|_| ()), None);
    let changes: [Change; 12] = [
        |p| p.seats[1].starting_life = Some(21),
        |p| p.seats[1].starting_life = None,
        |p| p.seats[1].starting_hand = Some(vec![]),
        |p| p.seats[1].starting_battlefield = vec![p.seats[1].deck[0]],
        |p| p.seats[1].team = Some(1),
        |p| p.seats[0].controller = SeatController::Ai(baylee_core::preset::AIProfile::default()),
        |p| {
            let more = [p.seats[0].deck[0]; 191];
            p.seats[0].deck.extend(more);
        },
        |p| p.seats.truncate(1),
        |p| p.seats.push(p.seats[0].clone()),
        |p| p.format = FormatId::Custom,
        |p| p.format = FormatId::Commander,
        |p| p.house_rules = HouseRules::default(),
    ];
    for (at, change) in changes.into_iter().enumerate() {
        assert!(ruled(change).is_some(), "change {at}");
    }
}
