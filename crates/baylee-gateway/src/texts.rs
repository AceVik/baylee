//! Card text, held in memory per language until the catalog's data stamp
//! moves (#220).
//!
//! The text a gateway serves is chosen over every printing of a card in the
//! asked language (`Catalog::text_by_card`), so one language's text for the
//! whole pool is 13 441 face rows out of Postgres in German, where the rule
//! it replaced read 2 813 (measured 2026-09-24). Every `/pool` and every
//! game's `/catalog/text` asked for it again, and nothing in it changes
//! between two requests unless the catalog does.
//!
//! # When it is stale
//!
//! An ingest runs as a process of its own while the gateway serves, so a
//! cache that lived until a restart would serve the old text after one. The
//! key is `Catalog::data_version`, which `ingest::bulk` moves when it starts
//! and when it ends; every request reads it, one primary-key lookup, and a
//! language held at another version is built again. Another, not an older
//! one: a catalog restored from a backup goes back to the stamp it was
//! saved at. It is not moved per `upsert`: an ingest writes about 1 356
//! batches, and a stamp per batch would rebuild every language on nearly
//! every request for the three minutes an ingest runs.
//!
//! No gateway writes cards into the catalog. It used to fill a
//! printing it lacked from Scryfall (`/catalog/text?ids=`), without moving
//! the stamp, which left a second gateway on the same database serving the
//! older pick until the next ingest; #270 took the fill out, and the limit
//! with it.
//!
//! # What is held
//!
//! Only the languages the catalog names (its `languages` table); any other
//! code is answered as English, which is what the catalog would have served
//! for it anyway. Memory is therefore bounded by the catalog's languages
//! times the pool, and a request cannot make this process build and keep a
//! language per code it made up. Every name of every pool card, which
//! `/pool` sends in each language, is held once for all of them.
//!
//! The list is read once, after the gateway has migrated the catalog,
//! which seeds it. A language a self-hoster adds to it is held after a
//! restart.

use crate::pool;
use baylee_catalog::{CardTextEntry, Catalog, LocalName};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// One language's text for the pool, at one data version.
pub struct Language {
    /// The catalog's data version this was read at.
    version: i64,
    /// The language it is in.
    lang: String,
    /// Every pool card's text, by oracle id.
    by_card: HashMap<String, CardTextEntry>,
    /// `/pool`'s whole answer in this language, serialized once.
    pool: bytes::Bytes,
}

impl Language {
    /// One card's text, when it is a pool card.
    pub fn entry(&self, oracle_id: &str) -> Option<&CardTextEntry> {
        self.by_card.get(oracle_id)
    }

    /// The language the entries are in (after the fallback to English).
    pub fn lang(&self) -> &str {
        &self.lang
    }

    /// `/pool`'s answer, as JSON.
    pub fn pool_json(&self) -> bytes::Bytes {
        self.pool.clone()
    }

    /// Reads the pool's text in `lang` at `version`.
    async fn read(
        catalog: &Catalog,
        lang: &str,
        version: i64,
        names: &[LocalName],
    ) -> anyhow::Result<Self> {
        let by_card = catalog
            .text_by_card(pool_cards(), lang)
            .await?
            .into_iter()
            .map(|entry| (entry.oracle_id.clone(), entry))
            .collect();
        Self::assemble(version, lang.to_owned(), by_card, names)
    }

    fn assemble(
        version: i64,
        lang: String,
        by_card: HashMap<String, CardTextEntry>,
        names: &[LocalName],
    ) -> anyhow::Result<Self> {
        let body = pool::body(&lang, &by_card, names);
        let pool = bytes::Bytes::from(serde_json::to_vec(&body)?);
        Ok(Self {
            version,
            lang,
            by_card,
            pool,
        })
    }
}

/// The pool's cards, by oracle id, sorted, each once.
fn pool_cards() -> &'static [String] {
    static CARDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    CARDS.get_or_init(|| {
        let mut cards: Vec<String> = baylee_cards::pool::rows()
            .iter()
            .map(|c| c.oracle_id.to_owned())
            .filter(|id| !id.is_empty())
            .collect();
        cards.sort_unstable();
        cards.dedup();
        cards
    })
}

/// One language's slot. Its own async lock is what makes a burst of
/// requests after a bump read the language once, while two languages read
/// side by side.
type Slot = Arc<tokio::sync::Mutex<Option<Arc<Language>>>>;

/// Every language this process holds.
#[derive(Default)]
pub struct TextCache {
    /// The languages the catalog names, read once.
    known: tokio::sync::OnceCell<HashSet<String>>,
    slots: Mutex<HashMap<String, Slot>>,
    /// Every pool card's name in every language, and the version it was
    /// read at. `/pool` sends all of them whatever language it is asked
    /// in, so the languages share one copy: 24 961 names, about 4 MB
    /// apiece (measured 2026-09-24), would otherwise be held nineteen
    /// times.
    names: tokio::sync::Mutex<Option<(i64, Arc<Vec<LocalName>>)>>,
    /// How many times a whole language was read from the catalog: what
    /// the tests hold "once per version" against.
    reads: std::sync::atomic::AtomicU64,
}

impl TextCache {
    /// The language a request for `lang` is answered in: `lang` when the
    /// catalog has a name for it, English otherwise.
    ///
    /// # Errors
    /// When the catalog's languages cannot be read.
    pub async fn language_of(&self, catalog: &Catalog, lang: &str) -> anyhow::Result<String> {
        let known = self
            .known
            .get_or_try_init(|| async {
                Ok::<_, anyhow::Error>(catalog.languages().await?.into_iter().collect())
            })
            .await?;
        Ok(if known.contains(lang) {
            lang.to_owned()
        } else {
            "en".to_owned()
        })
    }

    /// The pool's text in `lang`, read again when the catalog moved since
    /// it was last read.
    ///
    /// # Errors
    /// When the catalog cannot be read. Nothing is kept then, so the next
    /// request tries again.
    pub async fn get(&self, catalog: &Catalog, lang: &str) -> anyhow::Result<Arc<Language>> {
        let lang = self.language_of(catalog, lang).await?;
        // Read before the language is, so a write landing mid-read leaves
        // it stamped older than what it may already contain, and the next
        // request reads it again.
        let mut version = catalog.data_version().await?;
        let slot = self.slot(&lang);
        let mut held = slot.lock().await;
        if let Some(language) = held.as_ref() {
            // Held at a newer stamp than this request read: either another
            // request built it after an ingest moved the stamp, which must
            // not cost a second read, or the catalog was restored to an
            // older stamp, which must. Asking again tells them apart.
            if language.version > version {
                version = catalog.data_version().await?;
            }
            if language.version == version {
                return Ok(Arc::clone(language));
            }
        }
        self.reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let names = self.names_at(catalog, version).await?;
        let language = Arc::new(Language::read(catalog, &lang, version, &names).await?);
        *held = Some(Arc::clone(&language));
        Ok(language)
    }

    /// Every pool card's name at `version`, read when the ones held are
    /// from another.
    ///
    /// Another rather than an older one, for the reason [`Self::get`]
    /// gives. A request that read the stamp just before it moved reads the
    /// names once more than it had to, which is cheaper than telling it
    /// apart here.
    async fn names_at(
        &self,
        catalog: &Catalog,
        version: i64,
    ) -> anyhow::Result<Arc<Vec<LocalName>>> {
        let mut held = self.names.lock().await;
        if let Some((at, names)) = held.as_ref()
            && *at == version
        {
            return Ok(Arc::clone(names));
        }
        let names = Arc::new(catalog.names(pool_cards()).await?);
        *held = Some((version, Arc::clone(&names)));
        Ok(names)
    }

    fn slot(&self, lang: &str) -> Slot {
        Arc::clone(self.slots.lock().entry(lang.to_owned()).or_default())
    }
}

#[cfg(test)]
mod tests {
    //! Against a real catalog in a schema of its own, as the catalog's own
    //! tests are: what is held, and when it is read again, is a question
    //! about Postgres rows and a stamp another connection moves. Without
    //! `DATABASE_URL` they fail rather than skip.

    use super::*;
    use baylee_catalog::scryfall;
    use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
    use std::sync::atomic::{AtomicU32, Ordering};

    const OLD: &str = "{T}: Erhöhe deinen Manavorrat um {1}.\n{1}, {T}, opfere den Gedankenstein: Ziehe eine Karte.";
    const NEW: &str = "{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.";

    struct Sandbox {
        catalog: Catalog,
        admin: DatabaseConnection,
        schema: String,
    }

    static NTH: AtomicU32 = AtomicU32::new(0);

    impl Sandbox {
        async fn open(what: &str) -> Self {
            let url = std::env::var("DATABASE_URL")
                .ok()
                .filter(|u| !u.is_empty())
                .expect("DATABASE_URL is not set, and these tests are about PostgreSQL");
            let schema = format!(
                "t_texts_{what}_{}_{}",
                std::process::id(),
                NTH.fetch_add(1, Ordering::Relaxed)
            );
            let admin = Database::connect(&url).await.expect("connecting");
            admin
                .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
                .await
                .expect("creating a schema");
            let sep = if url.contains('?') { '&' } else { '?' };
            let scoped = format!("{url}{sep}options=-c%20search_path%3D{schema},public");
            let catalog = Catalog::connect(&scoped).await.expect("connecting scoped");
            catalog
                .migrate()
                .await
                .expect("building the catalog schema");
            Self {
                catalog,
                admin,
                schema,
            }
        }

        async fn close(self) {
            self.admin
                .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
                .await
                .expect("dropping the schema");
        }
    }

    /// Mind Stone, a pool card, as the registry names it.
    fn mind_stone() -> &'static baylee_cards::pool::PoolCard {
        baylee_cards::pool::rows()
            .iter()
            .find(|c| c.english_name == "Mind Stone")
            .expect("Mind Stone is in the pool")
    }

    /// A German printing of it, `n` its id's last digits.
    fn german(n: u32, date: &str, printed: &str) -> scryfall::Card {
        scryfall::Card {
            id: format!("00000000-0000-4000-8000-{n:012}"),
            oracle_id: Some(mind_stone().oracle_id.to_owned()),
            lang: "de".to_owned(),
            set: format!("s{n}"),
            collector_number: "1".to_owned(),
            released_at: Some(date.to_owned()),
            layout: Some("normal".to_owned()),
            name: "Mind Stone".to_owned(),
            printed_name: Some("Gedankenstein".to_owned()),
            type_line: Some("Artifact".to_owned()),
            oracle_text: Some(
                "{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card.".to_owned(),
            ),
            printed_text: Some(printed.to_owned()),
            mana_cost: Some("{2}".to_owned()),
            ..scryfall::Card::default()
        }
    }

    fn printed(language: &Language) -> Option<&str> {
        language.entry(mind_stone().oracle_id)?.faces[0]
            .printed
            .as_deref()
    }

    fn reads(cache: &TextCache) -> u64 {
        cache.reads.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// An ingest writes from a process of its own, so what it wrote is not
    /// seen until it moves the stamp — and then it is.
    #[tokio::test]
    async fn a_language_is_read_once_until_the_catalog_moves() {
        let sandbox = Sandbox::open("once").await;
        let catalog = &sandbox.catalog;
        catalog
            .upsert(&[german(1, "2007-07-13", OLD)])
            .await
            .expect("seeding");
        let cache = TextCache::default();

        let first = cache.get(catalog, "de").await.expect("reading");
        assert_eq!(printed(&first), Some(OLD));
        let again = cache.get(catalog, "de").await.expect("reading");
        assert!(
            Arc::ptr_eq(&first, &again),
            "the second request is served from memory"
        );
        assert_eq!(reads(&cache), 1);

        catalog
            .upsert(&[german(2, "2025-06-13", NEW)])
            .await
            .expect("an ingest's batch");
        let during = cache.get(catalog, "de").await.expect("reading");
        assert_eq!(printed(&during), Some(OLD), "no stamp, no read");
        assert_eq!(reads(&cache), 1);

        catalog
            .bump_data_version()
            .await
            .expect("the ingest's stamp");
        let after = cache.get(catalog, "de").await.expect("reading");
        assert_eq!(printed(&after), Some(NEW));
        assert_eq!(reads(&cache), 2);
        sandbox.close().await;
    }

    /// A catalog restored from a backup brings back the stamp it was saved
    /// at, older than the one held. It is read again, rather than served
    /// from memory until ingests overtake the stamp held.
    #[tokio::test]
    async fn a_catalog_restored_to_an_older_stamp_is_read_again() {
        let sandbox = Sandbox::open("restore").await;
        let catalog = &sandbox.catalog;
        catalog
            .upsert(&[german(1, "2007-07-13", OLD)])
            .await
            .expect("seeding");
        catalog.bump_data_version().await.expect("stamping");
        catalog.bump_data_version().await.expect("stamping");
        let cache = TextCache::default();
        assert_eq!(
            printed(&cache.get(catalog, "de").await.expect("reading")),
            Some(OLD)
        );

        catalog
            .upsert(&[german(2, "2025-06-13", NEW)])
            .await
            .expect("the backup's rows");
        sandbox
            .admin
            .execute_unprepared(&format!(
                "UPDATE \"{}\".catalog_meta SET value = '1' WHERE key = 'data_version'",
                sandbox.schema
            ))
            .await
            .expect("the backup's stamp");
        assert_eq!(
            printed(&cache.get(catalog, "de").await.expect("reading")),
            Some(NEW)
        );
        cache.get(catalog, "de").await.expect("reading");
        assert_eq!(reads(&cache), 2);
        sandbox.close().await;
    }

    /// Requests that arrive together after a stamp read the language once,
    /// not once each.
    ///
    /// Arriving together is arranged, not hoped for: on its own a burst
    /// is serialized by the connection pool, the first request taking the
    /// connection back for its whole read, and would pass without the
    /// slot's lock. So the test holds that lock until every request is
    /// waiting on it, which it sees in the slot's count: a request takes
    /// its handle after it has read the stamp.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_burst_after_the_stamp_moves_reads_the_language_once() {
        const BURST: usize = 8;
        let sandbox = Sandbox::open("burst").await;
        let catalog = &sandbox.catalog;
        catalog
            .upsert(&[german(1, "2007-07-13", OLD)])
            .await
            .expect("seeding");
        let cache = Arc::new(TextCache::default());
        cache.get(catalog, "de").await.expect("reading");
        catalog.bump_data_version().await.expect("stamping");

        let slot = cache.slot("de");
        let held = slot.lock().await;
        let burst: Vec<_> = (0..BURST)
            .map(|_| {
                let (cache, catalog) = (Arc::clone(&cache), catalog.clone());
                tokio::spawn(async move { cache.get(&catalog, "de").await })
            })
            .collect();
        // The map's handle, this test's, and one per waiting request.
        for _ in 0..2_000 {
            if Arc::strong_count(&slot) == 2 + BURST {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert_eq!(
            Arc::strong_count(&slot),
            2 + BURST,
            "the burst never arrived"
        );
        drop(held);

        let mut read = Vec::new();
        for request in burst {
            read.push(request.await.expect("joining").expect("reading"));
        }
        assert!(read.iter().all(|language| Arc::ptr_eq(&read[0], language)));
        assert_eq!(printed(&read[0]), Some(OLD));
        assert_eq!(reads(&cache), 2, "one read before the stamp, one after");
        sandbox.close().await;
    }

    /// A code the catalog has no name for is answered in English and holds
    /// nothing of its own, so made-up codes cannot fill this process.
    #[tokio::test]
    async fn an_unknown_language_is_english_and_holds_nothing_new() {
        let sandbox = Sandbox::open("unknown").await;
        let catalog = &sandbox.catalog;
        let cache = TextCache::default();
        let made_up = cache.get(catalog, "xx-made-up").await.expect("reading");
        assert_eq!(made_up.lang(), "en");
        let english = cache.get(catalog, "en").await.expect("reading");
        assert!(Arc::ptr_eq(&made_up, &english));
        assert_eq!(cache.slots.lock().len(), 1);
        assert_eq!(reads(&cache), 1);
        sandbox.close().await;
    }
}
