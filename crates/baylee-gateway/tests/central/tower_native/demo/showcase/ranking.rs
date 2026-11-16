//! Explicit synthetic podium; never a verified result or production seed path.
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use uuid::Uuid;

pub(super) async fn prepare(db: &DatabaseConnection, schema: &str, cohort: Uuid, owner: Uuid) {
    for (at, (name, wins, defeats)) in [
        ("Sylva", 8_i16, 0_i16),
        ("Rowan", 8, 0),
        ("Briar", 8, 1),
        ("Alder", 8, 3),
        ("Willow", 7, 5),
        ("Thorne", 5, 5),
    ]
    .into_iter()
    .enumerate()
    {
        let account = Uuid::now_v7();
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            format!("INSERT INTO \"{schema}\".players(account,echo_consent,rank_revision,rank_listed,rank_name,rank_tag) VALUES($1,false,1,true,$2,$3)"),
            [account.into(), name.into(), (8192_i64 + i64::try_from(at).unwrap()).into()])).await.unwrap();
        for house in [false, true] {
            db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                format!("INSERT INTO \"{schema}\".personal_bests(account,cohort,house,wins,defeats,listed) VALUES($1,$2,$3,$4,$5,true)"),
                [account.into(), cohort.into(), house.into(), wins.into(), defeats.into()])).await.unwrap();
        }
    }
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        format!("INSERT INTO \"{schema}\".personal_bests(account,cohort,house,wins,defeats,listed) VALUES($1,$2,true,8,1,false)"),
        [owner.into(), cohort.into()])).await.unwrap();
}
