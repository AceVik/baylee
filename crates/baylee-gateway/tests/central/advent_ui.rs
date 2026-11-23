//! The native calendar fixture only runs inside the owned, disposable integration DB.
#[path = "advent_ui/table.rs"]
mod table;
use super::{Account, Endpoint, PASSWORD, Runtime, common, ui_fixture};
use baylee_account::decks::DeckDraft;
use central_api::advent::{
    DoorState, Request, Response,
    puzzle::{Goal, Line, Move, Phase, Puzzle, Step, Unit},
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, Statement,
    TransactionTrait, Value,
};
use std::io::Cursor;
use uuid::Uuid;
#[path = "advent_ui/captures.rs"]
mod captures;
use captures::check_captures;

fn statement(sql: &str, args: impl IntoIterator<Item = Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, args)
}

fn position() -> Puzzle {
    Puzzle {
        goal: Goal::Defeat,
        life: [8, 1],
        mana: 0,
        units: [
            vec![Unit {
                power: 1,
                toughness: 2,
                keywords: vec![],
                tapped: false,
            }],
            vec![],
        ],
        hand: vec![],
        steps: vec![Step {
            new_turn: None,
            draw: None,
            phase: Phase::Attack { guards: [None; 3] },
        }],
    }
}

async fn art(db: &DatabaseTransaction, n: u8) -> Uuid {
    let (kind, width, height) = if n == 0 {
        ("hero", 600_u32, 400_u32)
    } else {
        ("sleeve", 180, 252)
    };
    let pixels = image::RgbImage::from_fn(width, height, |x, y| {
        let band = ((x + y / 2) / (width / 8)) % 3;
        let star = ((x.wrapping_mul(17) + y.wrapping_mul(31) + u32::from(n)) % 229) < 2;
        image::Rgb(if star {
            [244, 225, 168]
        } else if band == 0 {
            [22 + n, 64, 84]
        } else {
            [32 + n, 87 + (y * 37 / height) as u8, 104]
        })
    });
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(pixels)
        .write_to(&mut encoded, image::ImageFormat::WebP)
        .unwrap();
    let name = format!("Fixture aurora {n:02}");
    let bytes = encoded.into_inner();
    db.query_one_raw(statement(
        "INSERT INTO central_identity.advent_art
        (kind,title_de,title_en,mime,width,height,size,digest,content)
        VALUES ($1,$2,$2,'image/webp',$3,$4,octet_length($5::bytea),sha256($5),$5) RETURNING id",
        [
            kind.into(),
            name.into(),
            i32::try_from(width).unwrap().into(),
            i32::try_from(height).unwrap().into(),
            bytes.into(),
        ],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "id")
    .unwrap()
}

async fn campaign(operator: &DatabaseConnection) -> (u16, Uuid) {
    let tx = operator.begin().await.unwrap();
    let hero = art(&tx, 0).await;
    let row = tx.query_one_raw(statement("WITH period AS (SELECT statement_timestamp()-interval '2 hours' AS opens)
        INSERT INTO central_identity.advent_campaigns(year, starts_at, closes_at, hero_id)
        SELECT extract(year FROM (statement_timestamp() AT TIME ZONE 'UTC'))::smallint, opens, opens+interval '888 hours', $1 FROM period RETURNING year", [hero.into()])).await.unwrap().unwrap();
    let year: i16 = row.try_get("", "year").unwrap();
    let puzzle = serde_json::to_string(&position()).unwrap();
    let mut first = None;
    for day in 1..=24_u8 {
        let reward = art(&tx, day).await;
        first.get_or_insert(reward);
        tx.execute_raw(statement("INSERT INTO central_identity.advent_doors
            (year,door,puzzle,hint1_de,hint1_en,hint2_de,hint2_en,art_id) VALUES
            ($1,$2,$3,'Schau auf die Lebenspunkte.','Check the opposing life total.','Der Weg ist frei.','There are no blockers.',$4)",
            [year.into(),i16::from(day).into(),puzzle.clone().into(),reward.into()])).await.unwrap();
    }
    tx.commit().await.unwrap();
    (year.try_into().unwrap(), first.unwrap())
}

fn signed(endpoint: &Endpoint) -> Account {
    Account::login(
        endpoint.clone(),
        "synthetic".into(),
        PASSWORD.into(),
        Vec::new(),
    )
    .unwrap()
}

pub(super) fn exercise(
    rt: &Runtime,
    operator: &DatabaseConnection,
    endpoint: &Endpoint,
    host: &common::Gateway,
    origin: &str,
) {
    let (year, reward) = rt.block_on(campaign(operator));
    let mut account = signed(endpoint);
    let Response::Calendar {
        doors,
        hero: Some(hero),
        ..
    } = account.advent(Request::Calendar { year }).unwrap()
    else {
        panic!("missing campaign")
    };
    assert_eq!(doors.len(), 24);
    assert_eq!(doors[0].state, DoorState::Open);
    assert!(doors[1..].iter().all(|d| d.state == DoorState::Locked));
    account
        .advent_asset(central_api::advent::AssetScope::Calendar { year }, &hero)
        .unwrap();
    let ids = ["00 Advent QA A", "01 Advent QA B"].map(|name| {
        account
            .save_deck(
                None,
                None,
                DeckDraft {
                    name: name.into(),
                    format: "Freeform".into(),
                    cards: vec!["60 Island".into()],
                    ..Default::default()
                },
            )
            .unwrap()
            .id
    });
    if std::env::var_os("BAYLEE_CENTRAL_UI_READY").is_some() {
        drop(account);
        ui_fixture(rt, endpoint, host, origin);
        check_captures();
    } else {
        // Same signed API and fixed synthetic choices as the UI path.
        for (attackers, expected) in [(0, false), (1, true)] {
            let line = Line(vec![Move::Attack { attackers }]);
            assert!(
                matches!(account.advent(Request::Submit{year,door:1,line}).unwrap(),Response::Submission{won} if won==expected)
            );
        }
        assert!(
            matches!(account.advent(Request::Claim{year,door:1}).unwrap(),Response::Reward{reward:r} if r.id==*reward.as_bytes())
        );
        let a = account.load_deck(ids[0]).unwrap();
        let mut draft = a.draft;
        draft.look.sleeve = Some(*reward.as_bytes());
        account
            .save_deck(Some(ids[0]), Some(a.revision), draft)
            .unwrap();
    }
    let mut restored = signed(endpoint);
    let a = restored.load_deck(ids[0]).unwrap();
    let b = restored.load_deck(ids[1]).unwrap();
    assert_eq!(a.draft.look.sleeve, Some(*reward.as_bytes()));
    assert!(b.draft.look.is_empty());
    assert!(
        matches!(restored.advent(Request::Inventory {after:None}).unwrap(), Response::Inventory{items,..} if items.len()==1 && items[0].id==*reward.as_bytes())
    );
    table::exercise(rt, operator, host, origin, endpoint, &restored, &a.draft);
}
