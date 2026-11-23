//! The only player data-plane socket is Central's, before and after promotion.
use super::{Player, Registered, Served, common};
use baylee_account::RankedPlay;
use futures_util::{SinkExt as _, StreamExt as _};
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpSocket;
use tokio_tungstenite::{MaybeTlsStream, tungstenite::Message};

async fn tcp(addr: SocketAddr, client: u8) -> tokio::net::TcpStream {
    let socket = TcpSocket::new_v4().unwrap();
    // Linux routes the full 127/8 block. Distinct sources make hiding each
    // player's IP observable on the actual established kernel connections.
    let source = if cfg!(target_os = "linux") {
        Ipv4Addr::new(127, 0, 0, 2 + client)
    } else {
        Ipv4Addr::LOCALHOST
    };
    socket.bind((source, 0).into()).unwrap();
    socket.connect(addr).await.unwrap()
}

async fn dial(addr: SocketAddr, client: u8) -> common::Socket {
    let tcp = tcp(addr, client).await;
    let (ws, _) = tokio_tungstenite::client_async(
        format!("ws://{addr}/v1/ranked/socket"),
        MaybeTlsStream::Plain(tcp),
    )
    .await
    .unwrap();
    ws
}

/// Exercise a real account-authenticated ranked HTTP admission from the same
/// distinguishable source address as its subsequent Central WebSocket.
async fn preflight(
    server: &Served,
    p: &Player,
    client: u8,
    fence: baylee_account::ranked::replica::Fence,
) {
    use baylee_account::{
        ranked::{
            self,
            replica::{
                SeatRequest, SeatResponse,
                relay::{Call, Reply},
            },
        },
        wire,
    };
    use http_body_util::{BodyExt as _, Full};
    let bytes = central_api::encode(&wire::Request::Ranked {
        request: ranked::Request::Replicate {
            request: SeatRequest::Relay {
                fence,
                call: Call::Admit {},
            },
        },
    })
    .unwrap();
    let (mut sender, connection) = hyper::client::conn::http1::handshake(
        hyper_util::rt::TokioIo::new(tcp(server.address, client).await),
    )
    .await
    .unwrap();
    let job = tokio::spawn(connection);
    let request = hyper::Request::post("/v1/native")
        .header(hyper::header::HOST, server.address.to_string())
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(hyper::header::CONNECTION, "close")
        .header(central_api::PROTOCOL_HEADER, central_api::PROTOCOL_VERSION)
        .header(
            hyper::header::AUTHORIZATION,
            p.account.bearer_header().unwrap().as_str(),
        )
        .body(Full::new(bytes::Bytes::from(bytes)))
        .unwrap();
    let response = sender.send_request(request).await.unwrap();
    assert_eq!(response.status(), hyper::StatusCode::OK);
    let body = http_body_util::Limited::new(response.into_body(), central_api::MAX_WIRE_BYTES)
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert!(
        matches!(
            central_api::decode::<wire::Response>(&body).unwrap(),
            wire::Response::Ranked {
                response: ranked::Response::Replica {
                    response: SeatResponse::Relayed {
                        reply: Reply::Http {
                            status: 200,
                            body: Some(_)
                        }
                    }
                }
            }
        ),
        "Central relayed the preflight and returned a scoped opaque handle"
    );
    drop(sender);
    job.await.unwrap().unwrap();
}

pub(super) fn admissions(
    rt: &tokio::runtime::Runtime,
    server: &Served,
    host: &Registered,
    players: &[Player; 2],
    id: super::Id,
) {
    use baylee_account::ranked::replica::{SeatRequest, SeatResponse};
    let SeatResponse::Status { status } = players[0]
        .account
        .replica(SeatRequest::Status { match_id: id })
        .unwrap()
    else {
        panic!("expected assigned participant status")
    };
    #[cfg(target_os = "linux")]
    let old = peers(host.gateway.port, false);
    rt.block_on(async {
        tokio::time::timeout(common::WAIT_BUDGET, async {
            tokio::join!(
                preflight(server, &players[0], 0, status.fence),
                preflight(server, &players[1], 1, status.fence)
            );
        })
        .await
        .expect("both Central HTTP preflights completed");
    });
    #[cfg(target_os = "linux")]
    {
        let observed = peers(host.gateway.port, false);
        assert!(
            observed.difference(&old).count() >= 2,
            "two new upstream TCP exchanges reached the approved gateway"
        );
        assert!(
            observed
                .difference(&old)
                .all(|p| p.ip() == &Ipv4Addr::LOCALHOST)
        );
        let clients = peers(server.address.port(), false);
        for ip in [2, 3].map(|i| Ipv4Addr::new(127, 0, 0, i)) {
            assert!(clients.iter().any(|p| *p.ip() == ip));
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = host;
}

pub(super) async fn connect(server: &Served, ticket: &RankedPlay, client: u8) -> common::Socket {
    assert_eq!(ticket.relay_origin, server.endpoint.origin());
    let mut socket = dial(server.address, client).await;
    let bytes = ticket.seat.seat_token.as_bytes();
    assert!(
        bytes.len() == 64
            && bytes
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    );
    socket
        .send(Message::Binary(bytes.to_vec().into()))
        .await
        .unwrap();
    socket
}

pub(super) async fn reject(server: &Served, token: &[u8]) {
    let mut socket = dial(server.address, 0).await;
    socket
        .send(Message::Binary(token.to_vec().into()))
        .await
        .unwrap();
    tokio::time::timeout(common::WAIT_BUDGET, async {
        while let Some(Ok(frame)) = socket.next().await {
            if frame.is_close() {
                break;
            }
            assert!(
                frame.is_ping() || frame.is_pong(),
                "denied relay connection produced application data"
            );
        }
    })
    .await
    .expect("unknown or previously consumed ticket must be closed by Central");
}

#[cfg(target_os = "linux")]
fn peers(port: u16, established: bool) -> std::collections::BTreeSet<std::net::SocketAddrV4> {
    fn addr(raw: &str) -> Option<std::net::SocketAddrV4> {
        let (ip, port) = raw.split_once(':')?;
        Some(std::net::SocketAddrV4::new(
            Ipv4Addr::from(u32::from_str_radix(ip, 16).ok()?.to_ne_bytes()),
            u16::from_str_radix(port, 16).ok()?,
        ))
    }
    std::fs::read_to_string("/proc/net/tcp")
        .unwrap()
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split_whitespace().skip(1);
            let a = addr(fields.next()?)?;
            let b = addr(fields.next()?)?;
            let state = fields.next()?;
            if state != "01" && (established || state != "06") {
                return None;
            }
            if a.port() == port {
                Some(b)
            } else if b.port() == port {
                Some(a)
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn verify_peers(server: &Served, host: &Registered) {
    #[cfg(target_os = "linux")]
    {
        let central = peers(server.address.port(), true);
        assert!(
            [2, 3]
                .map(|n| Ipv4Addr::new(127, 0, 0, n))
                .iter()
                .all(|ip| central.iter().any(|p| p.ip() == ip))
        );
        let upstream = peers(host.gateway.port, true);
        assert!(
            upstream.len() >= 2 && upstream.iter().all(|p| *p.ip() == Ipv4Addr::LOCALHOST),
            "the selected host must only see local agents and Central's relay address"
        );
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (server, host);
}
