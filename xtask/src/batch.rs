//! Card batches: picking the next cards by reach, and running a batch on a
//! clean tree.

use crate::{
    BTreeMap, BTreeSet, Path, PathBuf, acceptance, card_files, catalog, front_face_slug, fs,
    pool_additions, scriptgen, scripts, scripts_root, scryfall, stubgen, tokengen, unplayable_ids,
};
use anyhow::Context as _;

/// Refuses to start a batch on a tree that already has changes in it.
///
/// A batch rewrites hundreds of card files and writes new ones, and the only
/// cheap way back from one that goes wrong is `git restore` plus `git clean`.
/// Those are safe things to reach for exactly when nothing else in the tree is
/// uncommitted, so the guard is what makes the recovery instruction true
/// rather than hopeful.
///
/// Both halves, because the first run of this command proved one is not
/// enough: `git restore .` put the ten rewritten files back and left the new
/// card behind as an untracked file, and an orphan under `crates/cards/sets` that no
/// pool line claims is what `refuse_orphans` stops the *next* run on.
pub(crate) fn refuse_a_dirty_tree(root: &Path) -> anyhow::Result<()> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()?;
    anyhow::ensure!(
        out.status.success(),
        "git status failed; run the batch from a checkout"
    );
    let dirty = String::from_utf8_lossy(&out.stdout);
    let dirty = dirty.trim();
    anyhow::ensure!(
        dirty.is_empty(),
        "the tree has uncommitted changes, and a batch's way back is `git restore` \
         plus `git clean`:\n{dirty}"
    );
    Ok(())
}

/// Runs one `cargo run -p xtask -- …` step and bails on anything but success.
///
/// **Spawned rather than called.** Stages 5 and 6 of `codegen` are built from
/// the pool *compiled into the running binary*, so a card this run adds gets
/// its ability-line and name-table rows only from the next one — after a
/// rebuild. Calling `codegen()` twice in this process would therefore do the
/// first phase twice and the second never, while looking exactly like the
/// procedure it replaces. Handing each step to `cargo` is what puts the
/// rebuild between them.
pub(crate) fn batch_step(root: &Path, what: &str, args: &[&str]) -> anyhow::Result<()> {
    println!("\nbatch: {what}");
    let status = std::process::Command::new("cargo")
        .args(["run", "-q", "-p", "xtask", "--"])
        .args(args)
        .current_dir(root)
        .status()?;
    anyhow::ensure!(
        status.success(),
        "batch stopped at `{what}` — the tree was clean before this, so this puts \
         it back:\n    git restore . && git clean -f crates/cards/sets"
    );
    // Named, not inferred. A step that says nothing on success is a step
    // nobody can tell apart from one that did not run, and the whole reason
    // `codegen --check` is in here is that the two-phase gap is invisible
    // until something asserts it closed.
    println!("batch: pass — {what}");
    Ok(())
}

/// Card files this run changed that it was not asked to change.
///
/// Codegen rewrites every machine-owned card on every run, so a rewrite is
/// not news — an identical rewrite leaves no diff at all. A card that is
/// *modified* and is none of the ones being added is therefore a reader or a
/// printing that moved underneath this batch, and it has no business riding
/// into a commit about new cards. The commonest cause by far is Scryfall
/// having changed a printing since the header was written, which is
/// `xtask refresh-oracle`'s job and nobody else's.
///
/// Untracked (`??`) card files are the batch's own output and are expected.
pub(crate) fn cards_changed_unasked(root: &Path, added: &[String]) -> anyhow::Result<Vec<String>> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain", "--", "crates/cards/sets"])
        .current_dir(root)
        .output()?;
    anyhow::ensure!(out.status.success(), "git status failed during the batch");
    let asked: BTreeSet<String> = added.iter().map(|name| stubgen::slug(name)).collect();
    Ok(drift_from_status(
        &String::from_utf8_lossy(&out.stdout),
        &asked,
    ))
}

/// A file codegen writes for the registry rather than for one card: a
/// group's `src/lib.rs` or `Cargo.toml` under `crates/cards/sets`, or the
/// pool's tables and manifest.
fn is_registry_file(path: &str) -> bool {
    matches!(
        path,
        "crates/cards/pool/src/generated.rs" | "crates/cards/pool/Cargo.toml"
    ) || path
        .strip_prefix("crates/cards/sets/")
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(_, file)| file == "src/lib.rs" || file == "Cargo.toml")
}

/// The reading half of [`cards_changed_unasked`], with the `git` call taken
/// out so it can be held against a porcelain listing in a test.
pub(crate) fn drift_from_status(porcelain: &str, asked: &BTreeSet<String>) -> Vec<String> {
    let mut drift = Vec::new();
    for line in porcelain.lines() {
        if line.len() < 3 {
            continue;
        }
        let (status, path) = line.split_at(2);
        let path = path.trim();
        // `??` is this batch's own output. A rename is reported as two paths
        // and the destination is what a later run rewrites, so read that one.
        if status.contains('?') {
            continue;
        }
        let path = path.rsplit(" -> ").next().unwrap_or(path);
        // Codegen adds a module declaration to a group's `lib.rs` and a row
        // to the pool's tables for every new card, and a new group to both
        // manifests. Those exact files are not cards; deletion, renames and
        // any other path must still be reported rather than broadly ignored.
        if status.trim() == "M" && is_registry_file(path) {
            continue;
        }
        let slug = Path::new(path)
            .file_stem()
            .map_or_default(|s| s.to_string_lossy().to_string());
        if !asked.contains(&slug) {
            drift.push(path.to_string());
        }
    }
    drift
}

/// Takes the next `count` cards the reader can write, and runs the batch
/// procedure over them.
///
/// The five steps written out three times in prose (#71) are four here, and
/// the missing one is not an omission: `codegen` fills the payload cache from
/// the bulk feed itself, at the top of its own run and from the pool it is
/// about to read, so a `scryfall-cache` before it downloads the same feed
/// twice. It stays a command of its own for a cold machine that wants the
/// The names this batch takes, measured now rather than sliced off a list.
///
/// The reach moves when the *reader* moves, and downwards is the healthy
/// direction: a refusal learned since the last batch takes cards off the list
/// that a stale slice would still be proposing.
/// Filters `proposed` against `pool`, `decks_text`, and existing or
/// intra-batch `front_face_slug` values so twin spellings never trip
/// `refuse_twin_names`.
pub(crate) fn filter_batch_additions(
    pool: &str,
    decks_text: &str,
    proposed: &[&str],
) -> anyhow::Result<Vec<String>> {
    let rows = acceptance::parse_decks(decks_text)?;
    let mut seen_slugs: BTreeSet<String> = acceptance::all_names(&rows, pool)
        .into_iter()
        .map(|name| front_face_slug(&name))
        .collect();
    Ok(pool_additions(pool, proposed)
        .into_iter()
        .filter(|name| seen_slugs.insert(front_face_slug(name)))
        .collect())
}

pub(crate) fn batch_names(
    root: &Path,
    scripts_dir: &Path,
    cache: &Path,
    count: usize,
    pool: &str,
) -> anyhow::Result<Vec<String>> {
    let reach = reach_measure(root, scripts_dir, cache)?;
    println!(
        "batch: the reader writes {} of the {} unattempted rows that carry a script",
        reach.names.len(),
        reach.scripted
    );
    anyhow::ensure!(
        !reach.names.is_empty(),
        "the reach list is empty: every card this reader can write is already in the pool"
    );

    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let proposed: Vec<&str> = reach.names.iter().copied().take(count).collect();
    let additions = filter_batch_additions(pool, &decks_text, &proposed)?;
    println!(
        "batch: taking {} name(s) in ledger order{}",
        additions.len(),
        if additions.len() == proposed.len() {
            String::new()
        } else {
            format!(
                " ({} of the {} proposed are already in the pool or acceptance decks)",
                proposed.len() - additions.len(),
                proposed.len()
            )
        }
    );
    for name in additions.iter().take(10) {
        println!("  {name}");
    }
    if additions.len() > 10 {
        println!("  … and {} more", additions.len() - 10);
    }
    anyhow::ensure!(
        !additions.is_empty(),
        "every proposed name is already in the pool"
    );
    Ok(additions)
}

/// download without the run.
///
/// What this does *not* do is run the gate. The gate is a workspace build
/// that belongs under the session cargo lock and has to see the rebuilt pool;
/// printing the command and stopping is honest, where running it from inside
/// a `cargo run` would nest a second workspace build inside this one.
pub(crate) fn batch(
    root: &Path,
    scripts_dir: &Path,
    cache: &Path,
    count: usize,
    dry_run: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(count > 0, "a batch of no cards is not a batch");
    if !dry_run {
        refuse_a_dirty_tree(root)?;
    }

    let pool_path = root.join("data/card-pool.txt");
    let pool = fs::read_to_string(&pool_path)?;
    let additions = batch_names(root, scripts_dir, cache, count, &pool)?;
    if dry_run {
        println!("\nbatch: --dry-run, nothing written");
        return Ok(());
    }

    let mut body = pool;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    for name in &additions {
        body.push_str(name);
        body.push('\n');
    }
    fs::write(&pool_path, body)?;
    println!(
        "batch: {} name(s) appended to data/card-pool.txt",
        additions.len()
    );

    let scripts = scripts_dir.to_string_lossy().to_string();
    let cache_arg = cache.to_string_lossy().to_string();
    let codegen: Vec<&str> = vec!["codegen", "--scripts", &scripts, "--cache", &cache_arg];
    batch_step(
        root,
        "codegen, first phase (the cards themselves)",
        &codegen,
    )?;
    // The second phase is the whole reason this is a command. It is not a
    // retry: the ability-line and name tables are built from the compiled
    // pool, so the cards the first run wrote get their rows here, and only
    // `--check` below can tell whether somebody skipped it.
    batch_step(
        root,
        "codegen, second phase (the two compiled-pool tables)",
        &codegen,
    )?;
    // Asked before `--check`, because the two failures read the same in a
    // diff and mean opposite things: `--check` catching a stale table is this
    // batch not having finished, while a card nobody asked for moving is
    // somebody else's change arriving inside it.
    let drift = cards_changed_unasked(root, &additions)?;
    anyhow::ensure!(
        drift.is_empty(),
        "{} card file(s) this batch did not ask for were rewritten by codegen:\n{}\n\n\
         That is a printing or a reader that moved since they were last written, \
         and it does not belong in a commit about new cards. Take it on its own:\n\
         \n    cargo run -p xtask -- refresh-oracle\n\
         \nand commit that first. Then put this batch back and run it again:\n\
         \n    git restore . && git clean -f crates/cards/sets",
        drift.len(),
        drift
            .iter()
            .take(20)
            .map(|p| format!("  {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    println!(
        "batch: pass — no card outside this batch was rewritten ({} asked for)",
        additions.len()
    );

    let mut check = codegen.clone();
    check.push("--check");
    batch_step(
        root,
        "codegen --check (the two-phase gap, as a failure)",
        &check,
    )?;
    batch_step(
        root,
        "validate (headers, type lines, prices against the printing)",
        &["validate"],
    )?;

    println!(
        "\nbatch: {} card(s) added. What is left is the gate, which this does not run:\n\
         \n    BAYLEE_SESSION=<session> /Users/viktor/.baylee-locks/with-cargo-lock.sh \\\n\
         \x20       cargo test --workspace --all-targets\n\
         \nThe pool-wide engine sweeps are in it, and they are what actually play the\n\
         new cards. The way back, if it is red:\n\
         \n    git restore . && git clean -f crates/cards/sets\n",
        additions.len()
    );
    Ok(())
}

/// Cards explicitly held back from automated batch selection (`data/held-back.tsv`).
#[derive(Debug, Default)]
pub(crate) struct HeldBack {
    ids: BTreeSet<String>,
    names: BTreeSet<String>,
}

impl HeldBack {
    /// Returns `true` when either the row's Oracle id or printed card name is held back.
    pub(crate) fn contains(&self, oracle_id: &str, name: &str) -> bool {
        self.ids.contains(oracle_id) || self.names.contains(name)
    }
}

fn is_oracle_id(s: &str) -> bool {
    let mut groups = s.split('-');
    [8usize, 4, 4, 4, 12].into_iter().all(|len| {
        groups.next().is_some_and(|g| {
            g.len() == len
                && g.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    }) && groups.next().is_none()
}

/// Parses `data/held-back.tsv`, accepting both `<oracle_id>\t<name>\t<reason>`
/// and `<name>\t<reason>`, skipping blanks and `#` comments.
pub(crate) fn parse_held_back(text: &str) -> anyhow::Result<HeldBack> {
    let mut out = HeldBack::default();
    for (n, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        match cols.as_slice() {
            [name, reason] => {
                let (name, reason) = (name.trim(), reason.trim());
                anyhow::ensure!(
                    !name.is_empty() && !reason.is_empty(),
                    "data/held-back.tsv:{}: empty name or reason",
                    n + 1
                );
                out.names.insert(name.to_owned());
            }
            [id, name, reason] => {
                let (id, name, reason) = (id.trim(), name.trim(), reason.trim());
                anyhow::ensure!(
                    is_oracle_id(id) && !name.is_empty() && !reason.is_empty(),
                    "data/held-back.tsv:{}: expected <oracle_id>\\t<name>\\t<reason>",
                    n + 1
                );
                out.ids.insert(id.to_owned());
                out.names.insert(name.to_owned());
            }
            _ => anyhow::bail!(
                "data/held-back.tsv:{}: expected <oracle_id>\\t<name>\\t<reason> or <name>\\t<reason>",
                n + 1
            ),
        }
    }
    Ok(out)
}

/// Reads `data/held-back.tsv` under `root`.
pub(crate) fn held_back_set(root: &Path) -> anyhow::Result<HeldBack> {
    let path = root.join("data/held-back.tsv");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    parse_held_back(&text)
}

/// Resolves scripts from the live `dir` when present so stale `upcoming/*.txt`
/// entries in `data/script-index.json` never break `batch` or `reach-list`,
/// falling back to `data/script-index.json` when only a snapshot exists.
pub(crate) fn resolve_script_index(
    root: &Path,
    dir: &Path,
) -> anyhow::Result<BTreeMap<String, String>> {
    if dir.is_dir() {
        let index = scripts::build_index(dir)?;
        anyhow::ensure!(
            !index.is_empty(),
            "no card scripts found under {}",
            dir.display()
        );
        return Ok(index);
    }
    let script_index: BTreeMap<String, String> = serde_json::from_str(
        &fs::read_to_string(root.join("data/script-index.json")).unwrap_or_default(),
    )
    .unwrap_or_default();
    anyhow::ensure!(
        !script_index.is_empty(),
        "data/script-index.json is missing or empty; run `cargo xtask script-index`"
    );
    Ok(script_index)
}

/// Nontraditional / non-deck card types excluded by `docs/mechanics-roadmap.md` §C4/§E1
/// (`Vanguard`, `Plane`, `Scheme`, `Conspiracy`, `Phenomenon`, `Dungeon`, `Attraction`, `Stickers`).
pub(crate) fn is_non_deck_card_type(type_line: &str) -> bool {
    let mut has_planeswalker = false;
    let mut has_dungeon = false;
    for word in type_line.split_whitespace() {
        match word {
            "Vanguard" | "Plane" | "Scheme" | "Conspiracy" | "Phenomenon" | "Attraction"
            | "Stickers" => return true,
            "Planeswalker" => has_planeswalker = true,
            "Dungeon" => has_dungeon = true,
            _ => {}
        }
    }
    has_dungeon && !has_planeswalker
}

/// Digital-only Alchemy rebalances (`A-*` in the append-only ledger) were dropped
/// from Scryfall feeds and `/cards/named?exact=`, causing `fetch_named` to fall
/// back to `?fuzzy=` and collide with the paper card's file path.
pub(crate) fn is_alchemy_rebalance(name: &str) -> bool {
    name.starts_with("A-")
}

/// What a reach measurement found: the names the reader can write today, and
/// the counts that say whether the walk itself did its job.
pub(crate) struct Reach {
    names: Vec<&'static str>,
    mine: usize,
    alchemy: usize,
    without_paper: usize,
    held_back: usize,
    non_deck: usize,
    scripted: usize,
    refused: usize,
    front: usize,
}

/// Walks the ledger and asks the transcoder, once, for both readers of this
/// worklist: `reach-list`, which prints and writes it, and `batch`, which
/// takes the first `count` of it.
pub(crate) fn reach_measure(
    root: &Path,
    scripts_dir: &Path,
    cache: &Path,
) -> anyhow::Result<Reach> {
    let dir = scripts_root(root, scripts_dir);
    let tokens = tokengen::TokenLookup::beside(&dir)?;
    let agent = ureq::Agent::new_with_defaults();
    let cache = root.join(cache);
    let cats = scryfall::fetch_subtype_catalogs(&agent, &cache)?;
    let paper = scryfall::paper_oracle_ids(&agent, &cache)?;
    let script_index = resolve_script_index(root, &dir)?;
    let have: BTreeSet<&str> = baylee_cards::all().map(|def| def.oracle_id).collect();
    // Never built, so never proposed (data/unplayable.tsv, A-* Alchemy, data/held-back.tsv).
    let unplayable = unplayable_ids(root)?;
    let held_back_list = held_back_set(root)?;
    let (mut mine, mut alchemy, mut held_back, mut non_deck, mut scripted, mut refused, mut front) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut without_paper = 0;
    let mut names: Vec<&'static str> = Vec::new();
    // By reference: `ROWS` is a 33 694-element array and not a slice, so
    // `for row in ROWS` copies every one of them onto the stack — measured,
    // because it is a stack overflow before the first script is read and it
    // looks exactly like a runaway parser.
    for row in &baylee_cards_index::ROWS {
        if have.contains(row.oracle_id) {
            mine += 1;
            continue;
        }
        if unplayable.contains_key(row.oracle_id) {
            continue;
        }
        if is_alchemy_rebalance(row.name) {
            alchemy += 1;
            continue;
        }
        if !paper.contains(row.oracle_id) {
            without_paper += 1;
            continue;
        }
        if held_back_list.contains(row.oracle_id, row.name) {
            held_back += 1;
            continue;
        }
        let Some(rel) = script_for(&script_index, row.name) else {
            continue;
        };
        let text = fs::read_to_string(dir.join(rel))?;
        let script = scriptgen::parse(&text);
        if is_non_deck_card_type(&script.type_line) {
            non_deck += 1;
            continue;
        }
        scripted += 1;
        if !script_index.contains_key(row.name) {
            front += 1;
        }
        if scriptgen::transcode(&script, &cats, tokens.as_ref()).is_some()
            || scriptgen::is_vanilla(&script)
        {
            names.push(row.name);
        } else {
            refused += 1;
        }
    }
    Ok(Reach {
        names,
        mine,
        alchemy,
        without_paper,
        held_back,
        non_deck,
        scripted,
        refused,
        front,
    })
}

pub(crate) fn reach_list(
    root: &Path,
    scripts_dir: &Path,
    out: Option<&Path>,
    count: usize,
    cache: &Path,
) -> anyhow::Result<()> {
    let Reach {
        mut names,
        mine,
        alchemy,
        without_paper,
        held_back,
        non_deck,
        scripted,
        refused,
        front,
    } = reach_measure(root, scripts_dir, cache)?;
    println!(
        "reach: {} ledger rows — {mine} already in the pool, {scripted} of the rest have a \
         reference script, {} of those read in full ({refused} refused)",
        baylee_cards_index::ROWS.len(),
        names.len()
    );
    if alchemy > 0 {
        println!("reach: {alchemy} digital Alchemy A-* row(s) skipped");
    }
    if without_paper > 0 {
        println!("reach: {without_paper} ledger row(s) without a paper printing skipped");
    }
    if held_back > 0 {
        println!("reach: {held_back} held-back row(s) skipped (data/held-back.tsv)");
    }
    if non_deck > 0 {
        println!(
            "reach: {non_deck} non-deck row(s) skipped (Vanguard/Plane/Scheme/Conspiracy/Dungeon/Attraction/Stickers)"
        );
    }
    // Reported rather than assumed: a second tier that matched nothing would
    // be a lookup quietly answering for one spelling only, and this is the
    // number that says it is doing its job.
    println!("reach: {front} of them were found by their front face alone");
    if count > 0 && names.len() > count {
        println!("reach: writing the first {count} in ledger order");
        names.truncate(count);
    }
    if let Some(path) = out {
        let mut body = names.join("\n");
        body.push('\n');
        fs::write(path, body)?;
        println!(
            "reach: {} name(s) written to {}",
            names.len(),
            path.display()
        );
    } else {
        for name in names.iter().take(20) {
            println!("  {name}");
        }
        if names.len() > 20 {
            println!("  … and {} more (--out to write them)", names.len() - 20);
        }
    }
    Ok(())
}

/// The reference script for a ledger name, at the two tiers a two-faced card
/// is spelled at.
///
/// The ledger writes a card's whole name (`Fire // Ice`) because Scryfall
/// does; the reference names some of them that way and some by their front
/// face alone, so one lookup answers for part of the corpus and says nothing
/// about the rest.
pub(crate) fn script_for<'a>(
    index: &'a BTreeMap<String, String>,
    name: &str,
) -> Option<&'a String> {
    index
        .get(name)
        .or_else(|| index.get(name.split(" // ").next().unwrap_or(name)))
}

/// Chooses the cards that would teach the engine the most, and says what
/// each one asks for.
///
/// One refused reference script, reduced to the mechanics it uses.
pub(crate) struct Card {
    name: String,
    atoms: Vec<String>,
}

/// Picks `count` cards, each time taking the one whose still-uncovered atoms
/// block the most other scripts. `max_new` keeps a card that would drag in a
/// dozen unrelated mechanics out of the plan — it is a worklist, and an item
/// nobody can finish is not one.
pub(crate) fn greedy_pick(
    refused: &[Card],
    demand: &BTreeMap<String, usize>,
    covered: &mut BTreeSet<String>,
    count: usize,
    max_new: usize,
) -> Vec<(String, Vec<String>, usize)> {
    let mut chosen = Vec::new();
    let mut taken = vec![false; refused.len()];
    for _ in 0..count {
        let mut best: Option<(usize, usize, Vec<String>)> = None;
        for (i, card) in refused.iter().enumerate() {
            if taken[i] {
                continue;
            }
            let new: Vec<String> = card
                .atoms
                .iter()
                .filter(|a| !covered.contains(*a))
                .cloned()
                .collect();
            if new.is_empty() || new.len() > max_new {
                continue;
            }
            let score: usize = new
                .iter()
                .map(|a| demand.get(a).copied().unwrap_or(0))
                .sum();
            if best.as_ref().is_none_or(|(_, b, _)| score > *b) {
                best = Some((i, score, new));
            }
        }
        let Some((i, score, new)) = best else { break };
        taken[i] = true;
        covered.extend(new.iter().cloned());
        chosen.push((refused[i].name.clone(), new, score));
    }
    chosen
}

/// Greedy set cover, with one deliberate twist: a card's score is not how
/// many new atoms it has but how many *other cards in the corpus* those
/// atoms block. A mechanic one card uses is a curiosity; a mechanic four
/// hundred cards use is the next thing to build.
pub(crate) fn coverage_set(
    root: &Path,
    scripts_dir: &Path,
    count: usize,
    max_new: usize,
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

    let mut refused: Vec<Card> = Vec::new();
    // An atom in a script the transcoder reads in full is, by definition,
    // already handled — no rule list has to be restated here to know it.
    let mut known: BTreeSet<String> = BTreeSet::new();
    let mut demand: BTreeMap<String, usize> = BTreeMap::new();
    for path in &files {
        let text = fs::read_to_string(path)?;
        let script = scriptgen::parse(&text);
        let atoms = scriptgen::atoms(&script);
        if scriptgen::transcode(&script, &cats, tokens.as_ref()).is_some() {
            known.extend(atoms);
            continue;
        }
        for atom in &atoms {
            *demand.entry(atom.clone()).or_insert(0) += 1;
        }
        let name = text
            .lines()
            .find_map(|l| l.strip_prefix("Name:"))
            .unwrap_or_default()
            .trim()
            .to_string();
        if !name.is_empty() {
            refused.push(Card { name, atoms });
        }
    }

    let mut covered = known;
    let chosen = greedy_pick(&refused, &demand, &mut covered, count, max_new);

    let blocked_before = refused.len();
    let still_blocked = refused
        .iter()
        .filter(|c| c.atoms.iter().any(|a| !covered.contains(a)))
        .count();
    println!(
        "coverage plan: {} cards; {} of {blocked_before} refused scripts would have every \
         mechanic they use ({}%)",
        chosen.len(),
        blocked_before - still_blocked,
        ((blocked_before - still_blocked) * 100)
            .checked_div(blocked_before)
            .unwrap_or(0)
    );
    println!(
        "(a script with every mechanic covered is not automatically read — the rule for \
              each one still has to be written; this is the worklist, not the result)"
    );
    for (n, (name, new, score)) in chosen.iter().enumerate() {
        println!("{:>4}. {name}  [unblocks {score}]", n + 1);
        for atom in new {
            println!(
                "        {atom}  ({} scripts)",
                demand.get(atom).copied().unwrap_or(0)
            );
        }
    }
    Ok(())
}

/// The names of the cards whose generated file is still a stub.
///
/// Read from the marker rather than from a list, because the marker is
/// what `codegen` itself honours: a file that has lost it is hand-owned and
/// will not be rewritten, so ranking it as work would be ranking work
/// nobody can do.
pub(crate) fn stub_names(cards_dir: &Path) -> anyhow::Result<(BTreeSet<String>, usize)> {
    let mut out = BTreeSet::new();
    let mut cards = 0usize;
    // Through `card_files` and never `read_dir`: `crates/cards/sets` is a tree, and
    // a non-recursive walk over it would find nothing at all and report an
    // empty worklist as an answer rather than as a failure.
    for path in card_files(cards_dir)?.values() {
        let text = fs::read_to_string(path)?;
        if !text.contains("// GENERATED STUB") {
            continue;
        }
        cards += 1;
        // The header's first line is `//! <name> — <cost> — <types>`, and a
        // double-faced card's name there is `<front> // <back>` — while the
        // reference script for the pair is headed `Name:<front>` and holds the
        // back face after an `ALTERNATE` line. Matching only the joined name
        // made this worklist blind to every one of them: 104 of the 775
        // stubs, including all 81 that are not lands, and with them the
        // largest single entry the report could have had.
        if let Some(head) = text.lines().next().and_then(|l| l.strip_prefix("//! ")) {
            let name = head.split(" \u{2014} ").next().unwrap_or(head).trim();
            out.insert(name.to_string());
            if let Some((front, _)) = name.split_once(" // ") {
                out.insert(front.to_string());
            }
        }
    }
    Ok((out, cards))
}

pub(crate) fn collect_scripts(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_scripts(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "txt") {
            out.push(path);
        }
    }
    Ok(())
}

/// The most likely single reason a script was refused, for ranking work.
///
/// This is a heuristic over the script's own text rather than a report from
/// the transcoder: it names the first thing in the script that no rule
/// claims, which is what makes the output a worklist.
pub(crate) fn refusal_cause(
    script: &scriptgen::CardScript,
    cats: &catalog::SubtypeCatalogs,
    tokens: Option<&tokengen::TokenLookup>,
) -> String {
    if let Some(line) = script.unknown_lines.first() {
        let head = line.split(':').next().unwrap_or(line);
        return format!("unmodelled line kind `{head}:`");
    }
    // Ask the transcoder before guessing. It knows which line it stopped
    // on and why; re-reading the script here only knows what *this* function
    // recognises, which is how every unexplained refusal used to be filed
    // under a label that named the wrong work.
    //
    // A `K:` line used to be answered here first, out of a second reader
    // that knew only which keyword words existed. That reader went wrong the
    // moment a keyword became a *rule* with a parameter to read: equip for a
    // sacrifice is read and equip for a legendary creature is not, and the
    // list could not tell them apart — so it reported `keyword Equip` over a
    // thousand times for scripts the transcoder had stopped on somewhere
    // else entirely. The transcoder reports its own first refusal, and a
    // keyword is not a special case of that.
    if let Some(why) = scriptgen::refusal_reason(script, cats, tokens) {
        return why;
    }
    for (kind, spec) in &script.rules {
        for api in scriptgen::apis_used(spec, &script.svars) {
            if !scriptgen::is_supported_api(&api) {
                return format!("effect `{api}`");
            }
        }
        if *kind == 'R' {
            return "replacement effect (R:)".to_string();
        }
    }
    "refused with no reason recorded".to_string()
}

#[cfg(test)]
mod tests {
    use super::{HeldBack, parse_held_back, resolve_script_index};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    #[test]
    fn held_back_parses_both_layouts_and_rejects_malformed_rows() {
        let ok = "\
# comment line

79591c18-0065-49bb-a77a-734d4af87a3c\tBloodhall Ooze\treader-gap
Fake By Name\tfixture-gap
";
        let held: HeldBack = parse_held_back(ok).expect("valid held-back entries");
        assert!(held.contains("79591c18-0065-49bb-a77a-734d4af87a3c", "Other Name"));
        assert!(held.contains("00000000-0000-0000-0000-000000000000", "Bloodhall Ooze"));
        assert!(held.contains("00000000-0000-0000-0000-000000000000", "Fake By Name"));
        assert!(!held.contains("00000000-0000-0000-0000-000000000000", "Lightning Bolt"));

        for bad in [
            "just-one-column\n",
            "Name Only\t\n",
            "\treason\n",
            "not-a-uuid\tBloodhall Ooze\treader-gap\n",
            "79591c18-0065-49bb-a77a-734d4af87a3c\tBloodhall Ooze\textra\tcol\n",
        ] {
            assert!(parse_held_back(bad).is_err(), "should reject: {bad:?}");
        }
    }

    #[test]
    fn repository_held_back_file_matches_ledger_and_excludes_implemented() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/held-back.tsv");
        let text = fs::read_to_string(&path).expect("read data/held-back.tsv");
        let held = parse_held_back(&text).expect("parse data/held-back.tsv");

        let by_id: BTreeMap<&str, &str> = baylee_cards_index::ROWS
            .iter()
            .map(|row| (row.oracle_id, row.name))
            .collect();
        for (n, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() == 3 {
                let (id, name) = (cols[0].trim(), cols[1].trim());
                assert_eq!(
                    by_id.get(id).copied(),
                    Some(name),
                    "data/held-back.tsv:{} mismatches CardIndex ledger",
                    n + 1
                );
            }
        }

        for expected_held in [
            ("79591c18-0065-49bb-a77a-734d4af87a3c", "Bloodhall Ooze"),
            (
                "ac136d46-32b7-4222-a840-b8be011c1872",
                "Zethi, Arcane Blademaster",
            ),
            ("947a2665-2f4d-4193-8768-118f85334549", "Abundant Growth"),
            ("2970d09b-4bef-468e-8de5-f994c820ccc0", "Pestbrood Sloth"),
            ("5f5d6f2d-2793-4355-b3f6-6eba42603756", "A-The One Ring"),
        ] {
            assert!(
                held.contains(expected_held.0, expected_held.1),
                "missing expected held-back card {expected_held:?}"
            );
        }

        for already_implemented in [
            ("8dac693b-c5c5-4d80-9739-a3b95949a213", "Graceful Antelope"),
            ("1af26f29-627c-48e5-a19d-deb43a11a846", "Hazy Homunculus"),
            ("4fcfe370-ba33-438f-bd96-2ae560f59df9", "Wall of Vipers"),
            ("59489b46-9d02-4f3c-bcd0-884e7605e9a5", "Opalescence"),
            ("245e26ec-39eb-4af8-99af-8cc22edbc261", "Mercenary Informer"),
        ] {
            assert!(
                !held.contains(already_implemented.0, already_implemented.1),
                "already-implemented card should not be in data/held-back.tsv: {already_implemented:?}"
            );
        }
    }

    #[test]
    fn resolve_script_index_prefers_live_scripts_dir_over_stale_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(
            root.join("data/script-index.json"),
            serde_json::json!({ "Stale Card": "upcoming/stale.txt" }).to_string(),
        )
        .unwrap();

        let missing_dir = root.join("missing");
        let fallback = resolve_script_index(root, &missing_dir).expect("snapshot fallback");
        assert_eq!(
            fallback.get("Stale Card").map(String::as_str),
            Some("upcoming/stale.txt")
        );

        let live_dir = root.join("cardsfolder");
        fs::create_dir_all(live_dir.join("a")).unwrap();
        fs::write(live_dir.join("a/live_card.txt"), "Name:Live Card\n").unwrap();
        let live = resolve_script_index(root, &live_dir).expect("live scripts dir");
        assert!(!live.contains_key("Stale Card"));
        assert_eq!(
            live.get("Live Card").map(String::as_str),
            Some("a/live_card.txt")
        );
    }

    #[test]
    fn alchemy_rebalance_predicate_matches_only_a_dash_rows_in_ledger() {
        use super::is_alchemy_rebalance;

        assert!(is_alchemy_rebalance("A-Blood Artist"));
        assert!(is_alchemy_rebalance("A-The One Ring"));
        assert!(!is_alchemy_rebalance("Abu Ja'far"));
        assert!(!is_alchemy_rebalance("A Display of My Dark Power"));

        let mut matched = 0usize;
        for row in &baylee_cards_index::ROWS {
            if is_alchemy_rebalance(row.name) {
                assert!(
                    row.constant.starts_with("A_"),
                    "non-Alchemy ledger row matched A- filter: {} ({})",
                    row.name,
                    row.constant
                );
                matched += 1;
            }
        }
        assert_eq!(matched, 216);
    }

    #[test]
    fn filter_batch_additions_deduplicates_acceptance_decks_and_front_face_slugs() {
        use super::filter_batch_additions;

        let decks = "[deck:Sample]\n1 Delver of Secrets\n";
        let pool = "Fire // Ice\n";
        let proposed = [
            "Delver of Secrets // Insectile Aberration",
            "Fire",
            "Wear // Tear",
            "Wear",
            "Lightning Bolt",
        ];
        let added = filter_batch_additions(pool, decks, &proposed).expect("filter additions");
        assert_eq!(added, vec!["Wear // Tear", "Lightning Bolt"]);
    }

    #[test]
    fn non_deck_card_type_predicate_skips_vanguard_plane_scheme_conspiracy_dungeon_but_keeps_normal_cards()
     {
        use super::is_non_deck_card_type;

        for excluded in [
            "Vanguard",
            "Plane Mirrodin",
            "Ongoing Scheme",
            "Scheme",
            "Conspiracy",
            "Phenomenon",
            "Dungeon",
            "Artifact Attraction",
            "Stickers",
        ] {
            assert!(
                is_non_deck_card_type(excluded),
                "expected non-deck type line to be skipped: {excluded}"
            );
        }

        for normal in [
            "Legendary Planeswalker Dungeon Master",
            "Creature Human Soldier",
            "Legendary Creature Human Wizard",
            "Sorcery",
            "Artifact",
            "Instant",
            "Battle Siege",
        ] {
            assert!(
                !is_non_deck_card_type(normal),
                "expected normal deck card type line to be kept: {normal}"
            );
        }
    }
}
