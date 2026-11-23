//! A second genuinely released and claimed fixture item exercises both wire versions.
use super::{
    Account, DatabaseConnection, Runtime, Table, Uuid, Value, advent, json, proof, statement, table,
};
use advent::puzzle::{Line, Move};
use sea_orm::{ConnectionTrait, TransactionTrait};

pub(super) fn earn(
    rt: &Runtime,
    operator: &DatabaseConnection,
    owner: &Account,
    year: u16,
) -> advent::Reward {
    let framing = json!({
        "duel":{"focus":[3000,5500],"safe":[1500,2000,8000,8500]},
        "variants":[{"seats":4,"allies":2,"sides":2,"framing":{"focus":[6000,5800]}}]
    });
    rt.block_on(async {
        let tx = operator.begin().await.unwrap();
        let year = i16::try_from(year).unwrap();
        tx.execute_raw(statement(
            "INSERT INTO central_identity.playmat_framing(art_id,framing)
             SELECT art_id,$2::text::jsonb FROM central_identity.advent_doors WHERE year=$1 AND door=2",
            [year.into(), framing.to_string().into()],
        )).await.unwrap();
        // Cross the second release boundary only after the native first-door assertions.
        tx.execute_raw(statement(
            "UPDATE central_identity.advent_campaigns SET starts_at=starts_at-interval '24 hours',
             closes_at=closes_at-interval '24 hours' WHERE year=$1",
            [year.into()],
        )).await.unwrap();
        tx.commit().await.unwrap();
    });
    let solved = owner
        .advent(advent::Request::Submit {
            year,
            door: 2,
            line: Line(vec![Move::Attack { attackers: 1 }]),
        })
        .unwrap();
    assert!(matches!(solved, advent::Response::Submission { won: true }));
    let advent::Response::Reward { reward } = owner
        .advent(advent::Request::Claim { year, door: 2 })
        .unwrap()
    else {
        panic!("second door must yield the earned synthetic mat");
    };
    assert_eq!(reward.kind, advent::Kind::Playmat);
    assert_eq!(json!(reward.asset.mat_framing), framing);
    reward
}

fn fetch(table: &Table, seat: &str, framing: bool, page: Option<Value>) -> Value {
    let url = format!(
        "http://127.0.0.1:{}{}",
        table.port,
        table.path(if page.is_some() {
            "event-art"
        } else {
            "cosmetics"
        })
    );
    let bearer = format!("Bearer {seat}");
    let response = if let Some(page) = page {
        let mut call = ureq::post(url)
            .header("Authorization", bearer)
            .header("Content-Type", "application/json");
        if framing {
            call = call.header(advent::framing::CAPABILITY_HEADER, "1");
        }
        call.send(page.to_string())
    } else {
        let mut call = ureq::get(url).header("Authorization", bearer);
        if framing {
            call = call.header(advent::framing::CAPABILITY_HEADER, "1");
        }
        call.call()
    }
    .expect("admitted guest can read the selected playmat");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["Cache-Control"], "private, no-store");
    serde_json::from_slice(&response.into_body().read_to_vec().unwrap()).unwrap()
}

pub(super) fn verify(
    table: &Table,
    owner: &Account,
    origin: &str,
    [owner_seat, guest]: [&str; 2],
    sleeve: &advent::Reward,
    mat: &advent::Reward,
) {
    let game = Uuid::parse_str(&table.id).unwrap();
    let invitation = proof(
        owner,
        table::Request::Begin {
            origin: origin.into(),
            game: game.into_bytes(),
            seat: 0,
            look: advent::Equipment {
                sleeve: Some(sleeve.id),
                playmat: Some(mat.id),
            },
        },
    );
    assert_eq!(table.present(guest, &invitation), 403);
    assert_eq!(table.present(&table.owner_seat, &invitation), 403);
    assert_eq!(table.present(owner_seat, &invitation), 204);
    assert_eq!(
        table.page(&table.guest_seat, advent::Kind::Playmat, None).0,
        403
    );
    let origin = format!("http://127.0.0.1:{}", table.port);
    let client = baylee_account::GatewayHttp::development(&origin).unwrap();
    let own_bytes = owner
        .advent_asset(advent::AssetScope::Owned { item: mat.id }, &mat.asset)
        .unwrap();
    assert_eq!(
        client.advent_table_looks(game, guest).unwrap()[0].playmat,
        Some(mat.asset)
    );
    let bytes = client
        .advent_table_asset(game, guest, 0, advent::Kind::Playmat, &mat.asset, || true)
        .unwrap();
    assert_eq!(bytes.len(), own_bytes.len());
    assert!(bytes == own_bytes);
    for framed in [false, true] {
        let mut expected = mat.asset;
        if !framed {
            expected.mat_framing = None;
        }
        assert_eq!(
            fetch(table, guest, framed, None),
            json!({"0":{"event_sleeve":sleeve.asset,"event_playmat":expected}})
        );
        for cached in [None, Some(mat.asset.digest)] {
            let body = json!({"seat":0,"kind":"playmat","offset":0,"cached":cached});
            let response = fetch(table, guest, framed, Some(body));
            if cached.is_some() {
                assert_eq!(response, json!({"cached":{"asset":expected}}));
            } else {
                assert_eq!(response["asset"]["page"]["asset"], json!(expected));
                let advent::Response::Asset { page } = serde_json::from_value(response).unwrap()
                else {
                    panic!("expected image bytes");
                };
                assert_eq!(page.offset, 0);
                assert!(page.bytes == own_bytes);
            }
        }
    }
}
