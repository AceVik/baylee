//! Wizards' preconstructed decks, offered beside the house decks.
//!
//! `cargo run -p xtask -- decks-status` decides which of the precon lists in
//! `data/decks/precon/` this build can play and embeds exactly those
//! ([`PLAYABLE`], generated). [`sync`] makes the database agree with that
//! list as the gateway starts, so a precon arrives, changes and leaves with
//! the build that plays it and never needs a migration of its own
//! (`docs/precons.md` §"House decks").
//!
//! What a sync does, all in one transaction under an advisory lock, so two
//! gateways starting against one database take turns:
//!
//! - It compares a stamp of the lists with the one it last wrote
//!   (`deck_sync`). The same lists are one read and no write.
//! - A list it has not seen becomes a deck of kind `preconstructed` that
//!   belongs to nobody, found again by its `source` key.
//! - A list whose cards changed leaves the state it replaces in
//!   `deck_version` and moves the deck one version on, as a player's save
//!   does ([`deck::Model::same_cards`] decides for both). A copy a player
//!   took names the version it came from, and that version stays readable.
//! - A deck it wrote before that is no longer in the lists is **withdrawn**:
//!   `offered` goes false and the shared listing stops showing it. It is
//!   never deleted, so every copy still names what it came from; when it
//!   is playable again it comes back as the same deck, history and all.
//! - A deck with no `source` — a player's, or a house deck a migration
//!   seeded — is never read or written.

use crate::entity::deck;
use crate::entity::deck_version;
use crate::entity::prelude::{Deck, DeckVersion};
use anyhow::{Context as _, Result, anyhow, bail};
use baylee_deckio::format::{self, FormatId};
use sea_orm::{
    ActiveModelTrait as _,
    ActiveValue::{NotSet, Set},
    ColumnTrait as _, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait as _,
    QueryFilter as _, QueryOrder as _, Statement, TransactionTrait as _,
};
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use time::OffsetDateTime;

mod generated;
pub use generated::PLAYABLE;

/// One precon list, as the build embeds it.
#[derive(Clone, Copy, Debug)]
pub struct Precon {
    /// Where the list lives under `data/decks/precon/`, without `.txt`
    /// (`E02/sun-empire`). The deck's `source`, and so its identity.
    pub key: &'static str,
    /// The list, in Baylee's text format, header and all.
    pub text: &'static str,
}

/// Raised when what a sync writes *from the same lists* changes — the
/// description it composes, say — so that every database is written once
/// more although the lists' own stamp would not have moved.
pub const SYNC_VERSION: u32 = 1;

/// This sync's row in `deck_sync`.
const SYNC_NAME: &str = "precons";

/// The advisory lock two gateways take turns on. Any constant would do; this
/// one spells what it is for.
const LOCK: i64 = i64::from_be_bytes(*b"baylee:p");

/// What one sync did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Synced {
    /// The lists the build offers.
    pub offered: usize,
    /// Whether the stamp matched, so nothing was read past it or written.
    pub unchanged: bool,
    /// Decks written for the first time.
    pub added: usize,
    /// Decks whose cards, name, format or description moved.
    pub changed: usize,
    /// Withdrawn decks offered again.
    pub returned: usize,
    /// Decks withdrawn because the build no longer offers them.
    pub withdrawn: usize,
}

/// One list, read into the columns a deck stores.
struct List {
    key: &'static str,
    name: String,
    format: String,
    description: String,
    cards: Vec<String>,
    sideboard: Vec<String>,
    commanders: Vec<String>,
    /// What a new version of the deck is called in its history: the build
    /// of the source the list was taken from.
    summary: String,
}

impl List {
    /// Read one list with the reader a player's import uses, so a synced
    /// deck holds exactly the rows the same file imported by hand would.
    fn read(precon: &Precon) -> Result<Self> {
        let read = format::read(FormatId::Baylee, precon.text)
            .map_err(|e| anyhow!("precon {}: {e}", precon.key))?;
        if let Some(skipped) = read.skipped.first() {
            bail!(
                "precon {}: line {} is not a row: {}",
                precon.key,
                skipped.line,
                skipped.text
            );
        }
        let format = read
            .document
            .format
            .clone()
            .unwrap_or_else(|| "freeform".to_owned());
        let stored = read.document.stored();
        if !stored.maybe.is_empty() {
            bail!("precon {}: a list has no maybeboard", precon.key);
        }
        let header = |field: &str| {
            let prefix = format!("# {field}: ");
            precon
                .text
                .lines()
                .find_map(|line| line.strip_prefix(prefix.as_str()))
                .map(str::trim)
        };
        // What the product was, in the words its type and set are printed
        // in: "Theme Deck · E02 · 2017-11-24".
        let description = ["type", "set", "released"]
            .into_iter()
            .filter_map(header)
            .collect::<Vec<_>>()
            .join(" · ");
        let summary = header("source").map_or_else(
            || "neue Liste".to_owned(),
            |source| source.split(',').next().unwrap_or(source).to_owned(),
        );
        Ok(Self {
            key: precon.key,
            name: stored.name.unwrap_or_else(|| precon.key.to_owned()),
            format,
            description,
            cards: stored.cards.iter().map(ToString::to_string).collect(),
            sideboard: stored.sideboard.iter().map(ToString::to_string).collect(),
            commanders: stored.commanders,
            summary,
        })
    }

    /// Whether a deck already says everything this list does.
    fn matches(&self, row: &deck::Model) -> bool {
        row.offered
            && row.name == self.name
            && row.format == self.format
            && row.description.as_deref() == Some(self.description.as_str())
            && row.same_cards(&self.cards, &self.sideboard, &self.commanders)
    }
}

/// The stamp of a set of lists: the sync's version, then every key and text
/// with its length before it, so no two sets of lists run together into one.
fn stamp(precons: &[Precon]) -> String {
    let mut sha = Sha256::new();
    sha.update(SYNC_VERSION.to_le_bytes());
    for precon in precons {
        for part in [precon.key, precon.text] {
            sha.update((part.len() as u64).to_le_bytes());
            sha.update(part.as_bytes());
        }
    }
    sha.finalize()
        .iter()
        .fold(String::with_capacity(64), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// Make the database offer exactly these precons.
///
/// Every list is read before anything is written, so a list that does not
/// read changes nothing. See the module for what is written.
///
/// # Errors
///
/// A list that does not read, a key listed twice, or a database that
/// refuses.
pub async fn sync(db: &DatabaseConnection, precons: &[Precon]) -> Result<Synced> {
    let lists = precons.iter().map(List::read).collect::<Result<Vec<_>>>()?;
    let mut keys = BTreeSet::new();
    if let Some(twice) = lists.iter().find(|list| !keys.insert(list.key)) {
        bail!("precon {} is listed twice", twice.key);
    }
    let stamp = stamp(precons);
    let mut done = Synced {
        offered: lists.len(),
        ..Synced::default()
    };

    let tx = db.begin().await?;
    let backend = tx.get_database_backend();
    tx.execute_raw(Statement::from_sql_and_values(
        backend,
        "SELECT pg_advisory_xact_lock($1)",
        [LOCK.into()],
    ))
    .await
    .context("taking the precon sync's lock")?;
    let last: Option<String> = tx
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            "SELECT stamp FROM deck_sync WHERE name = $1",
            [SYNC_NAME.into()],
        ))
        .await?
        .map(|row| row.try_get("", "stamp"))
        .transpose()?;
    if last.as_deref() == Some(stamp.as_str()) {
        tx.commit().await?;
        done.unchanged = true;
        return Ok(done);
    }

    let now = OffsetDateTime::now_utc();
    let mut synced: BTreeMap<String, deck::Model> = Deck::find()
        .filter(deck::Column::Source.is_not_null())
        .all(&tx)
        .await?
        .into_iter()
        .filter_map(|row| Some((row.source.clone()?, row)))
        .collect();

    for list in lists {
        match synced.remove(list.key) {
            None => {
                add(&tx, list, now).await?;
                done.added += 1;
            }
            Some(row) if list.matches(&row) => {}
            Some(row) => {
                if row.offered {
                    done.changed += 1;
                } else {
                    done.returned += 1;
                }
                rewrite(&tx, row, list, now).await?;
            }
        }
    }
    // What is left was written by an earlier sync and is not offered now.
    for row in synced.into_values().filter(|row| row.offered) {
        let mut withdrawn: deck::ActiveModel = row.into();
        withdrawn.offered = Set(false);
        withdrawn.updated_at = Set(now);
        withdrawn.update(&tx).await?;
        done.withdrawn += 1;
    }

    tx.execute_raw(Statement::from_sql_and_values(
        backend,
        "INSERT INTO deck_sync (name, stamp, synced_at) VALUES ($1, $2, now()) \
         ON CONFLICT (name) DO UPDATE SET stamp = excluded.stamp, synced_at = excluded.synced_at",
        [SYNC_NAME.into(), stamp.into()],
    ))
    .await?;
    tx.commit().await?;
    Ok(done)
}

/// A list's first deck.
async fn add(db: &impl ConnectionTrait, list: List, now: OffsetDateTime) -> Result<(), DbErr> {
    Deck::insert(deck::ActiveModel {
        id: NotSet,
        account_id: Set(None),
        kind: Set(deck::KIND_PRECONSTRUCTED.to_owned()),
        name: Set(list.name),
        format: Set(list.format),
        description: Set(Some(list.description)),
        copied_from: Set(None),
        copied_version: Set(None),
        version: Set(1),
        cards: Set(list.cards),
        sideboard: Set(list.sideboard),
        commanders: Set(list.commanders),
        sleeve: Set(None),
        playmat: Set(None),
        updated_at: Set(now),
        source: Set(Some(list.key.to_owned())),
        offered: Set(true),
    })
    .exec(db)
    .await?;
    Ok(())
}

/// A deck made to say what its list says now, offered, with the state it
/// held left in its history when the cards moved.
async fn rewrite(
    db: &impl ConnectionTrait,
    row: deck::Model,
    list: List,
    now: OffsetDateTime,
) -> Result<(), DbErr> {
    let moved = !row.same_cards(&list.cards, &list.sideboard, &list.commanders);
    let version = if moved {
        DeckVersion::insert(deck_version::ActiveModel {
            deck_id: Set(row.id),
            version: Set(row.version),
            cards: Set(row.cards.clone()),
            sideboard: Set(row.sideboard.clone()),
            commanders: Set(row.commanders.clone()),
            summary: Set(Some(list.summary)),
            superseded_at: Set(now),
        })
        .exec(db)
        .await?;
        row.version + 1
    } else {
        row.version
    };
    let mut deck: deck::ActiveModel = row.into();
    deck.name = Set(list.name);
    deck.format = Set(list.format);
    deck.description = Set(Some(list.description));
    deck.cards = Set(list.cards);
    deck.sideboard = Set(list.sideboard);
    deck.commanders = Set(list.commanders);
    deck.version = Set(version);
    deck.offered = Set(true);
    deck.updated_at = Set(now);
    deck.update(db).await?;
    Ok(())
}

/// The decks that belong to nobody and are offered: the house decks and
/// every precon the build plays, never a withdrawn one.
///
/// Here rather than in the gateway so that the sync's tests read the
/// listing players read. The house's own decks come first (`house` sorts
/// before `preconstructed`), then the newest.
///
/// # Errors
///
/// If the database refuses.
pub async fn shared_decks(db: &impl ConnectionTrait) -> Result<Vec<deck::Model>, DbErr> {
    Deck::find()
        .filter(deck::Column::Kind.ne(deck::KIND_ACCOUNT))
        .filter(deck::Column::Offered.eq(true))
        .order_by_asc(deck::Column::Kind)
        .order_by_desc(deck::Column::UpdatedAt)
        .order_by_asc(deck::Column::Name)
        .order_by_asc(deck::Column::Id)
        .all(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const ESTRID: &str = "# baylee deck export v1\n\
        # name: Adaptive Enchantment\n\
        # format: commander\n\
        # source: MTGJSON 5.3.0+20260929, https://mtgjson.com, AdaptiveEnchantment_C18.json\n\
        # licence: MIT (Copyright © 2018 – Present, Zach Halpern); see NOTICE\n\
        # type: Commander Deck\n\
        # set: C18\n\
        # released: 2018-08-10\n\
        # written by `cargo run -p xtask -- decks-import`; do not edit by hand\n\
        CMD: 1 Estrid, the Masked (C18) 40 *F*\n\
        1 Sol Ring (C18) 222\n\
        98 Forest\n\
        SB: 1 Lightning Bolt\n";

    #[test]
    fn a_list_reads_into_the_columns_a_players_import_would_fill() {
        let list = List::read(&Precon {
            key: "C18/adaptive-enchantment",
            text: ESTRID,
        })
        .expect("the list reads");
        assert_eq!(list.name, "Adaptive Enchantment");
        assert_eq!(list.format, "commander");
        assert_eq!(list.description, "Commander Deck · C18 · 2018-08-10");
        assert_eq!(list.summary, "MTGJSON 5.3.0+20260929");
        // The commander is a row among the cards, printing and all, and
        // named again bare — the shape `Document::stored` gives an import.
        assert_eq!(
            list.cards,
            [
                "1 Estrid, the Masked (C18) 40 *F*",
                "1 Sol Ring (C18) 222",
                "98 Forest"
            ]
        );
        assert_eq!(list.commanders, ["Estrid, the Masked"]);
        assert_eq!(list.sideboard, ["1 Lightning Bolt"]);
    }

    #[test]
    fn a_line_that_is_no_row_refuses_the_list() {
        let broken = format!("{ESTRID}this is not a row\n");
        let text: &'static str = Box::leak(broken.into_boxed_str());
        assert!(
            List::read(&Precon {
                key: "X/broken",
                text
            })
            .is_err()
        );
    }

    #[test]
    fn every_list_the_build_offers_reads() {
        for precon in PLAYABLE {
            List::read(precon).unwrap_or_else(|e| panic!("{e:#}"));
        }
    }

    #[test]
    fn the_stamp_moves_with_a_key_a_text_or_the_order() {
        let a = Precon {
            key: "A/a",
            text: "1 Forest\n",
        };
        let b = Precon {
            key: "B/b",
            text: "1 Island\n",
        };
        let base = stamp(&[a, b]);
        assert_eq!(base, stamp(&[a, b]), "the same lists, the same stamp");
        assert_ne!(base, stamp(&[b, a]));
        assert_ne!(base, stamp(&[a]));
        assert_ne!(
            base,
            stamp(&[
                a,
                Precon {
                    key: "B/b",
                    text: "2 Island\n"
                }
            ])
        );
        // A key and a text cannot trade bytes and keep the stamp.
        assert_ne!(
            stamp(&[Precon {
                key: "ab",
                text: "c"
            }]),
            stamp(&[Precon {
                key: "a",
                text: "bc"
            }])
        );
    }
}
