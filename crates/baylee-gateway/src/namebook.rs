//! The names the lobby shows, read from the database once per account.
//!
//! Every lobby feed renders the listing again whenever the lobby moves, and
//! the listing names the people at every table. Asking the database for
//! those names on each render made one change cost a query per open feed:
//! measured with 200 feeds, a room opened and left again took 30 ms at the
//! median to answer, because the 400 queries it set off held every
//! connection in the pool (`tests/load.rs`).
//!
//! A handle never changes once its account exists: no route renames an
//! account, and the tag is drawn when it is made. So a name read is a name
//! for good, and the book only has to learn the accounts that sat down
//! since it last looked, and forget the ones that stood up. A deleted
//! account leaves every table (`Lobby::forget_account`) before the lobby
//! says it moved, so it is gone from the book at the next render.

use std::collections::HashMap;
use std::sync::Arc;

/// Display handles by account id, for the accounts seated in the lobby.
#[derive(Default)]
pub struct NameBook {
    /// What the last render knew. Replaced whole, never edited in place, so
    /// a render holding the old map keeps reading a consistent one.
    ///
    /// A `tokio` mutex, held across the query on purpose: when a change
    /// wakes every feed at once, the first one asks and the rest wait for
    /// its answer instead of asking the same question beside it.
    known: tokio::sync::Mutex<Arc<HashMap<String, String>>>,
    /// How many times the database was asked, for the tests that say when.
    queries: std::sync::atomic::AtomicU64,
}

impl NameBook {
    /// The handles of `wanted` (account ids), from the book where it has
    /// them and from the database for the rest. An account the database
    /// does not know is left out, as the query alone left it out.
    pub async fn names(
        &self,
        db: &sea_orm::DatabaseConnection,
        wanted: Vec<String>,
    ) -> Arc<HashMap<String, String>> {
        let mut known = self.known.lock().await;
        let missing: Vec<String> = wanted
            .iter()
            .filter(|id| !known.contains_key(*id))
            .cloned()
            .collect();
        let surplus = known.len() + missing.len() > wanted.len();
        if missing.is_empty() && !surplus {
            return Arc::clone(&known);
        }
        let mut next: HashMap<String, String> = wanted
            .iter()
            .filter_map(|id| known.get(id).map(|name| (id.clone(), name.clone())))
            .collect();
        if !missing.is_empty() {
            self.queries
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            match crate::store::display_names(db, missing).await {
                Ok(found) => next.extend(found),
                // Not remembered: the next render asks again, as every
                // render used to.
                Err(e) => tracing::error!("{e:#}"),
            }
        }
        *known = Arc::new(next);
        Arc::clone(&known)
    }
}

#[cfg(test)]
mod tests {
    //! Against real account rows in a schema of their own: what the book
    //! holds is a question about which rows it read and when. Without
    //! `DATABASE_URL` they fail rather than skip.

    use super::*;
    use sea_orm::{ConnectionTrait, Database};

    fn queries(book: &NameBook) -> u64 {
        book.queries.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn guest(db: &sea_orm::DatabaseConnection, name: &str) -> String {
        crate::store::create_guest(
            db,
            crate::store::NewGuest {
                display_name: name.to_owned(),
                created_at: 1_700_000_000,
                lang: "en".to_owned(),
            },
            crate::auth::token_digest(&format!("token-{name}")),
            1_800_000_000,
            None,
        )
        .await
        .expect("a guest")
        .expect("admitted")
        .id
    }

    /// One query per account that sat down, none for a render that finds
    /// nobody new, and nobody kept who stood up.
    #[tokio::test]
    async fn a_name_is_read_once_and_forgotten_when_its_account_stands_up() {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .expect("DATABASE_URL is not set, and this test is about PostgreSQL rows");
        let schema = format!("t_namebook_{}", std::process::id());
        let admin = Database::connect(&url).await.expect("connecting");
        admin
            .execute_unprepared(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
            .await
            .expect("clearing a schema");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("creating a schema");
        let sep = if url.contains('?') { '&' } else { '?' };
        let scoped = format!("{url}{sep}options=-c%20search_path%3D{schema},public");
        let db = baylee_db::connect(&scoped, 2).await.expect("the store");
        let ada = guest(&db, "Ada").await;
        let bo = guest(&db, "Bo").await;
        let nobody = uuid::Uuid::now_v7().to_string();

        let book = NameBook::default();
        let names = book.names(&db, vec![ada.clone()]).await;
        assert!(names[&ada].starts_with("Ada"), "{names:?}");
        assert_eq!(queries(&book), 1);

        let again = book.names(&db, vec![ada.clone()]).await;
        assert!(
            Arc::ptr_eq(&names, &again),
            "a render with nobody new is the same map"
        );
        assert_eq!(queries(&book), 1, "and asks nothing");

        let both = book.names(&db, vec![ada.clone(), bo.clone()]).await;
        assert_eq!(both.len(), 2);
        assert!(both[&bo].starts_with("Bo"), "{both:?}");
        assert_eq!(queries(&book), 2, "one query, for the one who sat down");

        let left = book.names(&db, vec![bo.clone()]).await;
        assert_eq!(left.len(), 1, "who stood up is forgotten: {left:?}");
        assert_eq!(queries(&book), 2, "without asking");
        let back = book.names(&db, vec![ada.clone(), bo.clone()]).await;
        assert_eq!(back.len(), 2);
        assert_eq!(queries(&book), 3, "and read again when they sit back down");

        let unknown = book.names(&db, vec![bo.clone(), nobody.clone()]).await;
        assert_eq!(
            unknown.len(),
            1,
            "an account the database lacks is left out"
        );
        assert!(!unknown.contains_key(&nobody));

        admin
            .execute_unprepared(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
            .await
            .expect("dropping the schema");
    }
}
