use super::{Assignment, Player, Registered, common};
use baylee_core::ids::PlayerId;
use baylee_engine::{
    choice::{Pending, PlayerAction},
    win::Victor,
};
use baylee_protocol::v1::{self, Envelope, envelope::Msg};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub(super) fn seats(
    rt: &Runtime,
    host: &Registered,
    players: &[Player; 2],
    assignments: &[Assignment; 2],
) -> Uuid {
    let mut pending = [None, None];
    let play = |i: usize, pending: &mut Option<_>| {
        players[i]
            .account
            .play_ranked(
                players[i].key,
                Some((players[i].commitment, &players[i].deck)),
                pending,
            )
            .unwrap()
    };
    assert!(play(0, &mut pending[0]).is_none());
    let second = play(1, &mut pending[1]).expect("both private registrations open one game");
    // Freshly reacquire a game-only ticket and recover the frozen seat without giving the host another deck.
    let first = players[0]
        .account
        .play_ranked(players[0].key, None, &mut None)
        .unwrap()
        .expect("bound game can be recovered");
    assert_eq!(first.seat.game_id, second.seat.game_id);
    assert_eq!(first.seat.seat, u32::from(assignments[0].seat));
    assert_eq!(second.seat.seat, u32::from(assignments[1].seat));
    let game = first.seat.game_id;
    rt.block_on(async {
        let mut a =
            common::dial_seat(host.gateway.port, &game.to_string(), &first.seat.seat_token).await;
        let mut b = common::dial_seat(
            host.gateway.port,
            &game.to_string(),
            &second.seat.seat_token,
        )
        .await;
        for s in [&mut a, &mut b] {
            common::send(
                s,
                &Envelope {
                    msg: Some(Msg::SeatReady(v1::SeatReady {})),
                },
            )
            .await;
        }
        tokio::time::timeout(common::WAIT_BUDGET, async {
            // The engine broadcasts the first decision to its addressed seat. A
            // concession comes only after both clients crossed the loading barrier.
            loop {
                let msg =
                    tokio::select! {m=common::next_msg(&mut a)=>m,m=common::next_msg(&mut b)=>m};
                match msg {
                    Some(Msg::ChoiceRequest(_)) => break,
                    Some(Msg::Error(_)) | None => panic!("ranked opening failed"),
                    _ => {}
                }
            }
            common::send(
                &mut a,
                &Envelope {
                    msg: Some(Msg::PlayerAction(v1::PlayerActionMsg {
                        game_id: String::new(),
                        seat_token: String::new(),
                        action: baylee_core::wire::encode(&PlayerAction::Concede),
                    })),
                },
            )
            .await;
            let winner = Victor::Player(PlayerId::new(assignments[1].seat));
            tokio::join!(terminal(&mut a, winner), terminal(&mut b, winner));
        })
        .await
        .expect("both actual sockets receive the confirmed engine result");
    });
    game
}

async fn terminal(socket: &mut common::Socket, winner: Victor) {
    while let Some(message) = common::next_msg(socket).await {
        match message {
            Msg::ChoiceRequest(q) => {
                let pending: Pending = baylee_core::wire::decode(&q.pending).unwrap();
                if let Pending::GameOver(result) = pending {
                    assert_eq!(result.winner, Some(winner));
                    return;
                }
            }
            Msg::Error(e) => panic!("engine rejected concession: {}", e.message),
            Msg::GameRecordChunk(_) => panic!("private engine record reached a player"),
            _ => {}
        }
    }
    panic!("socket closed without an authoritative terminal result");
}
