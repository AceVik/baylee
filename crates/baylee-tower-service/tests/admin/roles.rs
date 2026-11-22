use super::{Client, id, support};
use baylee_tower_store::Role;
use central_api::Token;
use sea_orm::{ConnectOptions, ConnectionTrait as _, Database, DbBackend, Statement};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn runtime_table_grants_and_valid_operator_tokens_do_not_grant_administration() {
    let sandbox = support::Sandbox::open().await;
    let role = format!("t_tower_role_{}", Uuid::now_v7().simple());
    let password = Token::generate().unwrap();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let owner = Client::new(&schema);
    let mut restricted = Client::new(&schema);
    let mut url =
        url::Url::parse(restricted.configuration["database_url"].as_str().unwrap()).unwrap();
    url.set_username(&role).unwrap();
    url.set_password(Some(password.hex().as_str())).unwrap();
    restricted.configuration["database_url"] = json!(url.as_str());
    let fixture_role = role.clone();
    let observed = tokio::spawn(async move {
        // Generated identifier and token hex only; SQL/value logging is disabled.
        db.execute_unprepared(&format!("CREATE ROLE \"{fixture_role}\" LOGIN PASSWORD '{}'", password.hex().as_str()))
            .await.map_err(|_| ()).expect("owned synthetic login created");
        db.execute_unprepared(&format!("GRANT USAGE ON SCHEMA \"{schema}\" TO \"{fixture_role}\";
            GRANT SELECT,INSERT,UPDATE,DELETE ON ALL TABLES IN SCHEMA \"{schema}\" TO \"{fixture_role}\""))
            .await.map_err(|_| ()).expect("synthetic runtime table grants applied");
        confirm_login(url.as_str(), &fixture_role).await;
        let token = Token::generate().unwrap();
        let provision = json!({"action": "provision", "name": "synthetic runtime host",
            "role": Role::Host, "credential": token.hex().as_str()});
        let host = id(&owner.ok(provision.clone()).await, "id");
        let publication = support::publication();
        let cohort = id(&owner.ok(json!({"action": "publish", "publication": publication})).await, "cohort");
        for operation in [provision,
            json!({"action": "revoke", "operator": host}),
            json!({"action": "pause_enrollment", "cohort": cohort}),
            json!({"action": "publish", "publication": publication}),
            json!({"action": "approve_host", "host": host, "origin": "http://127.0.0.1:29875"}),
            json!({"action": "publish_house", "publisher": token.hex().as_str(), "cohort": cohort,
                "stage": 1, "archetype": 1, "build": {"deck": publication.starters[0].deck,
                    "active": [], "potions": [0, 0]}})] {
            restricted.refused(operation).await;
        }
    }).await;
    // Reap commands before removing their grants and the owned login, even on panic.
    sandbox
        .db
        .execute_unprepared(&format!("DROP OWNED BY \"{role}\"; DROP ROLE \"{role}\""))
        .await
        .map_err(|_| ())
        .expect("owned synthetic role removed");
    sandbox.close().await;
    observed.expect("runtime grants cannot administer");
}

async fn confirm_login(database_url: &str, role: &str) {
    let mut options = ConnectOptions::new(database_url);
    options.max_connections(1).sqlx_logging(false);
    let db = Database::connect(options)
        .await
        .map_err(|_| ())
        .expect("restricted login connects to the owned database");
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_user::text AS name",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "name").unwrap(), role);
    db.close().await.unwrap();
}
