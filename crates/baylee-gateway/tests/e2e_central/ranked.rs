//! Full native account → Central queue → selected gateway → two engine seats → private receipts.
//! Both public gateway origins are the actual loopback ports bound by their processes.
use super::{CentralHttp, Served, common, fresh, sdk, token};
use baylee_account::{
    Account, RankedDeck, Token,
    ranked::{Assignment, Format, Id, Outcome, QueueState, Request, Response},
    wire::Discovery,
};
use central_postgres::AccountStore;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[path = "ranked/consent.rs"]
mod consent;
#[path = "ranked/game.rs"]
mod game;
#[path = "ranked/receipts.rs"]
mod receipts;
#[path = "ranked/relay.rs"]
mod relay;
#[path = "ranked/replication.rs"]
pub(super) mod replication;

#[test]
#[ignore = "requires owned PostgreSQL 18 fixture from cargo dev central integration --ranked-only"]
fn ranked_two_gateways_settle_and_restore_home() {
    super::integration(super::Scenario::Ranked);
}

struct Registered {
    gateway: common::Gateway,
    id: Id,
    token: Token,
    origin: String,
}

fn registered(
    rt: &Runtime,
    served: &Served,
    store: &AccountStore,
    operator: &DatabaseConnection,
    username: &str,
    verified: bool,
) -> Registered {
    let (gateway, admin) = token::spawn_with_terms(&served.endpoint, verified);
    let origin = format!("http://127.0.0.1:{}", gateway.port);
    let http = CentralHttp::new(served.endpoint.clone());
    let Discovery::Central { identity } = sdk::discover(&http, username.to_owned()).unwrap() else {
        panic!("fixture account is enrolled at Central");
    };
    let (_, token, id) = token::minted(rt, store, operator, identity, &origin, verified);
    assert_eq!(
        token::paste(admin, &baylee_account::owner::token_text(&token)).0,
        200
    );
    common::central_ready(gateway.port);
    if verified {
        consent::wait_for_terms(&http, &origin);
    }
    Registered {
        gateway,
        id,
        token,
        origin,
    }
}

struct Player {
    account: Account,
    key: Id,
    deck: RankedDeck,
    commitment: [u8; 32],
    home: baylee_account::Admission,
}

fn player(mut account: Account, served: &Served, gateway: &Registered, count: u32) -> Player {
    // Exercise the private library the real ranked client reads before joining.
    let id = account
        .save_deck(
            None,
            None,
            baylee_account::decks::DeckDraft {
                name: format!("Private forest {count}"),
                format: "standard".into(),
                cards: vec![format!("{count} Forest")],
                ..Default::default()
            },
        )
        .unwrap()
        .id;
    let draft = account.load_deck(id).unwrap().draft;
    let commitment = baylee_cards::ranked::registration(
        &draft.format,
        &draft.cards,
        &draft.sideboard,
        &draft.commanders,
    )
    .unwrap()
    .digest;
    let deck = RankedDeck {
        format: Format::Standard,
        cards: draft.cards,
        sideboard: draft.sideboard,
        commanders: draft.commanders,
    };
    if let Some(doc) =
        sdk::gateway_terms(&CentralHttp::new(served.endpoint.clone()), &gateway.origin).unwrap()
    {
        account
            .accept_terms(
                baylee_account::terms::TermsSubject::Gateway {
                    origin: gateway.origin.clone(),
                },
                &doc.version,
            )
            .unwrap();
    }
    let home = account.admit(&gateway.origin).unwrap();
    assert_eq!(
        common::http(gateway.gateway.port, "GET", "/me", Some(&home.token), "").0,
        200
    );
    Player {
        account,
        key: *Uuid::now_v7().as_bytes(),
        deck,
        commitment,
        home,
    }
}

/// Query the authority's own store until the real host heartbeat has observed a
/// compatible connected agent and a catalog. Tests never manufacture a Ready.
fn capacity(rt: &Runtime, operator: &DatabaseConnection, host: &Registered) {
    rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(65), async {
            loop {
                let query = Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM central_identity.ranked_hosts
                      WHERE host_id=$1 AND capacity>0) AS ready",
                    [Uuid::from_bytes(host.id).into()],
                );
                let row = operator.query_one_raw(query).await.unwrap().unwrap();
                if row.try_get::<bool>("", "ready").unwrap() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(180)).await;
            }
        })
        .await
        .expect("verified gateway's real heartbeat never published available engine capacity");
    });
}

pub(super) fn run(
    rt: &Runtime,
    served: &Served,
    store: &AccountStore,
    operator: &DatabaseConnection,
) {
    let names = ["ranked_one", "ranked_two"];
    let one = fresh::register_as(
        rt,
        &served.service,
        &served.endpoint,
        names[0],
        None,
        fresh::DOCUMENT,
    );
    let two = fresh::register(
        rt,
        &served.service,
        &served.endpoint,
        names[1],
        fresh::DOCUMENT,
    );
    let home = registered(rt, served, store, operator, names[0], false);
    let host = registered(rt, served, store, operator, names[1], true);
    let agent = rt.block_on(common::ranked::provision(&host.gateway));
    capacity(rt, operator, &host);
    let players = [
        player(one, served, &home, 60),
        player(two, served, &host, 61),
    ];
    receipts::before(served, &players);
    let original = [&home, &host].map(|g| g.gateway.port);
    let me = std::array::from_fn::<_, 2, _>(|i| {
        common::http(original[i], "GET", "/me", Some(&players[i].home.token), "").1
    });
    for p in &players {
        assert_eq!(
            p.account
                .ranked(Request::Join {
                    key: p.key,
                    format: Format::Standard,
                    deck: p.commitment
                })
                .unwrap_err(),
            baylee_account::Error::UpdateRequired
        );
    }
    let seats = replication::join(&players);
    for assignment in &seats {
        assert_eq!(assignment.host_id, host.id);
        assert_eq!(assignment.origin, host.origin);
        assert_eq!(assignment.format, Format::Standard);
    }
    assert_eq!(seats[0].match_id, seats[1].match_id);
    assert_ne!(seats[0].seat, seats[1].seat);
    assert_ne!(seats[0].account, seats[1].account);
    consent::approve(&players, seats[0].match_id, &[&host], |_| {
        consent::unopened(rt, operator, seats[0].match_id);
    });
    relay::admissions(rt, served, &host, &players, seats[0].match_id);
    let game = game::seats(rt, served, &host, &players, &seats);
    let outcome = if seats[0].seat == 0 {
        Outcome::SecondWins
    } else {
        Outcome::FirstWins
    };
    receipts::verify(served, &home, &host, &players, &seats, game, outcome);
    for (i, p) in players.iter().enumerate() {
        let (status, body) = common::http(original[i], "GET", "/me", Some(&p.home.token), "");
        assert_eq!(
            status, 200,
            "original lobby session still available after remote play"
        );
        assert_eq!(
            common::json_field(&me[i], "id"),
            common::json_field(&body, "id")
        );
    }
    agent.abort();
}
