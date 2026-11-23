//! What each native player can actually see at the certified position.
use super::{Id, Player, Registered, authority, common};
use baylee_account::{
    RankedPlay,
    ranked::replica::{Phase, SeatRequest, SeatResponse, SeatStatus},
};
use baylee_core::ids::PlayerId;
use baylee_engine::{choice::Pending, win::Victor};
use baylee_protocol::v1::{self, Envelope, envelope::Msg};
use baylee_view::{PlayerView, ViewReader};
use prost::Message as _;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

pub(super) fn bind(players: &[Player; 2], match_id: Id, with_decks: bool) -> [RankedPlay; 2] {
    let mut result = [None, None];
    let mut auth = [None, None];
    let until = Instant::now() + Duration::from_secs(80);
    while result.iter().any(Option::is_none) {
        for (i, p) in players.iter().enumerate() {
            if result[i].is_some() {
                continue;
            }
            let route = authority::status(p, match_id);
            assert!(matches!(route.phase, Phase::Preparing | Phase::Playing));
            let deck = with_decks.then_some((p.commitment, &p.deck));
            match p
                .account
                .play_ranked_replica(p.key, &route, deck, &mut auth[i])
            {
                Ok(play) => result[i] = play,
                Err(
                    baylee_account::Error::Conflict
                    | baylee_account::Error::Busy
                    | baylee_account::Error::Unavailable,
                ) => {}
                Err(error) => panic!("ranked admission failed: {error:?}"),
            }
        }
        assert!(Instant::now() < until, "both frozen seats must be granted");
        if result.iter().any(Option::is_none) {
            std::thread::sleep(Duration::from_millis(350));
        }
    }
    result.map(Option::unwrap)
}

pub(super) struct Connected {
    pub(super) sockets: [common::Socket; 2],
    seats: [PlayerId; 2],
}

impl Connected {
    pub(super) fn disconnected(&mut self, rt: &Runtime) {
        async fn closed(socket: &mut common::Socket) {
            while common::next_msg(socket).await.is_some() {}
        }
        let [a, b] = &mut self.sockets;
        rt.block_on(async {
            tokio::time::timeout(common::WAIT_BUDGET, async {
                tokio::join!(closed(a), closed(b));
            })
            .await
            .expect("crashed primary closed both actual player sockets");
        });
    }

    pub(super) fn new(
        fixture: &super::Fixture<'_>,
        host: &Registered,
        tickets: &[RankedPlay; 2],
        players: &[Player; 2],
        status: &SeatStatus,
    ) -> Self {
        assert_ne!(tickets[0].seat.seat, tickets[1].seat.seat);
        let seats = tickets.each_ref().map(|t| {
            assert_eq!(t.seat.game_id.as_bytes(), &status.fence.match_id);
            assert_eq!(t.assignment.host_id, host.id);
            assert_eq!(t.seat.seat, u32::from(t.assignment.seat));
            PlayerId::new(t.assignment.seat)
        });
        let connect = |i: usize| async move {
            let mut socket =
                super::super::relay::connect(fixture.served, &tickets[i], u8::try_from(i).unwrap())
                    .await;
            common::send(
                &mut socket,
                &Envelope {
                    msg: Some(Msg::SeatReady(v1::SeatReady {})),
                },
            )
            .await;
            socket
        };
        let (a, b) = fixture
            .rt
            .block_on(async { tokio::join!(connect(0), connect(1)) });
        for p in players {
            assert!(matches!(
                p.account
                    .replica(SeatRequest::Presence {
                        fence: status.fence,
                        connected: true
                    })
                    .unwrap(),
                SeatResponse::Status { .. }
            ));
        }
        Self {
            sockets: [a, b],
            seats,
        }
    }

    pub(super) fn views(&mut self, rt: &Runtime, route: &SeatStatus) -> [PlayerView; 2] {
        let [a, b] = &mut self.sockets;
        let (va, vb) = rt.block_on(async {
            tokio::time::timeout(common::WAIT_BUDGET, async {
                tokio::join!(snapshot(a, route), snapshot(b, route))
            })
            .await
            .expect("both private frozen views reached their players")
        });
        for (view, seat) in [(&va, self.seats[0]), (&vb, self.seats[1])] {
            assert_eq!(view.seat, seat);
            assert_eq!(view.hand.len(), 7);
            assert!(view.shared_hands.is_empty() && view.controlled_hands.is_empty());
        }
        [va, vb]
    }

    pub(super) fn terminal(&mut self, rt: &Runtime, route: &SeatStatus) {
        let [a, b] = &mut self.sockets;
        let winner = Victor::Player(self.seats[1]);
        rt.block_on(async {
            tokio::time::timeout(common::WAIT_BUDGET, async {
                tokio::join!(terminal(a, route, winner), terminal(b, route, winner));
            })
            .await
            .expect("both player sockets delivered the independently certified winner");
        });
    }
}

async fn projection(socket: &mut common::Socket, status: &SeatStatus) -> Msg {
    loop {
        match common::next_msg(socket)
            .await
            .expect("assigned seat disconnected")
        {
            Msg::RankedSeatProjection(frame) => {
                assert_eq!(frame.generation, status.fence.generation);
                if frame.position < status.head.position {
                    continue;
                }
                assert_eq!(frame.position, status.head.position);
                assert_eq!(frame.chain.as_slice(), status.head.chain);
                return Envelope::decode(frame.envelope.as_slice())
                    .unwrap()
                    .msg
                    .unwrap();
            }
            Msg::StateDelta(_)
            | Msg::ChoiceRequest(_)
            | Msg::GameStatic(_)
            | Msg::GameRecordChunk(_) => {
                panic!("ranked gameplay escaped the certified per-seat projection")
            }
            _ => {}
        }
    }
}

async fn snapshot(socket: &mut common::Socket, status: &SeatStatus) -> PlayerView {
    let mut statics: Option<baylee_view::GameStatic> = None;
    let mut reader = ViewReader::default();
    loop {
        match projection(socket, status).await {
            Msg::GameStatic(frame) if !frame.statics.is_empty() => {
                let roster: baylee_view::GameStatic =
                    baylee_core::wire::decode(&frame.statics).unwrap();
                assert_eq!(roster.seats.len(), 2);
                for (index, label) in ["Seat A", "Seat B"].iter().enumerate() {
                    assert_eq!(roster.seats[index].display_name, *label);
                    assert_eq!(usize::from(roster.seats[index].player.get()), index);
                }
                statics = Some(roster);
            }
            Msg::StateDelta(frame) => {
                reader.read(&frame.view_frame).unwrap();
            }
            _ => {}
        }
        if let (Some(statics), Some(view)) = (&statics, reader.view()) {
            assert_eq!(statics.your_seat, view.seat);
            return view.clone();
        }
    }
}

async fn terminal(socket: &mut common::Socket, status: &SeatStatus, winner: Victor) {
    loop {
        if let Msg::ChoiceRequest(q) = projection(socket, status).await
            && let Pending::GameOver(result) = baylee_core::wire::decode(&q.pending).unwrap()
        {
            assert_eq!(result.winner, Some(winner));
            return;
        }
    }
}

pub(super) fn same_view(before: &PlayerView, after: &PlayerView) {
    assert_eq!(
        (
            before.seat,
            before.seq,
            before.turn,
            before.phase,
            before.step,
            before.awaiting
        ),
        (
            after.seat,
            after.seq,
            after.turn,
            after.phase,
            after.step,
            after.awaiting
        )
    );
    assert_eq!(before.hand, after.hand);
    assert_eq!(before.deciding, after.deciding);
    assert_eq!(before.battlefield, after.battlefield);
    assert_eq!(before.seats, after.seats);
    assert_eq!(before.stack, after.stack);
}
