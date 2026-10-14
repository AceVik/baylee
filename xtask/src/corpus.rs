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
/// repository ([`pick_cardsfolder`]: an upstream checkout first). Failing
/// all three it hands back the path that was asked for, so a caller that
/// finds nothing reports what a person named rather than something this
/// function invented. The choice is said on stderr.
pub(crate) fn scripts_root(root: &Path, given: &Path) -> PathBuf {
    let asked = root.join(given);
    let (chosen, how) = if asked.exists() {
        (asked, "--scripts")
    } else if let Some(named) = std::env::var_os(SCRIPTS_ENV)
        .map(|named| root.join(named))
        .filter(|named| named.exists())
    {
        (named, SCRIPTS_ENV)
    } else {
        match pick_cardsfolder(&root.join(".."), SEARCH_DEPTH) {
            Some((found, true)) => (found, "found, an upstream Card-Forge/forge checkout"),
            Some((found, false)) => (found, "found, no upstream checkout among the hits"),
            None => (asked, "nothing found; the path asked for"),
        }
    };
    eprintln!("card scripts: {} ({how})", chosen.display());
    chosen
}

/// The `cardsfolder` the search settles on, and whether it is upstream's.
///
/// A fork of the reference may sit beside the upstream checkout, and its
/// scripts are not the reference's: a hit inside a checkout whose `origin`
/// is github.com/Card-Forge/forge wins, and only without one does the first
/// hit in walk order stand.
pub(crate) fn pick_cardsfolder(at: &Path, depth: usize) -> Option<(PathBuf, bool)> {
    let mut hits = Vec::new();
    every_cardsfolder(at, depth, &mut hits);
    if let Some(upstream) = hits.iter().find(|hit| is_upstream_checkout(hit)) {
        return Some((upstream.clone(), true));
    }
    hits.into_iter().next().map(|first| (first, false))
}

/// Every directory named `cardsfolder` within `depth` levels of `at`, in
/// walk order; never descending into one, nor into a hidden directory,
/// `target` or `node_modules`.
///
/// Bounded on purpose: the corpus is a sibling checkout a few levels away
/// (`forge-gui/res/cardsfolder` inside it, so five levels reach a checkout
/// one folder down), and an unbounded walk from the parent of a repository
/// is a walk of somebody's whole home directory.
fn every_cardsfolder(at: &Path, depth: usize, hits: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(at) else {
        return;
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "cardsfolder" {
            hits.push(path);
        } else if !(name.starts_with('.') || name == "target" || name == "node_modules") {
            dirs.push(path);
        }
    }
    for dir in dirs {
        every_cardsfolder(&dir, depth - 1, hits);
    }
}

/// Whether the git checkout holding `path` has an `origin` naming
/// github.com/Card-Forge/forge, over SSH or HTTPS. Read from the checkout's
/// own `config` file (a worktree's `.git` file followed to its common
/// directory); no shell and no `git` process.
fn is_upstream_checkout(path: &Path) -> bool {
    path.ancestors()
        .find_map(git_dir)
        .and_then(|dir| fs::read_to_string(dir.join("config")).ok())
        .and_then(|config| origin_url(&config))
        .is_some_and(|url| is_upstream_url(&url))
}

/// The git directory of a checkout rooted at `top`, if it is one.
fn git_dir(top: &Path) -> Option<PathBuf> {
    let dot = top.join(".git");
    if dot.is_dir() {
        return Some(dot);
    }
    let pointer = fs::read_to_string(&dot).ok()?;
    let gitdir = top.join(pointer.strip_prefix("gitdir:")?.trim());
    match fs::read_to_string(gitdir.join("commondir")) {
        Ok(common) => Some(gitdir.join(common.trim())),
        Err(_) => Some(gitdir),
    }
}

/// `remote.origin.url` out of a git `config` file.
fn origin_url(config: &str) -> Option<String> {
    let mut in_origin = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_origin = line
                .replace(' ', "")
                .eq_ignore_ascii_case("[remote\"origin\"]");
        } else if in_origin
            && let Some((key, value)) = line.split_once('=')
            && key.trim().eq_ignore_ascii_case("url")
        {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// `https://github.com/Card-Forge/forge(.git)`, `git@github.com:Card-Forge/forge.git`
/// and `ssh://git@github.com/Card-Forge/forge`, in any case.
fn is_upstream_url(url: &str) -> bool {
    let url = url.trim().to_ascii_lowercase();
    let url = url.trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    ["github.com/card-forge/forge", "github.com:card-forge/forge"]
        .iter()
        .any(|tail| {
            url.strip_suffix(tail)
                .is_some_and(|head| head.is_empty() || head.ends_with(['/', '@']))
        })
}

/// How many levels below the repository's parent the search looks.
pub(crate) const SEARCH_DEPTH: usize = 5;

/// The environment variable that names the corpus, for a checkout sitting
/// somewhere this cannot guess.
pub(crate) const SCRIPTS_ENV: &str = "BAYLEE_CARD_SCRIPTS";

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

#[cfg(test)]
mod tests {
    use super::*;

    fn checkout(at: &Path, url: &str) -> PathBuf {
        let git = at.join(".git");
        fs::create_dir_all(&git).unwrap();
        fs::write(
            git.join("config"),
            format!("[core]\n\tbare = false\n[remote \"origin\"]\n\turl = {url}\n"),
        )
        .unwrap();
        let folder = at.join("forge-gui/res/cardsfolder");
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// A fork found first loses to an upstream checkout found later; with
    /// no upstream the first hit stands.
    #[test]
    fn an_upstream_checkout_wins_over_a_fork() {
        let dir = tempfile::tempdir().unwrap();
        // `a` sorts and (on most file systems) walks first, and sits higher.
        let fork = checkout(
            &dir.path().join("a-forge"),
            "git@github.com:someone/forge.git",
        );
        let upstream = checkout(
            &dir.path().join("z/forge-reference"),
            "https://github.com/Card-Forge/forge.git",
        );
        assert_eq!(pick_cardsfolder(dir.path(), 5), Some((upstream, true)));
        fs::remove_dir_all(dir.path().join("z")).unwrap();
        assert_eq!(pick_cardsfolder(dir.path(), 5), Some((fork, false)));
    }

    #[test]
    fn upstream_urls_are_recognised_over_ssh_and_https() {
        for url in [
            "https://github.com/Card-Forge/forge",
            "https://github.com/card-forge/forge.git/",
            "git@github.com:Card-Forge/forge.git",
            "ssh://git@github.com/Card-Forge/forge.git",
        ] {
            assert!(is_upstream_url(url), "{url}");
        }
        for url in [
            "git@github.com:someone/forge.git",
            "https://github.com/Card-Forge/forge-fork",
            "https://example.com/github.com/Card-Forge/forgex",
        ] {
            assert!(!is_upstream_url(url), "{url}");
        }
        let config = "[remote \"upstream\"]\n\turl = a\n[remote \"origin\"]\n\turl = b\n";
        assert_eq!(origin_url(config).as_deref(), Some("b"));
    }

    /// A worktree's `.git` is a file; its config is in the common directory.
    #[test]
    fn a_worktree_checkout_reads_its_common_config() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("main");
        checkout(&main, "https://github.com/Card-Forge/forge");
        let wt_git = main.join(".git/worktrees/wt");
        fs::create_dir_all(&wt_git).unwrap();
        fs::write(wt_git.join("commondir"), "../..\n").unwrap();
        let wt = dir.path().join("wt");
        fs::create_dir_all(wt.join("res/cardsfolder")).unwrap();
        fs::write(wt.join(".git"), format!("gitdir: {}\n", wt_git.display())).unwrap();
        assert!(is_upstream_checkout(&wt.join("res/cardsfolder")));
    }
}
