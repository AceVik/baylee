//! `ledger`: assigning `CardIndex` over the whole corpus.

use crate::{Path, fs, ledger};

/// Assigns a `CardIndex` to every corpus card that has none.
///
/// The corpus arrives already in first-appearance order — release date, then
/// name, then oracle id, the last of which no two cards share — so this walks
/// it once and hands out the next free index. That order is the *whole*
/// assignment rule, which is why it is produced by one query with one `ORDER
/// BY` rather than reconstructed here.
///
/// Appending is the normal case and reseeding is not: a run over a corpus
/// whose cards all have rows writes nothing and says so. Nothing in here can
/// move an index that exists — `IndexLedger::assign` returns the stored one —
/// so the worst a bad corpus file can do is add rows at the end.
///
/// `--reseed` is the exception and is a one-off: it reads no ledger at all and
/// numbers the corpus from zero, because that is what throwing the
/// assignments away means. Every stored `CardIndex` anywhere becomes wrong, so
/// it is a deliberate migrate-or-discard operation and never part of
/// maintenance.
///
/// Either way the run ends by asking the **registry** whether every card this
/// repo compiles got a row, and refuses to write if one did not. That is the
/// half the corpus cannot know: its filter is Scryfall's vocabulary and says
/// nothing about what somebody here has implemented, and codegen cannot build
/// a card with no index. The answer to a refusal is
/// `data/corpus-keep.tsv`, not a special case in here.
pub(crate) fn ledger_cmd(
    root: &Path,
    corpus: &Path,
    check: bool,
    reseed: bool,
) -> anyhow::Result<()> {
    let corpus_path = if corpus.is_absolute() {
        corpus.to_path_buf()
    } else {
        root.join(corpus)
    };
    let text = fs::read_to_string(&corpus_path).map_err(|e| {
        anyhow::anyhow!(
            "reading {} ({e}) — write it with `baylee-catalog corpus`",
            corpus_path.display()
        )
    })?;
    // The file this writes is the source of the binary writing it, which is
    // the same shape as the orphan guard below linking `baylee_cards::all()`.
    // It is safe because assignment only ever appends: a run reading a table
    // one build old re-derives exactly the rows that table already has, and
    // adds the rest. What it cannot do is move one.
    let mut ledger = if reseed {
        ledger::IndexLedger::default()
    } else {
        ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS)?
    };
    let before = ledger.entries().len();

    let mut seen = 0usize;
    for (n, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.splitn(4, '\t');
        let (Some(oracle_id), Some(_released), Some(set), Some(name)) =
            (cols.next(), cols.next(), cols.next(), cols.next())
        else {
            anyhow::bail!(
                "{}:{}: expected oracle_id<TAB>released_at<TAB>set<TAB>name, got {line:?}",
                corpus_path.display(),
                n + 1
            );
        };
        ledger.assign(oracle_id, name, set)?;
        seen += 1;
    }

    // The corpus filter is Scryfall's vocabulary and knows nothing about this
    // repo, so a card somebody implemented can fall outside it — eight acorn
    // lands do. Codegen cannot build a card with no index, so a run that would
    // leave one without a row writes nothing and names it. The fix is a line
    // in data/corpus-keep.tsv, which puts the card back into the corpus and
    // lets the same query decide its set and its place in the order.
    let orphans: Vec<&'static str> = baylee_cards::all()
        .filter(|def| ledger.index_of(def.oracle_id).is_none())
        .map(baylee_cards::dsl::CardDef::name)
        .collect();
    if !orphans.is_empty() {
        for name in &orphans {
            println!("  no row: {name}");
        }
        anyhow::bail!(
            "{} card(s) this repo compiles have no index.\n\
             Add each one's oracle_id to data/corpus-keep.tsv, rerun \
             `baylee-catalog corpus`, and try again.",
            orphans.len()
        );
    }

    let assigned = ledger.entries().len() - before;
    println!(
        "corpus: {seen} cards, ledger: {} rows ({assigned} newly assigned)",
        ledger.entries().len()
    );
    // Rendered and compared rather than appended to, so a run with nothing to
    // assign still repairs a table somebody edited by hand.
    let rows_path = root.join("crates/baylee-cards-index/src/generated.rs");
    let content = ledger.render_rows();
    if fs::read_to_string(&rows_path).unwrap_or_default() == content {
        println!("nothing to assign; {} is unchanged", rows_path.display());
        return Ok(());
    }
    if check {
        println!("--check: would write {}", rows_path.display());
        return Ok(());
    }
    fs::write(&rows_path, &content)
        .map_err(|e| anyhow::anyhow!("writing {} ({e})", rows_path.display()))?;
    println!("wrote {}", rows_path.display());
    // The assigner reads the table it has just written, so the binary in
    // `target/` is one assignment behind until the next build. `cargo run`
    // rebuilds on its own; a cached binary invoked directly does not.
    println!("rebuild before `cargo xtask codegen` — the table it reads has changed");
    Ok(())
}
