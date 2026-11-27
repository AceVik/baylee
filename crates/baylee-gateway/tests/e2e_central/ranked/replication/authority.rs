//! Only the account's real Central session may originate a journal entry.
use super::{Assignment, Format, Id, Player, QueueState, Request, Response};
use baylee_account::ranked::replica::{CommandBytes, Phase, SeatRequest, SeatResponse, SeatStatus};
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{Envelope, PlayerActionMsg, envelope::Msg};
use prost::Message as _;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub(in super::super) fn join(players: &[Player; 2]) -> [Assignment; 2] {
    for p in players {
        let reply = p
            .account
            .replica(SeatRequest::Join {
                key: p.key,
                format: Format::Standard,
                deck: p.commitment,
                ai: baylee_account::ranked::ai::AiOpponents::None,
            })
            .unwrap();
        assert!(matches!(reply, SeatResponse::Queue { .. }));
    }
    players.each_ref().map(|p| {
        let until = Instant::now() + Duration::from_secs(45);
        loop {
            match p.account.ranked(Request::Status).unwrap() {
                Response::Queue {
                    state: QueueState::Assigned { key, assignment },
                } => {
                    assert_eq!(key, p.key);
                    assert_eq!(assignment.account, p.account.public_id());
                    assert_eq!(assignment.deck, p.commitment);
                    return assignment;
                }
                Response::Queue {
                    state: QueueState::Waiting { .. },
                } if Instant::now() < until => {
                    std::thread::sleep(Duration::from_millis(250));
                }
                _ => panic!("expected this entry to receive a verified ranked assignment"),
            }
        }
    })
}

pub(super) fn status(p: &Player, match_id: Id) -> SeatStatus {
    let SeatResponse::Status { status } =
        p.account.replica(SeatRequest::Status { match_id }).unwrap()
    else {
        panic!("expected account-scoped replica routing")
    };
    status
}

pub(super) fn wait(
    p: &Player,
    match_id: Id,
    condition: impl Fn(&SeatStatus) -> bool,
) -> SeatStatus {
    let until = Instant::now() + Duration::from_secs(110);
    loop {
        let route = status(p, match_id);
        if condition(&route) {
            return route;
        }
        assert_ne!(
            route.phase,
            Phase::Disputed,
            "independent engines disagreed"
        );
        assert!(
            Instant::now() < until,
            "replica timed out at generation {}, position {}, phase {:?}",
            route.fence.generation,
            route.head.position,
            route.phase
        );
        std::thread::sleep(Duration::from_millis(350));
    }
}

pub(super) fn intent(route: &SeatStatus, action: &PlayerAction) -> SeatRequest {
    let inner = Envelope {
        msg: Some(Msg::PlayerAction(PlayerActionMsg {
            action: baylee_core::wire::encode(action),
            ..Default::default()
        })),
    };
    SeatRequest::Submit {
        fence: route.fence,
        expected: route.head,
        command_id: *Uuid::now_v7().as_bytes(),
        body: CommandBytes::new(inner.encode_to_vec()).unwrap(),
    }
}

pub(super) fn submit(p: &Player, request: &SeatRequest) {
    let SeatRequest::Submit {
        fence,
        expected,
        command_id,
        ..
    } = request
    else {
        unreachable!()
    };
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        match p.account.replica(request.clone()).unwrap() {
            SeatResponse::Status { status } => {
                // A newly attached seat can receive its first view before both
                // executors have pulsed. Retry only the original signed intent.
                assert_eq!((status.fence, status.head), (*fence, *expected));
                assert_eq!(status.phase, Phase::Playing);
                assert!(Instant::now() < until, "intent remained unacknowledged");
                std::thread::sleep(Duration::from_millis(350));
            }
            SeatResponse::Receipt {
                command_id: id,
                position,
            } => {
                assert_eq!((id, position), (*command_id, expected.position + 1));
                return;
            }
            _ => panic!("unexpected response to a participant's intent"),
        }
    }
}
