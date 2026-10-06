//! The Scryfall payloads a card is checked against: the cache, the pinned
//! printings, and filling both.

use crate::{BTreeMap, Path, acceptance, card_files, front_face_slug, fs, scryfall, stubgen};

/// The cached Scryfall payload for a pool card, read once for every check
/// that holds the card against its printing.
///
/// The cache is keyed by the name it was **fetched** under, and for a
/// double-faced card that is both faces joined — Agadeem's Awakening is on
/// disk as `agadeem_s_awakening_agadeem_the_undercrypt.json` — while the
/// card's *file* is named after its front face alone. Three checks built
/// the path from that file slug and so found nothing for 102 of the pool's
/// 1365: every double-faced card in it, every payload present, every
/// printing comparison quietly skipped. They were three copies of the same
/// four lines, which is why all three were wrong in the same way, so the
/// lookup is one function and the payload is now read once per card
/// instead of three times.
pub(crate) fn cached_printing(root: &Path, name: &str) -> Option<serde_json::Value> {
    payload_in(&root.join("data/scryfall-cache"), name)
}

/// [`cached_printing`] against a cache directory the caller names.
///
/// `validate` reads `data/scryfall-cache` because it takes no `--cache`;
/// `scryfall-cache` fills whatever it was pointed at, and a command that
/// filled one directory and then read another would report its own work as
/// missing.
pub(crate) fn payload_in(cache: &Path, name: &str) -> Option<serde_json::Value> {
    let path = cache.join(format!(
        "{}.json",
        baylee_cards_codegen::stubgen::slug(name)
    ));
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// The payload for a *printing* id, if one was fetched into the cache.
///
/// [`cached_printing`] answers with today's default printing for a name, which
/// moves when Scryfall ships a reprint. A card's header names one printing by
/// id and that never moves, so anything holding the header to what it claims
/// reads this first. `xtask scryfall-cache` is what fills it.
pub(crate) fn cached_printing_by_id(root: &Path, id: &str) -> Option<serde_json::Value> {
    let path = scryfall::printing_cache_path(&root.join("data/scryfall-cache"), id);
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// Which printing a card is, as far as this cache can say.
///
/// The header names **one** printing by id, and that is the card. What
/// [`cached_printing`] answers with is whatever Scryfall calls the default for
/// the *name* today, and that moves: 32 of this pool's headers became "wrong"
/// the day *Reality Fracture Commander* shipped, with nobody having touched a
/// file, and CI stayed red over it while every local gate was green (#50).
/// Refetching those 32 name payloads here left **24** still naming another
/// printing and 8 agreeing again within the same afternoon, which is the
/// argument for the pin rather than against it: whichever way a default
/// swings, an id does not.
///
/// This is the one place that measurement is written down; the two commands
/// that act on it ([`fill_pinned_printings`] and
/// [`scryfall::fetch_printing`]) point here rather than restating it.
pub(crate) enum Pinned<'a> {
    /// The header names the printing this payload already is — the ordinary
    /// case, and the only one a card with no header id can reach.
    Default(&'a serde_json::Value),
    /// The header names another printing, and the cache holds it.
    Other(serde_json::Value),
    /// The header names a printing nobody has fetched. Reading the default
    /// instead would hold the card against a piece of cardboard it was not
    /// written from, which reports the *reprint* as the card's defect.
    Missing,
}

/// Resolves a card's header to the printing it names.
///
/// One reader, because `validate` and `refresh-oracle` disagreeing about which
/// printing a card is would be worse than either being wrong on its own: the
/// check would report a header the refresh had just written.
pub(crate) fn pinned_printing<'a>(
    root: &Path,
    content: &str,
    default: &'a serde_json::Value,
) -> Pinned<'a> {
    let Some(id) = header_scryfall_id(content) else {
        // A header with no id at all is `check_header_matches_code`'s finding.
        return Pinned::Default(default);
    };
    if default.get("id").and_then(serde_json::Value::as_str) == Some(id) {
        return Pinned::Default(default);
    }
    cached_printing_by_id(root, id).map_or(Pinned::Missing, Pinned::Other)
}

/// The Scryfall id a card's `//! Set:` header names — the id that says which
/// printing the card *is*.
///
/// [`check_header_matches_code`] reads the same line with its own
/// label-parameterised scanner, because it asks the question twice (Scryfall
/// ID and Oracle ID) and about the code rather than about a payload. Every
/// reader that resolves the header to a *printing* goes through here.
pub(crate) fn header_scryfall_id(content: &str) -> Option<&str> {
    content
        .lines()
        .find(|l| l.starts_with("//!") && l.contains("Scryfall ID"))
        .and_then(|l| l.split("Scryfall ID: ").nth(1))
        .map(|v| v.split([' ', '|']).next().unwrap_or("").trim())
        .filter(|id| !id.is_empty())
}

/// Fills the Scryfall payload cache and reports what it cost.
///
/// The same two steps `codegen` takes before it generates anything — one bulk
/// download for a cold cache, then one request per card the feed did not
/// carry — with none of the generation behind them. That split is the point:
/// a scheduled job can keep the cache warm without the card-script corpus,
/// and without failing on the stale files a pool change legitimately leaves
/// behind.
/// Brings the payload cache up to the shape the readers ask of it, then says
/// so on disk.
///
/// Two conditions, one repair. A **missing** payload is the cold-cache case
/// and has always been handled: one bulk download instead of one request per
/// card. A payload that is present and **predates what a reader now reads**
/// is the other, and it is the one that hid — it is not missing, so a fill
/// that looks for missing files finds nothing to do and reports success. That
/// is #146: every session that touched a card was stopped by
/// `ScryfallCard::sides`, and the command its message named could not repair
/// it.
///
/// The stamp is written **after** the rewrite and only if the rewrite wrote
/// something, so a download that dies halfway leaves the cache stale and the
/// next run repairs it. It is not written when nothing was stale, because
/// then it is already there.
pub(crate) fn refresh_payload_cache(names: &[String], agent: &ureq::Agent, cache: &Path) {
    let stale = scryfall::schema_stale(cache);
    if stale && cache.exists() {
        eprintln!(
            "scryfall: {} holds payloads older than what this reader asks of them, rewriting",
            cache.display()
        );
    }
    let written = scryfall::fill_from_bulk(names, agent, cache, stale);
    if !stale {
        return;
    }
    if written > 0 {
        scryfall::write_schema_stamp(cache);
    } else if cache.exists() {
        // The bulk path declines under its own threshold, so a pool this
        // small cannot be rewritten wholesale — say it once rather than
        // print the line above on every run and change nothing.
        eprintln!(
            "scryfall: nothing was rewritten; delete {} and run again to refill it from scratch",
            cache.display()
        );
    }
}

pub(crate) fn scryfall_cache(root: &Path, cache: &Path, refetch: bool) -> anyhow::Result<()> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    let cache = if cache.is_absolute() {
        cache.to_path_buf()
    } else {
        root.join(cache)
    };
    let held = |names: &[String]| {
        names
            .iter()
            .filter(|n| cache.join(format!("{}.json", stubgen::slug(n))).exists())
            .count()
    };
    let before = held(&names);
    println!(
        "scryfall cache: {before}/{} payloads held in {}",
        names.len(),
        cache.display()
    );
    let agent = ureq::Agent::new_with_defaults();
    if refetch {
        // Asked for explicitly, so no stamp is consulted: the flag exists for
        // the case where a person knows the held files are wrong and nothing
        // on disk says so yet.
        let written = scryfall::fill_from_bulk(&names, &agent, &cache, true);
        if written > 0 {
            scryfall::write_schema_stamp(&cache);
        }
    } else {
        refresh_payload_cache(&names, &agent, &cache);
    }
    // Whatever the bulk feed did not carry, one at a time — the same call
    // codegen makes, so a card fetched here is byte-identical to one fetched
    // there. A card Scryfall does not know is fatal, as it is in codegen: a
    // cache quietly missing a card is a stub generated from nothing.
    for name in &names {
        scryfall::fetch_named(name, &agent, &cache)?;
    }
    let after = held(&names);
    println!(
        "scryfall cache: {after}/{} payloads held ({} fetched this run)",
        names.len(),
        after - before
    );
    fill_pinned_printings(root, &names, &agent, &cache)?;
    Ok(())
}

/// Fetches the printing each card's header actually names, where that is not
/// the one Scryfall defaults to today.
///
/// [`Pinned`] is why, and carries the measurement. What this adds is that the
/// fetch is **one request per drifted card and no bulk feed**, because the set
/// is the drift rather than the pool: 24 today against a pool of 1616. The
/// count is printed for the same reason `cross-read` carries a floor — the day
/// it stops being a handful should be visible rather than merely slow — and
/// past [`PINNED_PRINTING_HINT`] it says what to do about it.
///
/// It is also where a *wrong* id is caught: [`scryfall::fetch_printing`] fails
/// on a printing Scryfall does not know, where `validate` can only skip it.
pub(crate) fn fill_pinned_printings(
    root: &Path,
    names: &[String],
    agent: &ureq::Agent,
    cache: &Path,
) -> anyhow::Result<()> {
    let files = card_files(&root.join("crates/baylee-cards/src/cards"))?;
    let mut drifted = 0usize;
    let mut fetched = 0usize;
    for name in names {
        let (Some(path), Some(payload)) =
            (files.get(&front_face_slug(name)), payload_in(cache, name))
        else {
            continue;
        };
        let content = fs::read_to_string(path)?;
        let Some(header_id) = header_scryfall_id(&content) else {
            continue;
        };
        if payload.get("id").and_then(serde_json::Value::as_str) == Some(header_id) {
            continue;
        }
        drifted += 1;
        if scryfall::printing_cache_path(cache, header_id).exists() {
            continue;
        }
        scryfall::fetch_printing(header_id, agent, cache)?;
        fetched += 1;
    }
    println!(
        "scryfall cache: {drifted} header(s) name a printing other than today's default \
         ({fetched} fetched this run)"
    );
    if drifted > PINNED_PRINTING_HINT {
        println!(
            "scryfall cache: past {PINNED_PRINTING_HINT} of these, one request each is the \
             wrong shape \u{2014} fill them from the `default_cards` bulk feed the way \
             `fill_from_bulk` fills the name-keyed half"
        );
    }
    Ok(())
}

/// How many pinned printings may be fetched one at a time before the command
/// says the shape is wrong. Not a failure: it is a hint with a number on it,
/// because "a handful" is what makes one-request-each defensible and nothing
/// was measuring whether it still was.
pub(crate) const PINNED_PRINTING_HINT: usize = 100;

/// The oracle ids `data/unplayable.tsv` rules out: ante, dexterity and
/// subgame cards, which are never built (the file's header says why).
///
/// A row that is not `oracle_id<TAB>name<TAB>reason`, or whose reason is not
/// one of the three words, fails rather than being skipped: a misspelt row is
/// a card that would quietly count as buildable again.
pub(crate) fn unplayable_ids(root: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let text = fs::read_to_string(root.join("data/unplayable.tsv"))?;
    let mut out = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        anyhow::ensure!(
            cols.len() == 3 && matches!(cols[2], "ante" | "dexterity" | "subgame"),
            "data/unplayable.tsv:{}: expected oracle_id<TAB>name<TAB>ante|dexterity|subgame",
            n + 1
        );
        out.insert(cols[0].to_string(), cols[1].to_string());
    }
    anyhow::ensure!(!out.is_empty(), "data/unplayable.tsv lists no card");
    Ok(out)
}
