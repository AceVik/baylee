//! `transcode-report`: how far the readers reach, and what would join the
//! pool.

use crate::{
    BTreeMap, BTreeSet, Path, collect_scripts, fs, refusal_cause, scriptgen, scripts_root,
    scryfall, stub_names, tokengen,
};

/// Counts how many reference scripts the transcoder reads in full.
///
/// The number is the honest ceiling on what `codegen` can generate from the
/// rules reference: a script it refuses becomes an ordinary stub, so this is
/// also the list of rules worth adding next.
/// Which cards `transcode-report` ranks: all of the corpus, this pool's
/// stubs, the names in a file, or the stubs among those names.
pub(crate) struct Narrow<'a> {
    pub(crate) stubs: bool,
    pub(crate) names: Option<&'a Path>,
}

impl Narrow<'_> {
    /// The names to keep and how many cards they stand for, or `None` for
    /// the whole corpus.
    pub(crate) fn wanted(&self, root: &Path) -> anyhow::Result<Option<(BTreeSet<String>, usize)>> {
        let listed = match self.names {
            Some(path) => {
                let text = fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
                let set: BTreeSet<String> = text
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .map(str::to_string)
                    .collect();
                anyhow::ensure!(!set.is_empty(), "{} names no card", path.display());
                Some(set)
            }
            None => None,
        };
        let stubs = if self.stubs {
            Some(stub_names(&root.join("crates/baylee-cards/src/cards"))?)
        } else {
            None
        };
        Ok(match (listed, stubs) {
            (None, None) => None,
            (Some(set), None) => {
                let n = set.len();
                Some((set, n))
            }
            (None, Some(stubs)) => Some(stubs),
            (Some(set), Some((stubs, _))) => {
                let both: BTreeSet<String> = stubs.intersection(&set).cloned().collect();
                let n = both.len();
                Some((both, n))
            }
        })
    }
}

pub(crate) fn transcode_report(
    root: &Path,
    scripts_dir: &Path,
    samples: usize,
    narrow: &Narrow<'_>,
    reason: Option<&str>,
    top: usize,
) -> anyhow::Result<()> {
    let dir = scripts_root(root, scripts_dir);
    // The token corpus, found the same way the run that writes the ledger
    // finds it. A report run without it would rank "there is no token
    // directory" as a gap in the DSL and send somebody to write a rule that
    // is already there.
    let tokens = tokengen::TokenLookup::beside(&dir)?;
    let cache = root.join("data/scryfall-cache");
    let agent = ureq::Agent::new_with_defaults();
    let cats = scryfall::fetch_subtype_catalogs(&agent, &cache)?;
    let mut files = Vec::new();
    collect_scripts(&dir, &mut files)?;
    files.sort();
    let wanted = narrow.wanted(root)?;
    let (mut read, mut refused) = (0usize, 0usize);
    let mut causes: BTreeMap<String, usize> = BTreeMap::new();
    let mut shown = 0usize;
    // Which stubs a script was actually found for. Reported rather than
    // assumed: the set above holds two spellings for a double-faced card and
    // exactly one of them can match, so a hit is a card — and a worklist
    // that silently covers 671 of 775 stubs is one whose largest entry is
    // invisible, which is what happened.
    let mut hit: BTreeSet<String> = BTreeSet::new();
    for path in &files {
        let text = fs::read_to_string(path)?;
        if let Some((wanted, _)) = &wanted {
            let name = text
                .lines()
                .find_map(|l| l.strip_prefix("Name:"))
                .unwrap_or_default()
                .trim();
            if !wanted.contains(name) {
                continue;
            }
            hit.insert(name.to_string());
        }
        let script = scriptgen::parse(&text);
        if scriptgen::transcode(&script, &cats, tokens.as_ref()).is_some()
            || scriptgen::is_vanilla(&script)
        {
            read += 1;
        } else {
            refused += 1;
            let cause = refusal_cause(&script, &cats, tokens.as_ref());
            let wanted_cause = reason.is_none_or(|want| cause.contains(want));
            *causes.entry(cause.clone()).or_insert(0usize) += 1;
            if shown < samples && wanted_cause {
                shown += 1;
                println!("--- refused ({cause}): {}\n{text}", path.display());
            }
        }
    }
    let total = read + refused;
    println!(
        "transcoder: {read} / {total} scripts read in full ({}%)",
        (read * 100).checked_div(total).unwrap_or(0)
    );
    // The token half, which is the same reader's reach over a second corpus
    // and was a number nobody could ask for. A token's abilities are read by
    // the transcoder above, so the two move together — and the collision
    // count is the one fact here that is about the *naming* rule rather than
    // about the DSL, which is why it is printed even when it is nought.
    if let Some(tokens) = &tokens {
        let reach = tokens.reach(&cats);
        println!(
            "token scripts: {} / {} read in full, {} / {} of those that print a rules line; \
             {} names, {} claimed by two definitions",
            reach.read,
            reach.total,
            reach.ability_read,
            reach.with_ability,
            reach.names,
            reach.collisions.len(),
        );
        if !reach.collisions.is_empty() {
            println!(
                "  two definitions at one name: {}",
                reach.collisions.join(", ")
            );
        }
    }
    if let Some((_, cards)) = &wanted {
        println!(
            "  over the cards asked for: {} of {cards} have a reference script",
            hit.len()
        );
    }
    let mut ranked: Vec<(&String, &usize)> = causes.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1));
    println!("what the refused scripts need next:");
    let top = if top == 0 { ranked.len() } else { top };
    for (cause, n) in ranked.iter().take(top) {
        println!("  {n:>6}  {cause}");
    }
    // What a cap leaves out is said out loud. A ranking that simply stopped
    // at thirty read as the whole list, and a cause this project was about
    // to write — `Phase.PresentDefined`, five of our own stubs — sat below
    // the line where nothing could see it.
    if let Some(rest) = ranked.get(top..).filter(|rest| !rest.is_empty()) {
        let scripts: usize = rest.iter().map(|(_, n)| **n).sum();
        println!(
            "  {} more causes not shown, over {scripts} scripts (--causes 0 for all)",
            rest.len()
        );
    }
    Ok(())
}

/// Names the cards one reader would write in full and this pool has not got.
///
/// Three filters, and each of them is the answer to a way the worklist could
/// hand `codegen` a name it cannot use:
///
/// - **The ledger is the population.** A card with no row is a card `codegen`
///   stops the whole run on, so walking the ledger rather than the corpus is
///   what keeps a batch from dying at its first name. It also fixes the
///   spelling: the ledger follows Scryfall, and `data/card-pool.txt` says
///   names must.
/// - **The pool is excluded by `oracle_id`, never by name.** Three cards in
///   this pool share a name with a token, and `CardDef::name` is the front
///   face alone where the ledger writes `Fire // Ice`.
/// - **The script is found at two tiers**, whole name then front face, for
///   the same reason the payload cache matches at two: the reference names a
///   two-faced card by one of the two spellings and not always the same one.
///
/// What it does *not* claim is that the card comes out. `scriptgen` claiming
/// every clause is one reader agreeing; `stubgen::transcode_card` still needs
/// a printing, and an honest stub is a correct outcome. The batch is measured
/// after the run, which is why this writes a worklist and no promise.
/// Which of `names` `data/card-pool.txt` does not already hold, in the order
/// they were proposed.
///
/// The pool is matched case-insensitively and with the surrounding space
/// trimmed, because it is a hand-edited file: a name that differs from the
/// ledger's spelling only in case would otherwise be appended a second time,
/// and `codegen` refuses a pool that names one card twice (#47) — a refusal
/// that lands *after* the names are already in the file, where `xtask` can no
/// longer run to take them out again. A proposed name that the file could not
/// read back — empty, or a `#` comment — is dropped for the same reason.
pub(crate) fn pool_additions(pool: &str, names: &[&str]) -> Vec<String> {
    let mut have: BTreeSet<String> = pool
        .lines()
        .map(|line| line.trim().to_ascii_lowercase())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let mut out = Vec::new();
    for name in names {
        let line = name.trim();
        // A line that is empty or starts with `#` reads back as nothing, so
        // appending one would add a card the next run cannot see. No ledger
        // name is shaped like that — this is the guard that says so, rather
        // than the assumption that it is true.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Inserted as we go, so a name proposed twice in one batch is caught
        // by the same test as one the file already holds.
        if have.insert(line.to_ascii_lowercase()) {
            out.push(line.to_string());
        }
    }
    out
}
