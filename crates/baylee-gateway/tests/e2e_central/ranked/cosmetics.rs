//! One real 1v1 settlement opens the catalog entries used by the signed native client.
use super::{Served, capacity, consent, fresh, game, player, receipts, registered, replication};
use baylee_account::{
    Account, advent,
    decks::DeckDraft,
    ranked::{DeckPreview, Format, League, PoolEntry, QueueState},
};
use central_postgres::AccountStore;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};
use tokio::runtime::Runtime;
use uuid::Uuid;

fn collection(account: &Account) -> Vec<advent::collection::Item> {
    let advent::Response::Collection { items, next: None } = account
        .advent(advent::Request::Collection { after: None })
        .unwrap()
    else {
        panic!("expected one page of individually authorized synthetic collection items");
    };
    items
}

async fn prepare(db: &DatabaseConnection, players: &[super::Player; 2]) {
    let tx = db.begin().await.unwrap();
    for (entry, elo) in players.iter().zip([1730_i32, 1770]) {
        // Simulate prior history only. Gold II requires 620 games and 1760 Elo.
        tx.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO central_identity.ranked_ratings
             (season_id,format,account_id,elo,queue_elo,score,placement,games,wins,losses,league,division)
             SELECT id,'standard',$1,$2,$2,$2::int,$2::int,619,320,299,5,3
             FROM central_identity.ranked_seasons WHERE starts_at<=now() AND (ends_at IS NULL OR ends_at>now())",
            [Uuid::from_bytes(entry.account.public_id()).into(),elo.into()])).await.unwrap();
    }
    for (kind, title, variant) in [
        ("sleeve", "QA Sunburst", 1),
        ("playmat", "QA Gilded Hall", 2),
    ] {
        let (width, height, bytes) = super::super::advent_ui::art_file(variant);
        let art = tx.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO central_identity.advent_art
             (kind,title_de,title_en,mime,width,height,size,digest,content,source)
             VALUES ($1,$2,$2,'image/webp',$3,$4,octet_length($5::bytea),sha256($5),$5,'ranked') RETURNING id",
            [kind.into(),title.into(),i32::try_from(width).unwrap().into(),i32::try_from(height).unwrap().into(),bytes.into()]))
            .await.unwrap().unwrap().try_get::<Uuid>("","id").unwrap();
        tx.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.ranked_cosmetic_catalog(league,division,kind,art_id)
             VALUES (5,2,$1,$2)",
            [kind.into(), art.into()],
        ))
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
}

pub(in super::super) struct Earned {
    deck: Uuid,
    score: DeckPreview,
}

fn score(account: &Account, deck: &DeckDraft) -> DeckPreview {
    let registration = baylee_cards::ranked::registration(
        &deck.format,
        &deck.cards,
        &deck.sideboard,
        &deck.commanders,
    )
    .unwrap();
    let pool = baylee_cards::ranked::rating_pool(&registration)
        .into_iter()
        .map(|(card, copies)| PoolEntry { card, copies })
        .collect();
    account.preview_ranked_deck(Format::Standard, pool).unwrap()
}

impl Earned {
    pub(in super::super) fn verify(&self, served: &Served) {
        let mut owner = Account::login(
            served.endpoint.clone(),
            "synthetic".into(),
            super::super::PASSWORD.into(),
            Vec::new(),
        )
        .unwrap();
        let owned = collection(&owner);
        let deck = owner.load_deck(self.deck).unwrap().draft;
        for (kind, slot) in [
            (advent::Kind::Sleeve, deck.look.sleeve),
            (advent::Kind::Playmat, deck.look.playmat),
        ] {
            assert!(owned.iter().any(|item| item.kind == kind
                && Some(item.id) == slot
                && item.source
                    == advent::collection::Source::Ranked {
                        league: League::Gold,
                        division: Some(2)
                    }));
        }
        assert_eq!(
            score(&owner, &deck),
            self.score,
            "cosmetics preserve the exact ranked version and Elo"
        );
    }
}

pub(in super::super) fn earn(
    rt: &Runtime,
    served: &Served,
    store: &AccountStore,
    operator: &DatabaseConnection,
) -> Earned {
    let rival_name = "cosmetic_rival";
    let rival = fresh::register(
        rt,
        &served.service,
        &served.endpoint,
        rival_name,
        fresh::DOCUMENT,
    );
    let owner = Account::login(
        served.endpoint.clone(),
        "synthetic".into(),
        super::super::PASSWORD.into(),
        Vec::new(),
    )
    .unwrap();
    let host = registered(rt, served, store, operator, rival_name, true);
    let agent = rt.block_on(super::common::ranked::provision(&host.gateway));
    capacity(rt, operator, &host);
    let mut players = [
        player(rival, served, &host, 60),
        player(owner, served, &host, 61),
    ];
    rt.block_on(prepare(operator, &players));
    assert!(collection(&players[1].account).is_empty());
    let assignments = replication::join(&players);
    consent::approve(&players, assignments[0].match_id, &[&host], |_| {});
    game::seats(rt, served, &host, &players, &assignments);
    let QueueState::Finished { before, after, .. } = receipts::receipt(&players[1]) else {
        panic!("the engine verdict must settle in Central");
    };
    assert_eq!((before.league, before.division), (League::Gold, Some(3)));
    assert_eq!((after.league, after.division), (League::Gold, Some(2)));
    let items = collection(&players[1].account);
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item.source
        == advent::collection::Source::Ranked {
            league: League::Gold,
            division: Some(2)
        }));
    assert!(collection(&players[0].account).is_empty());
    agent.abort();
    let owner = &mut players[1].account;
    let deck = owner
        .list_decks()
        .unwrap()
        .into_iter()
        .find(|row| row.name == "Private forest 61")
        .unwrap()
        .id;
    let draft = owner.load_deck(deck).unwrap().draft;
    Earned {
        deck,
        score: score(owner, &draft),
    }
}
