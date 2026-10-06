//! Where the card-script reference is, and the tokens the pool reaches for
//! in it.

use crate::{
    BTreeSet, Path, PathBuf, Pool, fs, render_token_ledger, scriptgen, tokengen, write_or_check,
};

/// The card-script reference, as a directory on this machine.
///
/// The corpus is an external, read-only lookup that is never vendored (see
/// `NOTICE`), so there is no path that is right for everyone and a
/// hard-coded one is right for exactly the machine it was typed on. Three
/// answers in order: `--scripts` if what it names exists, then
/// `BAYLEE_CARD_SCRIPTS`, then a `cardsfolder` directory found beside the
/// repository. Failing all three it hands back the path that was asked for,
/// so a caller that finds nothing reports what a person named rather than
/// something this function invented.
pub(crate) fn scripts_root(root: &Path, given: &Path) -> PathBuf {
    let asked = root.join(given);
    if asked.exists() {
        return asked;
    }
    if let Some(named) = std::env::var_os(SCRIPTS_ENV) {
        let named = root.join(named);
        if named.exists() {
            return named;
        }
    }
    find_cardsfolder(&root.join(".."), 4).unwrap_or(asked)
}

/// The environment variable that names the corpus, for a checkout sitting
/// somewhere this cannot guess.
pub(crate) const SCRIPTS_ENV: &str = "BAYLEE_CARD_SCRIPTS";

/// The first directory named `cardsfolder` within `depth` levels of `at`.
///
/// Bounded and breadth-first on purpose: the corpus is a sibling checkout a
/// level or two away, and an unbounded walk from the parent of a repository
/// is a walk of somebody's whole home directory.
pub(crate) fn find_cardsfolder(at: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(at).ok()?.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == "cardsfolder") {
            return Some(path);
        }
        dirs.push(path);
    }
    dirs.into_iter()
        .find_map(|dir| find_cardsfolder(&dir, depth - 1))
}

/// One row per token, over bodies several stems may have produced.
///
/// The key is the **whole body** and not its constant, and that distinction
/// is the whole of the collision guard. A name-keyed set was right exactly
/// while the constant described the token in full; the day a token could
/// carry an ability — which the name leaves out — it began dropping the
/// second of two *different* definitions before anything compared them. So
/// `tokenledger::assign`'s refusal could never fire from a `codegen` run,
/// and a card meaning the plain Skeleton would have been handed the
/// regenerating one, chosen by the order the stems happen to iterate in.
///
/// Keyed on the body, a genuine clash arrives at the ledger, which stops the
/// run and names it.
pub(crate) fn one_row_per_token(
    bodies: impl Iterator<Item = tokengen::TokenBody>,
) -> Vec<tokengen::TokenBody> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for body in bodies {
        if seen.insert((body.constant.clone(), body.literal.clone())) {
            out.push(body);
        }
    }
    out
}

/// Every token a card in this pool reaches for, read in full.
///
/// The ledger is "every token there is" and may only be appended to, so what
/// goes into it is a decision about which rows exist forever. Filling it from
/// the whole corpus would file the reference's Dungeons and its
/// planeswalker-emblem tokens beside the Soldiers, with nothing that could
/// ever create them; filling it from the pool's own scripts grows it exactly
/// as far as the transcoder can reach, which is the same bargain the card
/// ledger makes.
///
/// A stem the reader refuses is simply absent — see [`tokengen::TokenLookup::body`]
/// for why that is one answer rather than two.
///
/// Two stems that read as the same token are one row; see
/// [`one_row_per_token`] for what "the same" is.
pub(crate) fn tokens_the_pool_reaches_for(
    pool: &Pool,
    scripts: &scriptgen::ScriptLookup,
    tokens: &tokengen::TokenLookup,
) -> Vec<tokengen::TokenBody> {
    let mut stems = BTreeSet::new();
    for name in pool.names {
        if let Some(script) = scripts.script(name) {
            stems.extend(scriptgen::token_stems(&script));
        }
    }
    let bodies = stems.iter().filter_map(|stem| tokens.body(stem, pool.cats));
    let out = one_row_per_token(bodies);
    println!(
        "token scripts: {} in the reference, {} named by this pool, {} read in full",
        tokens.len(),
        stems.len(),
        out.len()
    );
    out
}

/// Which id every token there is was assigned → `generated_tokens.rs`.
///
/// Written **before** the cards, because a card that creates a token names the
/// constant the ledger filed it under, so the id has to exist before the card
/// that spends it is written. It is also the one generated file that reads
/// *itself* back: the ids it has already given out are the ids it must give
/// out again, and the only thing a run may do to the table is append.
pub(crate) fn token_ledger(
    root: &Path,
    check: bool,
    pool: &Pool,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let bodies = if let (Some(scripts), Some(tokens)) = (pool.scripts, pool.tokens) {
        tokens_the_pool_reaches_for(pool, scripts, tokens)
    } else {
        println!("note: token scripts not found beside the card scripts, skipping");
        Vec::new()
    };
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated_tokens.rs"),
        &render_token_ledger(root, &bodies)?,
        changed,
    )
}
