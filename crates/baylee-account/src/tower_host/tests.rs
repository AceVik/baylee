use super::*;
use baylee_tower_api::host::{Lease, Prefix};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
};
use uuid::Uuid;

fn batch() -> Batch {
    Batch {
        lease: Lease {
            reservation: Uuid::now_v7(),
            claim: Uuid::now_v7(),
            fence: 7,
        },
        expected: Prefix::default(),
        chunks: vec![b"record\n".to_vec()],
    }
}

async fn reply(
    status: u16,
    headers: &str,
    body: Vec<u8>,
    batch: &Batch,
) -> Result<Acknowledged, Error> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = TowerHost::new(
        &format!("http://{address}"),
        Token::generate().unwrap(),
        true,
    )
    .unwrap();
    let head = format!(
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let read = async {
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") && header.len() < 8192 {
                header.push(stream.read_u8().await.unwrap());
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.starts_with("POST /v2/host/record HTTP/1.1"));
            let size: usize = header
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .unwrap()
                .1
                .trim()
                .parse()
                .unwrap();
            assert!(size <= 8192);
            let mut input = vec![0; size];
            stream.read_exact(&mut input).await.unwrap();
            assert!(Batch::decode(&input).is_ok());
        };
        tokio::time::timeout(Duration::from_secs(3), read)
            .await
            .unwrap();
        let _ = stream.write_all(head.as_bytes()).await;
        let _ = stream.write_all(&body).await;
    });
    let result = client.append(batch).await;
    server.await.unwrap();
    assert!(
        TcpStream::connect(address).await.is_err(),
        "owned listener closed"
    );
    result
}

#[tokio::test]
async fn host_transport_refuses_partial_wrong_generation_and_unbounded_acknowledgements() {
    let batch = batch();
    let header = format!(
        "Content-Type: application/json\r\nBaylee-Tower-Version: {}\r\n",
        baylee_tower_api::VERSION
    );
    let valid = header.as_str();
    let expected = Acknowledged {
        fence: batch.lease.fence,
        prefix: Prefix::default(),
    };
    let mut complete = expected;
    complete.prefix.chunks = 1;
    complete.prefix.bytes = batch.chunks[0].len() as u64;
    let wire =
        |acknowledgement| serde_json::to_vec(&Response::Acknowledged { acknowledgement }).unwrap();
    assert_eq!(
        reply(200, valid, wire(complete), &batch).await.unwrap(),
        complete
    );
    assert!(reply(200, valid, wire(expected), &batch).await.is_err());
    let mut changed = complete;
    changed.fence += 1;
    assert!(reply(200, valid, wire(changed), &batch).await.is_err());
    changed = complete;
    changed.prefix.bytes += 1;
    assert!(reply(200, valid, wire(changed), &batch).await.is_err());
    assert!(
        reply(
            200,
            "Content-Type: application/json\r\n",
            wire(complete),
            &batch
        )
        .await
        .is_err()
    );
    assert!(
        reply(200, valid, br#"{"result":"advertised"}"#.to_vec(), &batch)
            .await
            .is_err()
    );
    assert!(
        reply(200, valid, vec![b' '; host::MAX_RESPONSE_BYTES + 1], &batch)
            .await
            .is_err()
    );
    let sink = TcpListener::bind("127.0.0.1:0").await.unwrap();
    assert!(
        reply(
            307,
            &format!("Location: http://{}/stolen\r\n", sink.local_addr().unwrap()),
            Vec::new(),
            &batch
        )
        .await
        .is_err()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(20), sink.accept())
            .await
            .is_err()
    );
    drop(sink);
}

async fn claimed(status: u16, headers: &str, body: Vec<u8>) -> Result<Response, Refused> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = TowerHost::new(
        &format!("http://{address}"),
        Token::generate().unwrap(),
        true,
    )
    .unwrap();
    let head = format!(
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") && header.len() < 8192 {
            header.push(stream.read_u8().await.unwrap());
        }
        let header = String::from_utf8(header).unwrap();
        assert!(header.starts_with(&format!("POST {} HTTP/1.1", host::CONTROL_PATH)));
        let size: usize = header
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .unwrap()
            .1
            .trim()
            .parse()
            .unwrap();
        let mut input = vec![0; size];
        stream.read_exact(&mut input).await.unwrap();
        let _ = stream.write_all(head.as_bytes()).await;
        let _ = stream.write_all(&body).await;
    });
    let result = client
        .answered(&Request::Claim {
            account: Uuid::now_v7(),
            run: Uuid::now_v7(),
            claim: Uuid::now_v7(),
        })
        .await;
    server.await.unwrap();
    result
}

#[tokio::test]
async fn only_the_authority_s_own_refusal_of_a_claim_is_definite() {
    // "Not your run": nothing was committed, so the gateway may free its slot.
    assert_eq!(
        claimed(404, "", Vec::new()).await.err(),
        Some(Refused {
            error: Error::Rejected,
            definite: true
        })
    );
    for status in [429, 503] {
        assert!(
            !claimed(status, "", Vec::new())
                .await
                .err()
                .unwrap()
                .definite
        );
    }
    // A garbled 200 may follow a committed claim: it stays ambiguous.
    let header = format!(
        "Content-Type: application/json\r\nBaylee-Tower-Version: {}\r\n",
        baylee_tower_api::VERSION
    );
    assert_eq!(
        claimed(200, &header, b"{".to_vec()).await.err(),
        Some(Refused {
            error: Error::Rejected,
            definite: false
        })
    );
}
