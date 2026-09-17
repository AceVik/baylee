//! xtask — baylee development tasks (codegen, card explanation, …).

mod cr_check;

use baylee_cards_codegen::{
    acceptance, cardindex, catalog, landgen, layout, ledger, lines, names, scriptgen, scripts,
    scryfall, stubgen, tokenledger,
};
use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "xtask", about = "baylee development tasks", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Regenerate subtype constants, card stubs, registry, and the script index.
    Codegen {
        /// Verify generated files are up to date instead of writing (CI).
        #[arg(long)]
        check: bool,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Assign a `CardIndex` to every card in the corpus that has none.
    ///
    /// The only thing that writes the ledger, and the reason `codegen` never
    /// does: an index assigned as a side effect of a build takes its order
    /// from whatever happened to be fetched that day. This takes its order
    /// from the corpus — first appearance, over every card that exists — so
    /// adopting an old card inserts nothing.
    ///
    /// The ledger is `crates/baylee-cards-index/src/generated.rs`, a compiled
    /// table rather than a data file, because a data file is a second truth
    /// beside the code that nothing checks. So this reads the table it is
    /// about to write — one build old, which is safe because assignment only
    /// ever appends — and the build is what checks what came out.
    ///
    /// The corpus comes from `baylee-catalog corpus`, which needs a database.
    /// This half needs only the file it writes, so codegen stays offline.
    Ledger {
        /// The corpus, as `baylee-catalog corpus` writes it.
        #[arg(long, default_value = "data/card-corpus.tsv")]
        corpus: PathBuf,
        /// Report what would be assigned instead of writing it.
        #[arg(long)]
        check: bool,
        /// Throw every assignment away and number the corpus from zero.
        ///
        /// It reads no ledger at all — nothing is carried over, because a
        /// carried row is the one number the corpus did not produce. What
        /// protects a card this repo implements is the check at the end: the
        /// run asks the **registry** whether every compiled card got a row
        /// and refuses to write if one did not, and the answer to a refusal
        /// is `data/corpus-keep.tsv`. Every stored `CardIndex` anywhere
        /// becomes wrong, so this is a deliberate, one-off,
        /// migrate-or-discard operation and never a part of maintenance.
        #[arg(long)]
        reseed: bool,
    },
    /// Measure how well the compiled ability list lines up with the printed sentences.
    ///
    /// A report, not a check: it is the design input for the per-ability
    /// line index the stack panel needs.
    AbilityLines,
    /// Dump every compiled `CardDef` — the equivalence check for a refactor.
    ///
    /// A change that is meant to alter no rules (new macros, shared filters,
    /// a reshuffled literal) must leave this output byte-identical. Take a
    /// dump before, one after, and diff: anything that moved has the card's
    /// name on it. It is a tool rather than a test because there is nothing
    /// for it to assert on its own — the baseline lives outside the repo.
    PoolDump {
        /// Where to write the dump.
        #[arg(long)]
        out: PathBuf,
    },
    /// Report how much of the card-script reference corpus the transcoder reads.
    TranscodeReport {
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Print this many refused scripts, for finding the next rule to add.
        #[arg(long, default_value_t = 0)]
        samples: usize,
        /// Show only samples whose refusal reason contains this text.
        ///
        /// The ranking names a reason; this is how you read the scripts
        /// behind one of them without grepping the corpus by hand and
        /// guessing which of them the transcoder actually stopped on.
        #[arg(long)]
        reason: Option<String>,
        /// Rank only this project's own unfinished cards.
        ///
        /// The corpus is 33666 scripts; the deckbuilder offers about a
        /// thousand, and most of those are already finished — the lands by
        /// `landgen`, the rest by hand. So the two rankings answer different
        /// questions: the corpus says what the transcoder is worth in
        /// general, and this says which of *our* stubs the next rule would
        /// finish, which is the one a player would notice.
        #[arg(long)]
        stubs: bool,
        /// How many refusal causes to rank. 0 prints every one of them.
        ///
        /// The ranking is long — a corpus run distinguishes several hundred
        /// causes — and the tail is where a mechanic this project is *about
        /// to* write sits, because a rule nothing needs yet blocks a handful
        /// of scripts rather than a thousand. The default is a screenful,
        /// and what it leaves out is counted rather than dropped in silence.
        #[arg(long, default_value_t = 30)]
        causes: usize,
    },
    /// Read every hand-written card a second way and report the disagreements.
    ///
    /// A hand-written card is the one surface in the pool that no program
    /// checks against another program. `validate` pins its header to the
    /// printing and the engine sweeps pin its behaviour to the `CardDef` it
    /// builds — but the `CardDef` itself was typed by a person reading the
    /// card, and a person who read a clause wrong produces a card every one
    /// of those checks agrees with. The transcoder is a second reader of the
    /// same card from a different source, so where it reads a script **in
    /// full** and comes out with a different shape, one of the two is wrong.
    ///
    /// A disagreement never fails it. The transcoder writes what one rule
    /// can say, and a hand-written card is allowed to say more (a mode, a
    /// ward, a saga chapter). What the list is, is the shortest set of cards
    /// worth a second pair of eyes, and it is ranked by nothing — every line
    /// on it names a card and what the two readers disagreed about.
    ///
    /// The second reader is read at whichever of **two depths** the script
    /// reaches. Transcoding is the deep one and needs every clause claimed,
    /// which a hand-written card's script almost never offers — it is
    /// hand-written *because* a reader could not write it, so only 22 of the
    /// 207 overlap and that number shrinks as the transcoder grows. Counting
    /// the parser's own line kinds is the shallow one and reaches 141 of the
    /// 207 — those same 22 and 119 more: a script stopped on one unclaimed
    /// parameter still says plainly how many abilities it has and of what
    /// kind.
    ///
    /// Both depths obey the transcoder's honesty rule — **one unread clause
    /// and the script is not counted** — because the alternative is a tool
    /// that reports its own blind spots as defects in the cards. The skips
    /// are counted and *named* for the same reason, so the tail of the
    /// report is a worklist: thirteen cards are unreadable only because
    /// `K:ETBReplacement` hides an ability inside an `SVar` chain, and five
    /// only because `keyword_const` has no row for daybound.
    ///
    /// What it caught on its first honest run is the whole argument for it,
    /// and it is worth stating exactly, because the tool compares **shape**
    /// and nothing else. It pointed at *one* card: Raffine's Tower, which
    /// claimed `Coverage::Implemented` with no basic land types and no
    /// cycling ability at all. Reading the three neighbours in that cycle by
    /// hand — which is what a report is for — found Indatha, Raugrin and
    /// Zagoth cycling for `{2}` against the `{3}` their own `//! Oracle:`
    /// header prints. A wrong cost is invisible here by construction, so
    /// three of the four are this command's credit only in the sense that it
    /// put a person in front of the right file. Every other check in the repo
    /// agreed with all four, which is the hole this fills: `validate` pins a
    /// header to its printing and the sweeps pin behaviour to the `CardDef`,
    /// and neither can see a `CardDef` a person typed wrong.
    CrossRead {
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Print this many disagreeing cards with the script that caused it.
        #[arg(long, default_value_t = 0)]
        samples: usize,
    },
    /// Hold every `CR` citation in the tree against a local rules copy.
    ///
    /// A report and not a gate, for a reason that is not taste: the rules
    /// text is Wizards' and is deliberately not vendored (`docs/legal.md`
    /// §2), so CI has no copy and nothing in the test suite can read one.
    /// The citations rot anyway — an audit of all 1693 of them found 246
    /// naming the wrong rule, because Wizards renumbers a section whenever
    /// they insert a keyword action into it and sacrifice moved from 701.19a
    /// to 701.21a without anybody here touching a file.
    ///
    /// It checks two mechanical things. That the number exists at all. And
    /// that a line using one of the rules' **own** heading words — every
    /// `701.N` is a keyword action and every `702.N` a keyword ability —
    /// cites a number under that word. The second is the one worth having,
    /// because it is the exact shape the drift takes: the prose stays right
    /// and the number slides out from under it.
    ///
    /// `xtask/src/cr_check.rs` has the four rules that keep it quiet on the
    /// correct citations, each of which was added because it fired on one.
    CrCheck {
        /// A Comprehensive Rules text, relative to the repository or absolute.
        #[arg(long, default_value = "../mtg/MagicCompRules.txt")]
        rules: PathBuf,
        /// Print every citation beside the rule it names, not just the findings.
        #[arg(long)]
        all: bool,
    },
    /// Rank the land sentences `landgen` cannot read yet.
    ///
    /// The counterpart to `transcode-report`, and worth its own command for the
    /// reason that one exists: every card in the pool that is still a
    /// generated stub is a land — 792 of them — and land text is formulaic.
    /// A sentence shape taught to `landgen` is not one card; it is every land
    /// that prints that shape, generated with nobody reading the result.
    LandReport {
        /// Name up to this many example lands per shape.
        #[arg(long, default_value_t = 3)]
        samples: usize,
        /// Write the cards a parser should *not* be taught, one name per line.
        ///
        /// The split this command exists to make. A shape printed on eight
        /// lands repays a rule in `landgen`; a shape printed on one is a rule
        /// per card, which is what a person or a model does. This writes the
        /// second half — the tail, plus the lands that are not plain lands at
        /// all — in the form `card-batch --cards` takes.
        #[arg(long)]
        worklist: Option<PathBuf>,
        /// A shape printed on fewer than this many lands goes to the worklist.
        #[arg(long, default_value_t = 2)]
        tail: usize,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Choose the cards that would teach the engine the most, and say what
    /// each one asks for.
    ///
    /// Greedy set cover over the mechanics the corpus actually uses: every
    /// script contributes atoms (`api:Token`, `param:Pump.Duration`,
    /// `kw:Equip`, `line:S`), atoms already appearing in scripts the
    /// transcoder reads in full are struck off as known, and each card is
    /// scored by how many *cards elsewhere in the corpus* its remaining
    /// atoms would unblock. Picking by hand instead reliably picks famous
    /// cards, which are famous for their flavour, not their mechanics.
    CoverageSet {
        /// How many cards to choose.
        #[arg(long, default_value_t = 100)]
        count: usize,
        /// Skip a card that would need more than this many new mechanics —
        /// a planeswalker with three novel modes is a worse first card than
        /// three cards with one each.
        #[arg(long, default_value_t = 6)]
        max_new: usize,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
    },
    /// Show Scryfall + card-script reference data for a card side by side.
    Explain {
        /// Exact card name.
        #[arg(long)]
        name: String,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Prepare per-card task packages for LLM implementation batches.
    CardBatch {
        /// Only these cards (comma-separated names); default: every card in
        /// the pool that is still a generated stub.
        #[arg(long)]
        cards: Option<String>,
        /// Output directory for task packages.
        #[arg(long, default_value = "target/card-batch")]
        out: PathBuf,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Fill the Scryfall payload cache, and nothing else.
    ///
    /// What `codegen` does on its way to writing files, as a command of its
    /// own — so a scheduled job can warm the cache without building the
    /// generated tree, and without the card-script corpus it would need to
    /// generate anything. A cold cache is one bulk download and a handful of
    /// requests; a warm one is no network at all.
    ScryfallCache {
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Validate card-file conventions (header, coverage, tests).
    Validate,
    /// Take a generated card off the machine, so a person owns it from now on.
    ///
    /// A card one of the readers wrote in full is **machine-owned**: codegen
    /// rewrites it on every run, which is what makes "fix the reader, not the
    /// card" enforceable rather than a convention — a rule corrected in
    /// `landgen` or `scriptgen` reaches every card that rule wrote, at once.
    /// The cost is that a hand edit to such a file is reverted on the next
    /// run, silently as far as the editor is concerned.
    ///
    /// This is the way out, and it is deliberately a decision someone makes
    /// on purpose: it strips the ownership marker, and from then on the file
    /// is hand-owned like any card a person wrote from the stub. Reach for it
    /// when the card genuinely needs something the reader cannot say — not to
    /// get past a transcoding bug, which belongs in the reader where it fixes
    /// the other cards it also broke.
    Adopt {
        /// Printed card name, as the pool spells it.
        #[arg(long)]
        name: String,
    },
    /// Rewrite every card's `//! Oracle:` header from its cached printing.
    ///
    /// The header is *derived* data — Scryfall's own text, copied into the
    /// file so a person can read the card beside the code built from it — so
    /// a tool writes it and `validate` compares it. Hand-editing forty-eight
    /// of them by retyping rules text is exactly the transcription step this
    /// check exists to catch.
    ///
    /// It touches the header and nothing else, hand-owned files included:
    /// what a card *does* stays whoever's it was, and only the sentence it
    /// is measured against is refreshed. That is also how an errata is taken
    /// — refresh, then read the diff, because a changed printing usually
    /// means the implementation below it has to change too.
    RefreshOracle {
        /// Print what would change and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Seat a dev account at a table and print (or play) its ticket.
    ///
    /// Skips the lobby's sign-in and deck-picking screens and nothing else:
    /// the account, the deck, the room and the seat are all made through the
    /// gateway's own HTTP routes, and the game that comes out is played over
    /// the same engine ⇄ gateway ⇄ client sockets as any other.
    DevTable {
        /// Gateway base URL.
        #[arg(long, default_value = "http://127.0.0.1:28766")]
        gateway: String,
        /// How many chairs. Two is the one-tap game against the house; more
        /// opens a room and hands every other chair to the AI.
        #[arg(long, default_value_t = 2)]
        seats: usize,
        /// Which difficulty the AI chairs play at.
        #[arg(long, default_value = "steady")]
        ai: String,
        /// Which acceptance deck to bring.
        #[arg(long, default_value = "Allytifact")]
        deck: String,
        /// Which side each chair plays for, in seat order — `1,1,2` is a
        /// 2v1. `0` leaves a chair on its own side. Needs three chairs or
        /// more, a duel already having exactly two sides.
        #[arg(long, value_delimiter = ',')]
        teams: Vec<u8>,
        /// Launch the client on the seat instead of printing its ticket.
        #[arg(long)]
        play: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in <workspace>/xtask")
        .to_path_buf();
    match cli.cmd {
        Cmd::Codegen {
            check,
            scripts,
            cache,
        } => codegen(&root, check, &scripts, &cache),
        Cmd::Ledger {
            corpus,
            check,
            reseed,
        } => ledger_cmd(&root, &corpus, check, reseed),
        Cmd::AbilityLines => ability_lines(&root),
        Cmd::PoolDump { out } => pool_dump(&out),
        Cmd::TranscodeReport {
            scripts,
            samples,
            stubs,
            reason,
            causes,
        } => transcode_report(&root, &scripts, samples, stubs, reason.as_deref(), causes),
        Cmd::CrossRead { scripts, samples } => cross_read(&root, &scripts, samples),
        Cmd::CrCheck { rules, all } => cr_check::run(&root, &rules, all),
        Cmd::LandReport {
            samples,
            worklist,
            tail,
            cache,
        } => land_report(&root, &cache, samples, worklist.as_deref(), tail),
        Cmd::CoverageSet {
            count,
            max_new,
            scripts,
        } => coverage_set(&root, &scripts, count, max_new),
        Cmd::Explain {
            name,
            scripts,
            cache,
        } => explain(&root, &name, &scripts, &cache),
        Cmd::CardBatch {
            cards,
            out,
            scripts,
            cache,
        } => card_batch(&root, cards.as_deref(), &out, &scripts, &cache),
        Cmd::ScryfallCache { cache } => scryfall_cache(&root, &cache),
        Cmd::Validate => validate(&root),
        Cmd::Adopt { name } => adopt(&root, &name),
        Cmd::RefreshOracle { dry_run } => refresh_oracle(&root, dry_run),
        Cmd::DevTable {
            gateway,
            seats,
            ai,
            deck,
            teams,
            play,
        } => dev_table(&root, &gateway, seats, &ai, &deck, &teams, play),
    }
}

/// Formats Rust source with the toolchain's rustfmt so generated files are
/// fmt-stable (`cargo fmt --check` and `codegen --check` never conflict).
fn format_rust(content: &str) -> anyhow::Result<String> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(content.as_bytes())?;
    let out = child.wait_with_output()?;
    if out.status.success() {
        Ok(String::from_utf8(out.stdout)?)
    } else {
        // Unparseable generated code should fail at compile time anyway;
        // keep the raw text so the error points at the real file.
        Ok(content.to_string())
    }
}

/// Every card file under `cards/`, wherever the taxonomy has put it, keyed by
/// slug.
///
/// The slug is the file stem and the module name both, so two files claiming
/// one slug is not a layout question but a broken tree — `cards/mod.rs` could
/// only declare one of them, and which one would depend on directory order.
/// `mod.rs` itself is the tree's own bookkeeping and is skipped at every
/// level.
fn card_files(dir: &Path) -> anyhow::Result<BTreeMap<String, PathBuf>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(here) = stack.pop() {
        let Ok(entries) = fs::read_dir(&here) else {
            continue;
        };
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if stem == "mod" {
                continue;
            }
            if let Some(first) = out.insert(stem.to_string(), path.clone()) {
                anyhow::bail!(
                    "two files claim the slug {stem}: {} and {}",
                    first.display(),
                    path.display()
                );
            }
        }
    }
    Ok(out)
}

fn write_or_check(
    check: bool,
    path: &Path,
    content: &str,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let owned;
    let content = if path.extension().is_some_and(|e| e == "rs") {
        owned = format_rust(content)?;
        owned.as_str()
    } else {
        content
    };
    write_verbatim(check, path, content, changed)
}

/// The same, for generated text that is already in the form rustfmt would
/// leave it in.
///
/// The `CardIndex` tree is 383 files of one-line constants, and running
/// rustfmt over each of them costs a process apiece on every `codegen` and
/// every CI `--check` to change nothing. `the_index_tree_is_already_
/// formatted` in `xtask` is what holds the claim: if the renderer ever emits
/// something rustfmt would rewrite, `cargo fmt --all` and `codegen --check`
/// would disagree forever, each undoing the other.
fn write_verbatim(
    check: bool,
    path: &Path,
    content: &str,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing == content {
        return Ok(());
    }
    if check {
        changed.push(path.to_path_buf());
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// Every card the registry should hold: the acceptance decks (the architecture
/// proof, which says exactly what it says) plus `data/card-pool.txt` (a card
/// implemented for its own sake). Writes the stubs, the module list, the
/// `CardIndex` ledger and the registry tables.
fn cards(
    root: &Path,
    check: bool,
    agent: &ureq::Agent,
    cache: &Path,
    cats: &catalog::SubtypeCatalogs,
    scripts: Option<&scriptgen::ScriptLookup>,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    let from_decks = acceptance::unique_names(&rows).len();
    println!(
        "card pool: {} cards ({from_decks} from the acceptance decks, {} from the pool file)",
        names.len(),
        names.len() - from_decks
    );
    // Indices come from the ledger, never from a card's position in this list:
    // the list is alphabetical, so one new card would otherwise renumber every
    // card after it (see baylee-cards-codegen/src/ledger.rs).
    // Read from the compiled table, which means this run sees the ledger as
    // of the last build. That is the right way round: `cargo run` rebuilds
    // before it runs, and a card assigned since would fail the `no row` check
    // below rather than be given an index here.
    let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS)?;
    // The taxonomy `cards/` is arranged by is computed per card
    // (`layout::path_for`), so a card's file can be somewhere else than where
    // it belongs — after a type-line correction, or on the run that
    // introduced the layout. Placement is therefore a reconciliation, not a
    // write: find every card file first, move the misplaced ones, and only
    // then decide what to write.
    let cards_dir = root.join("crates/baylee-cards/src/cards");
    let cycles = layout::LandCycles::parse(
        &fs::read_to_string(root.join("data/land-cycles.tsv")).unwrap_or_default(),
    );
    let mut found = card_files(&cards_dir)?;

    // Refuse an orphan *before* the first rename, not after the last one. The
    // slug a card claims is known without fetching anything, and a bail in
    // the middle of a re-filing would leave every card moved and `mod.rs`
    // still pointing at the old paths — a tree that does not build, for a
    // stray file someone could have deleted in a second.
    let claimed: BTreeSet<String> = names.iter().map(|n| front_face_slug(n)).collect();
    refuse_orphans(
        found
            .iter()
            .filter(|(slug, _)| !claimed.contains(*slug))
            .map(|(_, path)| path),
        &cards_dir,
    )?;
    refuse_stale_cycles(&cycles, &claimed)?;

    // Fill a cold cache in one download before asking for anything card by
    // card. `fetch_named` below reads a cached file with no request at all, so
    // this decides whether the next loop makes 1365 requests or none — and on
    // a hosted runner, where an IP is shared, 1365 requests is where the rate
    // limiter's 60-second backoffs come from. It writes what it can and
    // returns; whatever the feed did not carry is fetched one at a time below,
    // which is what makes the pair self-healing.
    scryfall::fill_from_bulk(&names, agent, cache);

    let mut stubs = Vec::with_capacity(names.len());
    for name in &names {
        let card = scryfall::fetch_named(name, agent, cache)?;
        let oracle_id = card.oracle_id.clone().unwrap_or_default();
        // Codegen reads the ledger and never writes it. Assignment is its own
        // deliberate step (`cargo xtask ledger`) over the whole card corpus,
        // because an index assigned as a side effect of a build is an index
        // whose order depends on what happened to be fetched that day.
        //
        // The whole row rather than the number: a card file names its index
        // (`index = index::TAIGA`), and the constant is frozen in the ledger
        // precisely so nothing re-derives it from a name Scryfall may rename.
        let Some(row) = ledger.entry_of(&oracle_id) else {
            anyhow::bail!(
                "{name} ({oracle_id}) has no row in the CardIndex ledger.\n\
                 Indices are assigned over the whole card corpus and not on \
                 sight: run `baylee-catalog corpus` and then `cargo xtask ledger`."
            );
        };
        let (info, content) = stubgen::render_stub(&card, row, &ledger, cats, scripts, &cycles)?;
        let stub_path = cards_dir.join(&info.path);
        // A card that already exists somewhere else is *moved*, never
        // rewritten at the new path and left behind at the old one — an
        // orphan there would still compile, be declared by nothing, and be
        // read by nobody.
        if let Some(current) = found.remove(&info.slug)
            && current != stub_path
        {
            if check {
                changed.push(current.clone());
            } else {
                if let Some(parent) = stub_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&current, &stub_path)?;
                println!(
                    "moved {} -> {}",
                    current
                        .strip_prefix(&cards_dir)
                        .unwrap_or(&current)
                        .display(),
                    info.path
                );
            }
        }
        // Ownership decides, and the file says which it is. A stub and a card
        // a reader wrote in full are both **machine-owned**: codegen rewrites
        // them, so a card the transcoder got wrong is fixed in the reader and
        // every card the fix reaches is corrected at once, rather than one
        // file being patched while the rule that wrote it stays wrong. A card
        // a person finished — or one `xtask adopt` took off the machine —
        // carries neither marker and is never written here. Moving a file is
        // not writing it: where a card sits is codegen's to say either way.
        let hand_owned = fs::read_to_string(&stub_path)
            .is_ok_and(|existing| !stubgen::is_machine_owned(&existing));
        if hand_owned {
            if check {
                println!("skip (hand-owned): {}", info.slug);
            }
            stubs.push(info);
            continue;
        }
        write_or_check(check, &stub_path, &content, changed)?;
        stubs.push(info);
    }

    // The same refusal once more, now that every slug is the one the reader
    // actually produced rather than the one `front_face_slug` predicted. The
    // early check above is what keeps a re-filing atomic; this one is what
    // makes the guarantee true.
    refuse_orphans(found.values(), &cards_dir)?;
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/cards/mod.rs"),
        &stubgen::render_cards_mod(&stubs),
        changed,
    )?;
    // The ledger is an input here, not an output: it is written by
    // `cargo xtask ledger` alone. Rendering it back would be a no-op on a
    // good tree and a silent repair on a hand-edited one.
    let slots = ledger.slots();
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated.rs"),
        &stubgen::render_registry(&stubs, slots),
        changed,
    )?;
    write_index_tree(root, check, &ledger, changed)?;
    Ok(())
}

/// Writes `crates/baylee-core/src/generated/index/` — the ledger as Rust.
///
/// A *rendering*, exactly like `generated.rs`: `cargo xtask ledger` assigns
/// and this only draws what was assigned, which is what keeps one writer and
/// makes `codegen --check` the thing that holds the two in step.
///
/// A set file the ledger no longer names is removed rather than left behind.
/// An orphan there would compile, be declared by nothing in `mod.rs` and be
/// read by nobody — the same failure an orphaned card file is, and it earns
/// the same answer.
fn write_index_tree(
    root: &Path,
    check: bool,
    ledger: &ledger::IndexLedger,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let dir = root.join("crates/baylee-core/src/generated/index");
    let files = cardindex::render(ledger)?;
    let mut expected: BTreeSet<&str> = BTreeSet::new();
    for file in &files {
        expected.insert(file.name.as_str());
        let path = dir.join(&file.name);
        // `mod.rs` is the one file here rustfmt has an opinion about: it
        // reorders `mod` and `pub use` lists by *version* sort, where `40k`
        // sorts after `5dn` because 40 is more than 5. Rendering them in any
        // other order and writing them verbatim makes `cargo fmt --all` and
        // `codegen --check` undo each other forever, which is how this was
        // found. The set files are constants and rustfmt has nothing to say
        // about them, so they skip the process spawn — 382 of those apiece,
        // on every run, to change nothing.
        if file.name == "mod.rs" {
            write_or_check(check, &path, &file.content, changed)?;
        } else {
            write_verbatim(check, &path, &file.content, changed)?;
        }
    }
    if dir.exists() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "rs")
                && !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| expected.contains(n))
            {
                if check {
                    changed.push(path.clone());
                } else {
                    fs::remove_file(&path)?;
                    println!("removed {}", path.display());
                }
            }
        }
    }
    println!(
        "card index: {} constants across {} set modules",
        ledger.entries().len(),
        files.len() - 1
    );
    Ok(())
}

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
fn scripts_root(root: &Path, given: &Path) -> PathBuf {
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
const SCRIPTS_ENV: &str = "BAYLEE_CARD_SCRIPTS";

/// The first directory named `cardsfolder` within `depth` levels of `at`.
///
/// Bounded and breadth-first on purpose: the corpus is a sibling checkout a
/// level or two away, and an unbounded walk from the parent of a repository
/// is a walk of somebody's whole home directory.
fn find_cardsfolder(at: &Path, depth: usize) -> Option<PathBuf> {
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

fn codegen(root: &Path, check: bool, scripts_dir: &Path, cache: &Path) -> anyhow::Result<()> {
    let cache = root.join(cache);
    let agent = ureq::Agent::new_with_defaults();
    let mut changed = Vec::new();

    // 1. Subtype catalogs → generated subtypes.rs.
    let mut cats = catalog::SubtypeCatalogs {
        creature: scryfall::fetch_catalog("creature-types", &agent, &cache)?,
        artifact: scryfall::fetch_catalog("artifact-types", &agent, &cache)?,
        enchantment: scryfall::fetch_catalog("enchantment-types", &agent, &cache)?,
        land: scryfall::fetch_catalog("land-types", &agent, &cache)?,
        planeswalker: scryfall::fetch_catalog("planeswalker-types", &agent, &cache)?,
        spell: scryfall::fetch_catalog("spell-types", &agent, &cache)?,
    };
    cats.normalize();
    write_or_check(
        check,
        &root.join("crates/baylee-core/src/generated/subtypes.rs"),
        &catalog::render_subtypes_rs(&cats),
        &mut changed,
    )?;

    // 2. card-script reference index. Built before the stubs, because a stub is
    //    transcoded from the rules reference when one is checked out locally
    //    (read as an automated lookup, never copied).
    let scripts_dir = scripts_root(root, scripts_dir);
    let lookup = if scripts_dir.exists() {
        let index = scripts::build_index(&scripts_dir)?;
        write_or_check(
            check,
            &root.join("data/script-index.json"),
            &serde_json::to_string_pretty(&index)?,
            &mut changed,
        )?;
        println!("script index: {} scripts", index.len());
        Some(scriptgen::ScriptLookup::new(scripts_dir.clone(), index))
    } else {
        println!(
            "note: card-script reference not found at {}, skipping index",
            scripts_dir.display()
        );
        None
    };

    // 3. The card pool → per-card stubs + registry.
    cards(
        root,
        check,
        &agent,
        &cache,
        &cats,
        lookup.as_ref(),
        &mut changed,
    )?;

    // 4. Which printed sentence each ability came from → generated_lines.rs.
    //    Written here rather than beside the registry because it is built
    //    from a *different* pair of sources: the registry comes from the
    //    ledger and the files on disk, this comes from the **compiled**
    //    pool read against the cached printings. One renderer over two
    //    sources makes a `--check` failure unreadable.
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated_lines.rs"),
        &render_ability_lines(root)?,
        &mut changed,
    )?;

    // 5. Which card a printed English name is → generated_names.rs.
    //    Beside stage 4 and not beside the registry, for its reason: this
    //    is built from the **compiled** pool, so it is two-phase in the
    //    same way and `--check` is what makes the second run a build
    //    failure rather than a name that silently resolves to nothing.
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated_names.rs"),
        &render_name_table()?,
        &mut changed,
    )?;

    // 6. Which id every token there is was assigned → generated_tokens.rs.
    //    Last, because it is the one generated file that reads *itself*
    //    back: the ids it has already given out are the ids it must give
    //    out again, and the only thing a run may do to the table is append.
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated_tokens.rs"),
        &render_token_ledger(root)?,
        &mut changed,
    )?;

    if check {
        if changed.is_empty() {
            println!("codegen check: up to date");
            return Ok(());
        }
        for p in &changed {
            eprintln!("stale: {}", p.display());
        }
        anyhow::bail!(
            "{} generated file(s) are stale; run `cargo xtask codegen`",
            changed.len()
        );
    }
    println!("codegen complete");
    Ok(())
}

fn exemplar_for(type_line: &str) -> &'static str {
    if type_line.contains("Planeswalker") {
        return "jace_the_mind_sculptor";
    }
    if type_line.contains("Creature") {
        return "ondu_cleric";
    }
    if type_line.contains("Instant") {
        return "force_of_will";
    }
    if type_line.contains("Sorcery") {
        return "demonic_tutor";
    }
    if type_line.contains("Enchantment") {
        return "rhystic_study";
    }
    if type_line.contains("Artifact") {
        return "sol_ring";
    }
    "polluted_delta"
}

/// Builds per-card task packages (stub + reference script + exemplar + prompt).
fn card_batch(
    root: &Path,
    cards: Option<&str>,
    out: &Path,
    scripts_dir: &Path,
    cache: &Path,
) -> anyhow::Result<()> {
    let agent = ureq::Agent::new_with_defaults();
    let cache = root.join(cache);
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    // The same set `codegen` writes and `validate` checks. Reading only the
    // acceptance decks here is the bug `validate` already had: every card
    // added for its own sake was generated and then never offered to a
    // batch, which is most of the pool.
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    let script_index: BTreeMap<String, String> = serde_json::from_str(
        &fs::read_to_string(root.join("data/script-index.json")).unwrap_or_default(),
    )?;
    // One walk of the tree rather than one per card: `cards/` is a taxonomy
    // now, and a card is found by its slug wherever it has been filed.
    let files = card_files(&root.join("crates/baylee-cards/src/cards"))?;
    let wanted: Vec<String> = if let Some(list) = cards {
        list.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        names
            .iter()
            .filter(|name| {
                let Some(path) = files.get(&front_face_slug(name)) else {
                    return false;
                };
                // `// GENERATED STUB` and not `Coverage::Unimplemented`. A
                // stub does not write that line at all — `CardDef::DEFAULT`
                // is already `Unimplemented`, and restating a default is the
                // one thing the card DSL forbids outright. Filtering on it
                // matched nothing in the whole pool, so this command's
                // default selection silently prepared zero packages.
                fs::read_to_string(path).is_ok_and(|c| c.contains("// GENERATED STUB"))
            })
            .cloned()
            .collect()
    };
    println!(
        "preparing {} card task package(s) in {}",
        wanted.len(),
        out.display()
    );
    for name in &wanted {
        let slug = front_face_slug(name);
        let dir = out.join(&slug);
        fs::create_dir_all(&dir)?;
        // 1. Current stub.
        let stub_path = files
            .get(&slug)
            .ok_or_else(|| anyhow::anyhow!("no card file for {slug}"))?;
        let stub = fs::read_to_string(stub_path)?;
        fs::write(dir.join("STUB.rs"), &stub)?;
        // The package tells its reader which file to edit, so it has to name
        // the real one: a slug says nothing about where the taxonomy put it.
        let rel = stub_path
            .strip_prefix(root)
            .unwrap_or(stub_path)
            .display()
            .to_string();
        // 2. Reference script (ground truth).
        let mut has_script = false;
        if let Some(rel) = script_index.get(name) {
            let script = scripts_root(root, scripts_dir).join(rel);
            if script.exists() {
                fs::write(dir.join("SCRIPT.txt"), fs::read_to_string(script)?)?;
                has_script = true;
            }
        }
        // 3. Scryfall JSON (metadata).
        let card = scryfall::fetch_named(name, &agent, &cache)?;
        fs::write(
            dir.join("SCRYFALL.json"),
            serde_json::to_string_pretty(&card)?,
        )?;
        // 4. Exemplar by type.
        let type_line = card.type_line.as_deref().unwrap_or("");
        let exemplar = exemplar_for(type_line);
        if let Some(exemplar_path) = files.get(exemplar) {
            fs::write(dir.join("EXEMPLAR.rs"), fs::read_to_string(exemplar_path)?)?;
        }
        // 5. Prompt.
        fs::write(
            dir.join("PROMPT.md"),
            card_prompt(name, &rel, &dir, has_script),
        )?;
    }
    Ok(())
}

/// The task text a batched agent is handed for one card.
///
/// Written for an agent working *in the repository* (it has file and shell
/// tools and reads the package itself), not for one being handed pasted
/// text: `SCRYFALL.json` alone would dominate the budget, and most of it is
/// printing metadata the card does not care about.
fn card_prompt(name: &str, file: &str, package: &Path, has_script: bool) -> String {
    let prompt = format!(
        "# Implement `{name}` in this repository\n\n\
         Edit exactly one file: `{file}`.\n\
         Touch nothing else — not `src/generated.rs`, not `src/cards/mod.rs`,\n\
         not another card, not the DSL.\n\n\
         Read first, in this order:\n\
         - `crates/baylee-cards/AGENTS.md` — the playbook you are bound by.\n\
         - `docs/card-dsl.md` — the authoring contract and the full vocabulary.\n\
         {script_line}\
         - `{package}/EXEMPLAR.rs` — an implemented card of the same type; match its style.\n\
         - `{package}/SCRYFALL.json` — metadata, if you need the printed details.\n\n\
         Hard rules:\n\
         1. Every `//!` line at the top of the file, `index`, `oracle_id`,\n\
            `scryfall_id` and the `faces` literals are generated facts. Do\n\
            not edit, add or delete a single one of them. That header is\n\
            the human-verification surface: `xtask validate` compares it\n\
            against the card you build and fails on any drift. You may edit\n\
            only `coverage`, `keywords` and `abilities`.\n\
         2. Never restate a default. The macros in `baylee-cards-dsl/src/build.rs`\n\
            supply them, and the defaults are *rules* defaults.\n\
         3. Do not invent `Effect`, `Modifier` or `Filter` variants. If the\n\
            DSL cannot say what the card says, STOP and refuse — see below.\n\
         4. Every oracle sentence is implemented, or the card is refused. A\n\
            card that is nearly right is worse than a stub: the deckbuilder\n\
            offers implemented cards as playable.\n\
         5. The only command you may run is\n\
            `cargo check --all-targets -p baylee-cards` (`--all-targets` so\n\
            a test you wrote is compiled too), and only to find out whether\n\
            your own edit compiles. Do not run\n\
            `cargo test`, `cargo clippy`, `cargo fmt`, `cargo run` or any\n\
            `xtask` command. The harness around you runs the full gate on\n\
            this card the moment you finish and reverts the file if it\n\
            fails, so running any of that here buys nothing and costs more\n\
            time than writing the card does.\n\n\
         Refusing is a correct outcome, not a failure. If any clause is\n\
         inexpressible, revert your edits to that file so it stays the\n\
         generated stub, and report `status: \"refused\"`.\n\n\
         When you report a refusal, `cannot_say` must name **what the DSL\n\
         cannot express**, not which mechanic you think is missing, and\n\
         `nearest_existing` must name the closest variant that does exist.\n\
         Those two together are the whole value of a refusal: the last time\n\
         a blocker was read as a missing subsystem, the subsystem was\n\
         already there and one variant that could say \"the target\" was all\n\
         it needed.\n",
        package = package.display(),
        // Named only when it is there. Roughly one card in eight has no
        // script under its printed name, and pointing an agent at a file
        // that does not exist spends a turn and teaches it that the
        // package's promises are approximate.
        script_line = if has_script {
            format!(
                "- `{}/SCRIPT.txt` — the card-script reference script; rules ground truth.\n",
                package.display()
            )
        } else {
            "There is no card-script reference script for this card. The oracle text in \
             the stub header is all the ground truth there is; if that leaves a \
             clause genuinely ambiguous, refuse rather than guess.\n"
                .to_string()
        },
    );
    prompt
}

/// Ranks the land text `landgen` cannot read, by the line that stopped it.
///
/// Every card in the pool that is still a generated stub is a land, so this is
/// not a corner of the worklist — it *is* the worklist. Land text is
/// formulaic, which is what makes a ranking worth having: a sentence shape
/// taught to `landgen` is not one card but every land that prints that shape,
/// generated with nobody reading the result. That is a different economy from
/// asking a model to write hundreds of files a person then has to check.
///
/// Each line is normalised before it is counted. The numbers, mana symbols and
/// proper nouns in it are exactly what makes two printings of one shape look
/// like two problems, and a ranking that counted them apart would put a shape
/// printed on sixty cards below one printed on three.
fn land_report(
    root: &Path,
    cache: &Path,
    samples: usize,
    worklist: Option<&Path>,
    tail: usize,
) -> anyhow::Result<()> {
    let agent = ureq::Agent::new_with_defaults();
    let cache = root.join(cache);
    let mut cats = catalog::SubtypeCatalogs {
        creature: scryfall::fetch_catalog("creature-types", &agent, &cache)?,
        artifact: scryfall::fetch_catalog("artifact-types", &agent, &cache)?,
        enchantment: scryfall::fetch_catalog("enchantment-types", &agent, &cache)?,
        land: scryfall::fetch_catalog("land-types", &agent, &cache)?,
        planeswalker: scryfall::fetch_catalog("planeswalker-types", &agent, &cache)?,
        spell: scryfall::fetch_catalog("spell-types", &agent, &cache)?,
    };
    cats.normalize();
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);

    let (mut readable, mut nothing) = (0usize, 0usize);
    // Kept whole rather than counted, because the worklist below is the other
    // half of the same pass: a shape's cards *are* the answer to "who should
    // write these", and counting them would throw that away.
    let mut not_a_land: Vec<String> = Vec::new();
    let mut shapes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let files = card_files(&root.join("crates/baylee-cards/src/cards"))?;
    for name in &names {
        // A finished card is nobody's worklist, whoever finished it — and a
        // card with no file at all is not one either.
        let is_stub = files
            .get(&front_face_slug(name))
            .and_then(|p| fs::read_to_string(p).ok())
            .is_some_and(|c| c.contains("// GENERATED STUB"));
        if !is_stub {
            continue;
        }
        let card = scryfall::fetch_named(name, &agent, &cache)?;
        match landgen::read(&card, &cats) {
            // A stub `landgen` can read is a codegen run away from being a
            // card, so it belongs in neither ranking. Seeing one at all would
            // mean the generated files are stale.
            Ok(_) => readable += 1,
            Err(landgen::LandRefusal::NotAPlainLand) => not_a_land.push(name.clone()),
            Err(landgen::LandRefusal::NothingToSay) => nothing += 1,
            Err(landgen::LandRefusal::UnreadLine(line)) => {
                shapes
                    .entry(shape_of(&line))
                    .or_default()
                    .push(name.clone());
            }
        }
    }

    let blocked: usize = shapes.values().map(Vec::len).sum();
    println!(
        "land-report: {blocked} land(s) blocked on {} sentence shape(s); {} not plain \
         lands, {nothing} with nothing to say, {readable} readable already",
        shapes.len(),
        not_a_land.len()
    );
    let mut ranked: Vec<_> = shapes.into_iter().collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    let mut running = 0usize;
    for (shape, cards) in &ranked {
        running += cards.len();
        // The running total is the point of the ranking: it says how many
        // shapes have to be read before a given share of the pool is finished.
        println!("{:5} {running:5}  {shape}", cards.len());
        if samples > 0 {
            let examples: Vec<&str> = cards.iter().take(samples).map(String::as_str).collect();
            println!("             e.g. {}", examples.join(", "));
        }
    }

    if let Some(path) = worklist {
        let mut cards: Vec<&String> = not_a_land.iter().collect();
        cards.extend(
            ranked
                .iter()
                .filter(|(_, printed)| printed.len() < tail)
                .flat_map(|(_, printed)| printed.iter()),
        );
        cards.sort_unstable();
        // One name per line, and `card-batch --cards` takes them
        // comma-separated — `paste -sd,` is the one step in between, which is
        // better than writing a line this long into a file nobody can read.
        let mut text = cards.iter().fold(String::new(), |mut acc, name| {
            acc.push_str(name);
            acc.push('\n');
            acc
        });
        text.shrink_to_fit();
        fs::write(path, text)?;
        println!(
            "\nworklist: {} card(s) written to {} — every land on a shape printed \
             fewer than {tail} time(s), plus the ones that are not plain lands",
            cards.len(),
            path.display()
        );
    }
    Ok(())
}

/// Reduces one printed line to the shape it is an instance of.
///
/// "{T}: Add {G}" and "{T}: Add {W}" are one problem. What differs between two
/// instances of a shape is its mana symbols, its numbers and its proper nouns;
/// what is left after those are struck out is the grammar.
fn shape_of(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                // A mana symbol, a tap, a number in braces — all one token.
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                }
                out.push_str("{_}");
            }
            '0'..='9' => {
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    chars.next();
                }
                out.push('#');
            }
            c if c.is_uppercase() => {
                // A proper noun — a land type, a card name — collapses to one
                // token, so "Plains or Island" and "Swamp or Mountain" are the
                // same sentence. A sentence's first word is capitalised too and
                // collapses with them; that costs nothing, because a shape is
                // only ever compared with another shape.
                out.push('_');
                while chars.peek().is_some_and(|n| n.is_lowercase() || *n == '\'') {
                    chars.next();
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// A card's file stem.
///
/// Multi-face cards are filed under their front face, the way `codegen` slugs
/// them: "Zof Consumption // Zof Bloodbog" is one file called
/// `zof_consumption`. Slugging the whole printed name instead produces a path
/// that does not exist, so the card is read as implemented and skipped.
/// Refuses every card file under `cards/` that no card in the pool claims.
///
/// Such a file compiles, `cargo test` is green and nothing reads it, which is
/// exactly how an empty `lightning_bolt.rs` sat in the tree unnoticed. It is
/// asked twice: once before the first rename, on the slugs the pool's names
/// predict, so that a re-filing is atomic rather than half-applied over a
/// stray somebody could have deleted in a second; and once after the last
/// one, on the slugs the reader actually produced.
fn refuse_orphans<'a>(
    strays: impl Iterator<Item = &'a PathBuf>,
    cards_dir: &Path,
) -> anyhow::Result<()> {
    let strays: Vec<String> = strays.map(|p| relative(p, cards_dir)).collect();
    if strays.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "{} card file(s) under cards/ that no card claims: {}",
        strays.len(),
        strays.join(", ")
    )
}

/// Refuses a cycle-map entry naming a card the pool does not have.
///
/// The failure it catches is silent by construction: the map is additive, so
/// a name that matches nothing simply leaves its land filed one level
/// shallower. Ten pathways sat unfiled for a whole commit that way, the map
/// holding front-face names while Scryfall hands over `A // B`.
fn refuse_stale_cycles(
    cycles: &layout::LandCycles,
    claimed: &BTreeSet<String>,
) -> anyhow::Result<()> {
    let stale: Vec<&str> = cycles
        .names()
        .filter(|n| !claimed.contains(&front_face_slug(n)))
        .collect();
    if stale.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "data/land-cycles.tsv names {} card(s) the pool does not have: {}",
        stale.len(),
        stale.join(", ")
    )
}

/// A card file's path as it reads in a message: relative to `cards/`.
fn relative(path: &Path, cards_dir: &Path) -> String {
    path.strip_prefix(cards_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn front_face_slug(name: &str) -> String {
    baylee_cards_codegen::stubgen::slug(name.split(" // ").next().unwrap_or(name))
}

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
fn cached_printing(root: &Path, name: &str) -> Option<serde_json::Value> {
    let path = root.join("data/scryfall-cache").join(format!(
        "{}.json",
        baylee_cards_codegen::stubgen::slug(name)
    ));
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// Extracts the first `"`-quoted value after `key` (e.g. `name = "…"`).
/// What a card file writes after `<field> =`, with the line break rustfmt
/// may have put in between skipped.
///
/// Card files are ordinary rustfmt output since the macros moved to
/// parentheses, which means a value too long for its line is wrapped onto the
/// next one: Nesting Dovehawk's `Coverage::Partial("…")` is written
/// `coverage =\n        Coverage::Partial(…)`. A reader matching the literal
/// `"coverage = Coverage::"` read that as a card with no coverage flag at all
/// — the seventh textual reader of the pool to answer a question it could not
/// see the answer to. Anything asking a card file what a knob says goes
/// through here.
fn knob<'a>(content: &'a str, field: &str) -> Option<&'a str> {
    let at = content.find(&format!("{field} ="))?;
    Some(content[at + field.len() + 2..].trim_start())
}

fn quoted_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let start = content.find(key)? + key.len();
    let rest = &content[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// All `mana_cost` literals in the file (one per face), normalized:
/// `{0}` and `ManaCost::ZERO` are the same thing.
fn code_costs(content: &str) -> Vec<String> {
    content.split("face!(").skip(1).map(face_cost).collect()
}

/// The `mana_cost` one `face!` block writes, or `(no cost)` for a face that
/// writes none.
///
/// A costless face writes no `mana_cost` line at all — `FaceDef::DEFAULT`
/// supplies `ManaCost::ZERO` and the authoring rule is never to restate a
/// default — so [`code_costs`] has to be **positional** rather than a
/// scrape of the written lines. It was a scrape with one "(no cost)"
/// appended when the counts disagreed, which put the entry on the wrong
/// face: Ishgard, the Holy See is a Town on the front of an Adventure, so
/// the first cost the file writes belongs to the *second* face, and the
/// checker reported that the printing costs nothing while the code costs
/// {3}{W}{W}. Five cards said it, all of them a land or a town in front of
/// a spell — and none of them said it until the payload lookup started
/// finding double-faced cards at all.
///
/// A block reaches to the start of the next one, so the last block reaches
/// the end of the file and would pick up a `mana_cost:` written after the
/// `faces` list closes. Measured 2026-09-09 over the 109 multi-faced files:
/// none writes one there, and both callers would be blind to it if one did —
/// the header check compares sets and the printing check reads the front
/// face.
fn face_cost(face: &str) -> String {
    const NONE: &str = "(no cost)";
    let Some(pos) = face.find("mana_cost = ") else {
        return NONE.to_string();
    };
    let rest = &face[pos + "mana_cost = ".len()..];
    // Either spelling of the macro. It used to read only the qualified one,
    // and the day `mana!` reached the prelude all 478 costed faces in the
    // pool read as costing nothing — a reader pinned to one spelling that
    // answers a *value* when it cannot read, rather than saying so.
    let rest = rest
        .strip_prefix("mana!(\"")
        .or_else(|| rest.strip_prefix("baylee_core::mana!(\""));
    let Some(rest) = rest else {
        // A `mana_cost:` written some third way is this function going
        // blind, not a face with no cost. Callers compare against the
        // printing, so a sentinel here would read as a disagreement about
        // the card.
        return "(unreadable)".to_string();
    };
    match rest.find('"') {
        Some(end) if &rest[..end] != "{0}" => rest[..end].to_string(),
        _ => NONE.to_string(),
    }
}

/// Compares the mandatory human-readable header against the `CardDef`
/// data — the header is the safety net against generation drift, and it
/// is only a net if something checks it (two cost fixes once shipped
/// with stale headers).
/// Compares the code's first face against the **printing** — Scryfall's own
/// payload in `data/scryfall-cache`, which is what `codegen` reads.
///
/// [`check_header_matches_code`] compares two things a person wrote, so a
/// card whose header was edited to agree with a wrong `CardDef` passes it
/// with nothing to say. Fifteen cards in the pool had done exactly that: six
/// wrong mana costs and nine wrong power/toughness pairs, every one of them
/// green. Wartime Protestors was printed `{3}{R}` 4/4 and played `{2}{R}`
/// 3/2, which is how it was reported — a card that costs one mana less than
/// the one drawn on it.
///
/// The numbers on the front face are what this one checks: a cost is what a
/// player pays, a P/T is what survives combat, and a starting loyalty is what
/// a planeswalker has to lose before it dies — all single values a hand edit
/// can silently move. [`check_card_matches_the_printing`] is the other half,
/// asked of the *compiled* card rather than of its source text, because
/// colors and keywords are sets rather than literals a `find` can lift out of
/// a file.
///
/// A card with no cached payload is skipped rather than failed, because
/// failing on its absence would turn a data-fetching problem into a build
/// failure about card rules. It now reaches all 1365 — it reached 1263 until
/// [`cached_printing`] learned the name a double-faced card is filed under —
/// and [`PrintingTally`]'s floors are what say the reach has not shrunk
/// again.
fn check_code_matches_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // A double-faced card carries its per-face costs and P/T inside
    // `card_faces`, and its top-level `mana_cost` is the two sides joined
    // with " // " — a string no `CardDef` face ever holds.
    let front = payload.get("card_faces").and_then(|f| f.get(0));
    let printed = |key: &str| -> Option<String> {
        front
            .and_then(|f| f.get(key))
            .or_else(|| payload.get(key))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    };

    let Some(face) = content.find("faces = &[").map(|pos| &content[pos..]) else {
        return;
    };
    // Every face the printing puts a cost on, not the front one alone: an
    // MDFC's back side is a card you cast for its own printed cost, and
    // nothing here read it.
    //
    // Two tolerances, and both are about what a face's cost *means* rather
    // than about slack. The lists are compared only when they are the same
    // length, because a differing count is a modelling decision rather than
    // a typo — a split card and an adventure print two faces Scryfall's way
    // and are one `FaceDef` here. And a back face the printing gives no cost
    // is skipped **only when the code calls it a disturb face**, because
    // that is the one reason a `FaceDef` carries a cost the printing does
    // not: Ghastly Mimicry's `mana_cost` is its **disturb** cost, which
    // Scryfall writes in the oracle text and not in
    // `card_faces[1].mana_cost`. That is why this check would not have found
    // the `{5}{U}` it was built at — [`check_oracle_matches_the_printing`]
    // did, by comparing the sentence.
    //
    // The `disturb` half of that sentence was missing, and one card walked
    // through the gap. The True Scriptures is a *transformed* back — reached
    // by Sheoldred's `{4}{B}` ability, cast for nothing ever — and it was
    // written with a `{2}{B}{B}` no printing has. Nothing compared it,
    // because the tolerance covered every costless back rather than the
    // faces that earn it; and `a_back_face_with_no_printed_cost_is_never_
    // castable_from_the_hand` reads the *code's* cost, so an invented one is
    // exactly what makes a transformed back look like an MDFC's to it. The
    // cast wizard duly offered the Saga out of hand for five mana. A skipped
    // comparison is how a hand-written number gets to mean whatever it says.
    //
    // `tally.costs` is what keeps either tolerance from quietly swallowing
    // the whole check.
    let printed_costs: Vec<String> = match payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        Some(faces) => faces
            .iter()
            .map(|f| {
                f.get("mana_cost")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            })
            .collect(),
        None => vec![
            payload
                .get("mana_cost")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ],
    };
    let code_costs = code_costs(face);
    // Which of those code faces says `disturb: true`, positionally — the
    // tolerance below is for that field and nothing else. Blocks run to the
    // next `face!(` and the last to the end of the file, the same reach
    // [`face_cost`] documents.
    let code_disturb: Vec<bool> = face
        .split("face!(")
        .skip(1)
        .map(|block| block.contains("disturb = true"))
        .collect();
    if printed_costs.len() == code_costs.len() {
        for (at, (printed, code)) in printed_costs.iter().zip(&code_costs).enumerate() {
            if at > 0 && printed.is_empty() && code_disturb.get(at).copied().unwrap_or(false) {
                continue;
            }
            tally.costs += 1;
            if normalise_cost(printed) != *code {
                println!(
                    "{slug}: face {at} costs {printed} in the printing and {code} in the code"
                );
                *problems += 1;
            }
        }
    }
    // A creature only, and for loyalty a planeswalker only. `power` on a
    // printing with no P/T is absent, and a face that writes neither is a
    // noncreature card the check has nothing to say about. A transforming
    // planeswalker whose *front* face has no loyalty is skipped the same
    // way: the payload's front face carries `null` there, so nothing is
    // compared against the back face's number.
    for (key, field) in [
        ("power", "power = Some("),
        ("toughness", "toughness = Some("),
        ("loyalty", "loyalty = Some("),
    ] {
        let (Some(printed), Some(code)) = (printed(key), field_number(face, field)) else {
            continue;
        };
        if printed != code {
            println!("{slug}: the printing has {key} {printed} and the code has {code}");
            *problems += 1;
        }
        if key == "loyalty" {
            tally.loyalty += 1;
        }
    }
}

/// The `//! Oracle:` header against the text actually printed on the card.
///
/// The header is the pool's human-verification surface, and every other
/// check reads *around* it: the cost, the P/T, the colors and the keywords
/// are compared against the printing, and the abilities are compared
/// against nothing at all. So a card whose rules text had been paraphrased,
/// abbreviated or overtaken by errata read as correct to a person and to
/// every gate, and the code underneath it was written from the wrong
/// sentence.
///
/// That is not hypothetical: the first run of this check found 145
/// disagreements. Ninety-seven were one codegen bug — Scryfall carries a
/// double-faced card's text per face and `stubgen` wrote the absent
/// top-level field, so every two-faced header was blank — and the other
/// forty-eight were hand-written headers, refreshed by
/// [`refresh_oracle`]. Two of those forty-eight were a *card* written from
/// its own wrong header: Volrath's Stronghold's second ability had lost its
/// `{1}{B}`, and Mirrorhall Mimic's disturb was built at `{5}{U}` against a
/// printed `{3}{U}{U}`.
///
/// Whitespace is the only thing forgiven, because a line's indentation
/// inside a doc comment is not a rules fact. Wording is not: "you may draw a
/// card unless that player pays {1}" and "you may have that player pay {1};
/// if they don't, you draw a card" are the same card and *not* the same
/// question, and which of the two is printed decides who is asked.
fn check_oracle_matches_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let header = content
        .lines()
        .filter_map(|l| l.strip_prefix("//! Oracle:"))
        .collect::<Vec<_>>()
        .join("\n");
    let printed = printed_text(payload);
    tally.oracle += 1;
    if squash(&header) == squash(&printed) {
        return;
    }
    println!("{slug}: the header's oracle text is not the printing's");
    for line in diff_lines(&header, &printed) {
        println!("    {line}");
    }
    *problems += 1;
}

/// The `//! Set:` line against the printing it claims to name.
///
/// `validate` checked that this line *existed* and never what it said, which
/// is the whole of its job: the Scryfall id in it is carried in three places
/// and agreed everywhere, so the set code, the collector number and the set
/// name beside that id were prose nobody read. Forty hand-owned cards named
/// the wrong printing — Sensei's Divining Top said "EMA #232 — Eternal
/// Masters" over an id that is Double Masters 2022 #314 — and a person
/// checking the card by eye would have looked up a different piece of
/// cardboard and found it agreed.
///
/// It compares the whole line rather than the fields, because the line has
/// one author ([`baylee_cards_codegen::stubgen::set_line`]) and `xtask
/// refresh-oracle` writes exactly what this reads.
fn check_set_line_matches_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let Some(want) = set_header_line(payload) else {
        return;
    };
    let want = want.trim_end();
    tally.printings += 1;
    let Some(have) = content.lines().find(|l| l.starts_with("//! Set:")) else {
        return; // the "set line" check above already reported the absence
    };
    if have == want {
        return;
    }
    println!("{slug}: the header names a printing its own Scryfall id does not");
    println!("    header  {have}");
    println!("    printing {want}");
    *problems += 1;
}

/// A printed "up to N target" is a **count**, and the code has to be able to
/// say it.
///
/// A bare `TargetSpec` reads as *exactly one*, which is a different card: an
/// ability whose only legal answer is "none" cannot be activated at all, so
/// the sentence after it never happens. Karn, the Great Creator's `+1` and
/// Teferi, Time Raveler's `-3` were both written that way, and Teferi's is
/// the one that shows what it costs — "Return up to one target artifact,
/// creature, or enchantment to its owner's hand. **Draw a card.**" was a
/// draw a player could not reach with an empty board.
///
/// The check is textual on purpose. It reads the printing's own sentence and
/// then asks whether the card's source says a minimum of none *anywhere* —
/// `TargetReq::up_to_one`, `up_to`, `x_targets` or a written-out `min: 0`.
/// That is coarse: a card with two targeted abilities where only one prints
/// "up to" passes on the other's count. It is still worth having, because the
/// failure it is written for is a card that says "up to" in its header and
/// nowhere in its code, and because the shapes that *cannot* say it —
/// `Activated`, `ActivatedConditional`, `SagaChapter`, whose `target` is a
/// bare spec — have no way to pass except by being reported.
fn check_target_counts_match_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // A stub claims nothing, so there is nothing to disagree with — and a
    // card marked `Partial` has already said, in writing, that it diverges
    // from its printing. That is the sanctioned answer for a sentence the
    // DSL cannot say (Sheoldred's chapter I destroys one permanent *per
    // opponent*, which is not a number `TargetReq` has), and a checker that
    // reported it anyway would be asking the pool to lie the other way.
    if content.contains(stubgen::STUB_MARKER) || content.contains("Coverage::Partial") {
        return;
    }
    let printed = printed_text(payload).to_lowercase();
    let Some(phrase) = up_to_target_phrase(&printed) else {
        return;
    };
    tally.targets += 1;
    if ["up_to_one(", "up_to(", "x_targets(", "min: 0"]
        .iter()
        .any(|way| content.contains(way))
    {
        return;
    }
    println!("{slug}: the printing says \"{phrase}\" and the code says exactly one");
    *problems += 1;
}

/// A card whose printing names a **player** as a target has to be able to
/// name one.
///
/// The fault this closes is the one the owner reported from a live game and
/// it has a shape: `PlayerRel::Opponent` written where the card says "target
/// player", which is `EachOpponent` in `eval::players` — so Halimar
/// Excavator milled every opponent at once, and Primaris Eliminator's
/// Hyperfrag Round shrank every creature on the table including its own
/// side. Both read correctly in a duel, which is why both survived every
/// other check in this file and a whole test suite: with one opponent,
/// "each of them" and "the one you chose" are the same seat.
///
/// It is decidable without a second reader, because the card prints it. The
/// printed text says "target player" or "target opponent"; the code either
/// names a player-shaped [`TargetSpec`] or it does not. There is no
/// interpretation in between.
///
/// Three rules keep it from inventing findings:
///
/// - **Reminder text is stripped**, through the [`strip_reminders`] that
///   `check_scope_matches_the_text` already reads with. It is about a
///   keyword, not about this card — the same reading `claim_tests` had to
///   learn — and "target player" inside a parenthesis is somebody else's
///   sentence.
/// - **A stub claims nothing**, and a `Partial` card has already said in
///   writing that it diverges. Both are skipped, exactly as the target-count
///   check above skips them.
/// - It reads the **printing**, not the `//! Oracle:` header, so a card whose
///   header drifted cannot hide the gap by agreeing with its own code.
fn check_player_targets_match_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    if content.contains(stubgen::STUB_MARKER) || content.contains("Coverage::Partial") {
        return;
    }
    let printed = printed_text(payload).to_lowercase();
    let bare = strip_reminders(&printed);
    let Some(phrase) = ["target player", "target opponent"]
        .into_iter()
        .find(|p| bare.contains(p))
    else {
        return;
    };
    tally.player_targets += 1;
    if [
        "TargetSpec::AnyPlayer",
        "TargetSpec::AnyOpponent",
        "TargetSpec::AnyTarget",
    ]
    .iter()
    .any(|spec| content.contains(spec))
    {
        return;
    }
    println!("{slug}: the printing says \"{phrase}\" and the code names no player to target");
    *problems += 1;
}

/// A card whose printing says "you may" has to hand the player the choice.
///
/// The fault this closes is the one the owner reported by name: Ondu Cleric
/// prints "you may gain life equal to the number of Allies you control" and
/// gained it every time, because the word was read as decoration. Four cards
/// were written that way, and each of them has a board where the automatic
/// answer is the wrong one — which is the whole reason the word is printed.
///
/// It is decidable without a second reader for [`check_player_targets_match_the_printing`]'s
/// reason: the card prints the word, and the code either carries a construct
/// that offers a choice or it does not. What it must **not** be is a list of
/// card names — this is a sweep over the pool, and a sweep that recognises
/// cards one at a time stops being one the moment a card is added.
///
/// So it reads the **compiled `CardDef`** and not the file, through the
/// `Debug` rendering `pool-dump` already writes. That is what makes it a
/// check about constructs: Restoration Angel writes its "may" as a literal
/// `TargetReq { min: 0, .. }` and Sun Titan writes the same thing as
/// `TargetReq::up_to_one`, and the compiled form of both says `min: 0`. A
/// grep over the source would have called one of them a bug.
///
/// The list is longer than `MayDo` because a printed "you may" is said
/// several different ways here, all of them already asking:
///
/// - "you may have **target** player lose life" — a target requirement whose
///   minimum is zero. Declining is choosing no target, which the target
///   prompt already offers (Hagra Diabolist, Sun Titan, Restoration Angel).
/// - "you may pay 2 life" as a land enters — `TappedOrPayLife`, and
///   `PayLifeOrEnterTapped` where the same choice is an effect.
/// - "you may pay `{N}`" during resolution — `PlayerMayPayOr`.
/// - "you may pay … rather than pay this spell's mana cost" — an
///   `AlternativeCost`; a kicker is an additional cost.
/// - "you may have this enter as a copy" — `CopyOnEnter`.
/// - "you may choose new targets for the copy" — the copy effects, which ask
///   on their own (`AwaitingOp::CopyNewTargets`).
/// - "you may choose a nonland card from it" — `BottomCardFromHand`, which
///   offers a choice of none (Vendilion Clique).
/// - "you may search your library" — an optional search.
/// - "you may put it on the bottom" — scry, a choice by construction.
/// - "you may play those cards … and you may spend mana as though" —
///   `SearchTakeover`. A permission is not a decision: playing a card is
///   optional already, and there is nothing to ask.
///
/// A stub claims nothing and a `Partial` card has said in writing that it
/// diverges, so both are skipped — the same two exemptions the checks above
/// take.
///
/// What it counts is what it can decide, and the two are not the same
/// thing. It asks whether the card carries an asking construct *anywhere*,
/// so a printing with two "may"s and one construct passes. A clean sweep
/// therefore says no card in the pool is may-blind; it does not say every
/// printed "may" is asked, and reading it as the second claim is how a
/// check like this stops finding anything.
fn check_optional_clauses_are_offered(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // Every construct that hands a choice to the player, as the compiled
    // definition spells it. `min: 0` is a target requirement's and nothing
    // else's — it is the one `min` field in the whole DSL.
    const OFFERS_A_CHOICE: &[&str] = &[
        "MayDo",
        "min: 0",
        "PayLifeOrEnterTapped",
        "PlayerMayPayOr",
        "CopyOnEnter",
        "CopyTargetSpell",
        "CopySpell",
        "BottomCardFromHand",
        "OptionalBasicLandSearchFor",
        "optional: true",
        "Scry",
        "PutFromHandOnTop",
        "ReorderTopLibrary",
        "SearchTakeover",
    ];
    if !def.is_implemented() {
        return;
    }
    let printed = printed_text(payload).to_lowercase();
    if !strip_reminders(&printed).contains("you may") {
        return;
    }
    tally.optional_clauses += 1;
    // A cost a face offers rather than demands is read from the field, not
    // from the rendering: "you may pay … rather than" and a kicker are the
    // *absence* of a requirement, and an empty list renders as one word.
    if def.faces.iter().any(|f| {
        !f.alternative_costs.is_empty()
            || !f.additional_costs.is_empty()
            || f.miracle.is_some()
            || !f.enter_modifiers.is_empty()
    }) {
        return;
    }
    // Everything else is a construct that appears in the rendering.
    let compiled = format!("{def:#?}");
    if OFFERS_A_CHOICE.iter().any(|c| compiled.contains(c)) {
        return;
    }
    println!("{slug}: the printing says \"you may\" and the code never asks");
    *problems += 1;
}

/// The printed phrase that states a target count of "up to", if there is one.
///
/// Read by scanning rather than by matching a list of whole phrases, because
/// what sits between the number and the noun varies with the card — "up to
/// one target creature", "up to two target **other** creatures", "up to X
/// target lands" — and a list of exact spellings would quietly stop matching
/// the first time a card worded it a new way.
///
/// The window stops at a line break or a bullet, which is the difference
/// between a count of *targets* and a count of *modes*. Ertai Resurrected
/// prints "choose up to one —" and then two bulleted modes that each target
/// something; the "up to one" there is about how many modes are chosen, and
/// the card says it with an empty third mode rather than with a `TargetReq`.
fn up_to_target_phrase(printed: &str) -> Option<String> {
    let mut from = 0;
    while let Some(at) = printed[from..].find("up to ") {
        let start = from + at;
        let rest = &printed[start..printed.len().min(start + 44)];
        let window = rest.split(['\n', '\u{2022}']).next().unwrap_or_default();
        if let Some(i) = window.find("target") {
            return Some(window[..i + "target".len()].to_string());
        }
        from = start + "up to ".len();
    }
    None
}

/// Every non-blank line, trimmed — the one difference this check forgives.
fn squash(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// The two texts side by side, as `-` header and `+` printing lines.
///
/// A whole-line set difference rather than a real diff: the header is at
/// most a dozen lines, and what a reader needs is which sentence to look at,
/// not an edit script.
fn diff_lines(header: &str, printed: &str) -> Vec<String> {
    let (a, b) = (squash(header), squash(printed));
    let mut out = Vec::new();
    for line in &a {
        if !b.contains(line) {
            out.push(format!("- {line}"));
        }
    }
    for line in &b {
        if !a.contains(line) {
            out.push(format!("+ {line}"));
        }
    }
    out
}

/// What the printing checks actually compared.
///
/// Every one of them skips quietly — no cached payload, no field in it, no
/// symbol in the text, no comparison — so the whole family can go silent
/// without a single line of output changing. These counts are what says it
/// did not.
#[derive(Default)]
struct PrintingTally {
    /// Cards with a cached payload at all.
    payloads: usize,
    /// Cards whose starting loyalty was compared.
    loyalty: usize,
    /// Cards whose color identity was compared.
    identity: usize,
    /// Cards claiming at least one keyword bit that has a printed spelling.
    keywords: usize,
    /// Cards whose mana abilities were read and held against the text.
    mana: usize,
    /// Cards whose Oracle header was held against the printed text.
    oracle: usize,
    /// Face costs compared against the printing, over the whole pool.
    costs: usize,
    /// Cards whose printing names a player or an opponent as a target.
    player_targets: usize,
    /// Cards whose printing states a target count of "up to".
    targets: usize,
    /// Cards whose printing says "you may" outside reminder text.
    optional_clauses: usize,
    /// Cards whose `Set:` header line was held against the printing.
    printings: usize,
    /// Faces whose type line was held against the printed one.
    type_lines: usize,
}

/// The floor under each count in [`PrintingTally`].
///
/// Absolute numbers rather than a fraction of whatever happened to be on
/// disk, because every run is meant to have the same payloads: the cache is
/// not tracked (it is somebody else's card data, `docs/legal.md` §3), so CI
/// restores it and `codegen` fetches what is missing, and a checkout that
/// ran neither has no honest reason to compare fewer cards. Each is the
/// measured number with slack for cards leaving the pool.
/// The shape is `baylee_cards::lints`' own — "the sweep is not reaching the
/// pool" — and it exists for the same reason: a checker that silently stops
/// checking reports a clean pool.
/// Measured 2026-09-09 over a pool of 1365 and re-measured 2026-09-11 over
/// the same 1365: **1365** payloads, 7 loyalty, 1365 identity, 49 keyword,
/// 379 mana (was 361), **1365** oracle, 1472 cost (was 1370). Payloads,
/// identity and oracle are now every card in the pool rather than the 1263
/// the lookup used to find, so the floor under them is a real bound and not a
/// record of a gap: nothing but a card leaving the pool can move it down.
///
/// Cost is **1472** and the arithmetic is exact, re-derived 2026-09-11 with
/// the subtype count below because the two read the same faces: the pool
/// prints 1475 (1255 cards of one face, 110 of two), Emeritus of Woe is the
/// one card Scryfall writes as two faces and this pool as one — an adventure
/// — so its two are skipped whole, and Mirrorhall Mimic's disturb back is the
/// single face whose printed cost is empty against a `FaceDef` that carries
/// the disturb cost. 1475 − 2 − 1 = 1472. The sentence here used to name
/// Twining Twins beside Emeritus and to reach 1370; the card has since been
/// re-modelled as the two faces it prints, and a stale explanation of a count
/// is how a real drop in reach gets read as the number it was always going to
/// be.
///
/// The two that did not move are the two the new cards had nothing to add
/// to. Loyalty is 7 and the pool holds exactly seven planeswalker faces, so
/// that check was already whole. Mana is 379, and 97 of the pool's 109
/// multi-faced files still being `// GENERATED STUB` is why it is not higher
/// — a stub writes no mana ability for the check to read.
///
/// Optional clause is **47**, measured 2026-09-12: the cards whose printing
/// says "you may" outside reminder text. All 47 carry a construct that asks,
/// and four of them did not until `Effect::MayDo` existed — Ondu Cleric,
/// Kazandu Blademaster, Umara Raptor and Luminarch Ascension, which the check
/// names by slug when the recogniser for `MayDo` is taken out of it.
///
/// Type line is **1473** and that number is exact rather than measured: the
/// pool prints 1475 faces (1255 cards of one face and 110 of two), and the
/// only card whose printed and code face counts disagree is Emeritus of Woe,
/// an adventure Scryfall writes as two faces and this pool as one. Its two
/// are the two that are skipped, so the check reaches every face it can.
/// Nothing is skipped for an unknown subtype word today — every word the
/// pool's 1475 type lines print is in the catalog — which is the number to
/// watch if it ever drops below 1473 without a card leaving the pool.
const PRINTING_FLOOR: PrintingTally = PrintingTally {
    payloads: 1300,
    loyalty: 6,
    identity: 1300,
    keywords: 45,
    mana: 340,
    oracle: 1300,
    costs: 1340,
    player_targets: 20,
    targets: 10,
    optional_clauses: 40,
    printings: 1300,
    type_lines: 1400,
};

/// What [`check_header_matches_code`]'s type segment reached, less a margin.
///
/// It is the whole pool and nothing is skipped: every name in the acceptance
/// list resolves to a compiled `CardDef`, so the count is 1365 — and unlike
/// every floor above it this one needs no printing, which is why it is not a
/// field of [`PrintingTally`]. A card that stopped resolving would be a gap
/// in this command rather than in the pool, and would otherwise be silent.
const HEADER_TYPE_FLOOR: usize = 1360;

/// Keyword bits that have a printed spelling to look for.
///
/// Not every bit does, and the missing ones are deliberate rather than
/// forgotten. `UNBLOCKABLE` and `UNCOUNTERABLE` are our names for printed
/// *sentences* — "can't be blocked", "this spell can't be countered" — that
/// every card words its own way, so there is no single word to find.
const KEYWORD_WORDS: &[(baylee_cards::dsl::KeywordSet, &str)] = {
    use baylee_cards::dsl::KeywordSet as K;
    &[
        (K::FLYING, "flying"),
        (K::FIRST_STRIKE, "first strike"),
        (K::DOUBLE_STRIKE, "double strike"),
        (K::DEATHTOUCH, "deathtouch"),
        (K::HASTE, "haste"),
        (K::HEXPROOF, "hexproof"),
        (K::INDESTRUCTIBLE, "indestructible"),
        (K::LIFELINK, "lifelink"),
        (K::MENACE, "menace"),
        (K::REACH, "reach"),
        (K::TRAMPLE, "trample"),
        (K::VIGILANCE, "vigilance"),
        (K::DEFENDER, "defender"),
        (K::FLASH, "flash"),
        (K::SHROUD, "shroud"),
        (K::FEAR, "fear"),
        (K::INTIMIDATE, "intimidate"),
        (K::SHADOW, "shadow"),
        (K::HORSEMANSHIP, "horsemanship"),
        (K::INFECT, "infect"),
        (K::WITHER, "wither"),
        (K::PERSIST, "persist"),
        (K::UNDYING, "undying"),
        (K::PROWESS, "prowess"),
        (K::SKULK, "skulk"),
        (K::FLANKING, "flanking"),
        (K::CHANGELING, "changeling"),
        (K::PARTNER, "partner"),
        (K::REBOUND, "rebound"),
        (K::PROTECTION_BLACK, "protection from black"),
        (K::DAYBOUND, "daybound"),
        (K::NIGHTBOUND, "nightbound"),
    ]
};

/// Whether `text` says `word` as a word, so a Reach creature does not read as
/// having Flash because its own reminder text mentions flashback.
fn mentions_word(text: &str, word: &str) -> bool {
    let boundary = |c: char| !c.is_alphanumeric();
    text.match_indices(word).any(|(at, _)| {
        text[..at].chars().next_back().is_none_or(boundary)
            && text[at + word.len()..].chars().next().is_none_or(boundary)
    })
}

/// Every face's printed text, joined.
fn printed_text(payload: &serde_json::Value) -> String {
    let mut out = String::new();
    let mut push = |v: Option<&serde_json::Value>| {
        if let Some(s) = v.and_then(serde_json::Value::as_str) {
            out.push_str(s);
            out.push('\n');
        }
    };
    push(payload.get("oracle_text"));
    if let Some(faces) = payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        for face in faces {
            push(face.get("oracle_text"));
        }
    }
    out
}

/// The printed text of each face, kept apart.
///
/// [`printed_text`] joins them, which is right for a search over "does this
/// card say X anywhere" and wrong for anything that wants an index into one
/// face's sentences: Sheoldred's front prints three lines and its back
/// three more, and a saga chapter numbered against the joined string points
/// at the front face a client is not drawing.
fn face_texts(payload: &serde_json::Value) -> Vec<String> {
    let text = |v: Option<&serde_json::Value>| {
        v.and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    if let Some(faces) = payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        return faces.iter().map(|f| text(f.get("oracle_text"))).collect();
    }
    vec![text(payload.get("oracle_text"))]
}

/// Renders `crates/baylee-cards/src/generated_lines.rs`: per card, per
/// face, which printed sentence each ability came from.
///
/// The answer has to be precomputed, and it has to be precomputed *here*.
/// It comes from reading the English oracle text against the compiled
/// ability list, and a running game holds neither — the engine carries no
/// card text at all, and a client fetches the printing and language the
/// player chose rather than Scryfall's English. `xtask` is the one place
/// that links both halves: `baylee-cards` for the compiled `CardDef`s and
/// `baylee-cards-codegen` for the reader, which is the same shape
/// `validate` already has.
///
/// # It is two-phase, and that is what `--check` is for
///
/// The table is built from the pool **compiled into this binary**, so a
/// card added by the run that is writing it is not in the walk: its row
/// lands on the *next* `codegen`, after a rebuild. That is not a race to
/// be fixed here — it is the same order `validate` reads the pool in — and
/// `codegen --check` in CI is what turns a forgotten second run into a
/// build failure rather than a card whose stack entry silently says
/// nothing.
fn render_ability_lines(root: &Path) -> anyhow::Result<String> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    // The pool's *own* spelling of a name, because that is what the
    // printing is cached under: a double-faced card is `A // B` there and
    // its front face alone in `CardDef::name`.
    let names = acceptance::all_names(&rows, &pool_text);
    let by_name: BTreeMap<&str, &'static baylee_cards::dsl::CardDef> =
        baylee_cards::all().map(|def| (def.name(), def)).collect();

    let mut table: Vec<String> =
        vec![String::from("&[],"); baylee_cards::generated::BY_INDEX.len()];
    for name in &names {
        let Some(def) = by_name.get(name.split(" // ").next().unwrap_or(name)) else {
            continue;
        };
        let Some(payload) = cached_printing(root, name) else {
            continue;
        };
        let texts = face_texts(&payload);
        let mut faces: Vec<String> = Vec::new();
        let mut any = false;
        for face in 0..def.faces.len() {
            let abilities = def.abilities_for_face(face);
            let modes: &[baylee_cards::dsl::SpellMode] = abilities
                .iter()
                .find_map(|a| match a {
                    baylee_cards::dsl::AbilityDef::ModalSpell { modes } => Some(*modes),
                    _ => None,
                })
                .unwrap_or(&[]);
            let alternatives = def.faces[face].alternative_costs;
            // An alternative cost is not an ability, so a face that prints
            // one and nothing else — none today — would be dropped by the
            // emptiness test below if it asked about abilities alone. The
            // modes need no such clause: they come out of `abilities`, so
            // a face that has any has an ability too.
            any |= !abilities.is_empty() || !alternatives.is_empty();
            let printed = texts.get(face).map_or("", String::as_str);
            let mapping = lines::map(abilities, printed);
            let stackable = abilities
                .iter()
                .map(lines::ability_shape)
                .filter(|shape| shape.stackable())
                .count();
            let cell = |line: &Option<u8>| {
                line.map_or_else(|| "None".to_string(), |l| format!("Some({l})"))
            };
            let cells: Vec<String> = mapping.lines.iter().map(cell).collect();
            let mode_cells: Vec<String> =
                lines::map_modes(modes, printed).iter().map(cell).collect();
            let alt_cells: Vec<String> = lines::map_alternatives(alternatives, printed)
                .iter()
                .map(cell)
                .collect();
            faces.push(format!(
                "FaceLines {{ sentences: {}, stackable: {}, lines: &[{}], \
                 modes: &[{}], alternatives: &[{}] }}",
                baylee_core::oracle::sentence_count(printed),
                stackable,
                cells.join(", "),
                mode_cells.join(", "),
                alt_cells.join(", ")
            ));
        }
        // A card with no abilities on any face — a vanilla creature, a
        // stub — has nothing to point at, and an empty row keeps the file
        // to the cards the feature is about.
        if !any {
            continue;
        }
        let Some(slot) = table.get_mut(def.index.get() as usize) else {
            continue;
        };
        *slot = format!("// {}\n&[{}],", def.name(), faces.join(", "));
    }

    // The two-phase gap, said out loud where it happens rather than found
    // in CI a day later: cards this run added to the registry are not in
    // the pool this binary compiled against, so they have no row yet.
    let added = names
        .iter()
        .filter(|name| !by_name.contains_key(name.split(" // ").next().unwrap_or(name)))
        .count();
    if added > 0 {
        println!(
            "note: {added} card(s) are newer than the compiled pool; \
             run `cargo xtask codegen` again for their ability lines"
        );
    }

    Ok(format!(
        "// GENERATED by `cargo xtask codegen` — do not edit by hand.\n\
         \n\
         #![allow(missing_docs, unused_imports, dead_code, clippy::all, clippy::pedantic)]\n\
         \n\
         use crate::lines::FaceLines;\n\
         \n\
         /// Per card by `CardIndex`, per face, in `abilities_for_face` order:\n\
         /// which printed sentence each ability came from. `&[]` is a card with\n\
         /// no abilities, a retired index, or one with no cached printing.\n\
         pub static ABILITY_LINES: &[&[FaceLines]] = &[\n{}\n];\n",
        table.join("\n")
    ))
}

/// A Scryfall color letter.
fn color_of_letter(letter: &str) -> Option<baylee_cards::dsl::Color> {
    use baylee_cards::dsl::Color;
    Some(match letter {
        "W" => Color::White,
        "U" => Color::Blue,
        "B" => Color::Black,
        "R" => Color::Red,
        "G" => Color::Green,
        _ => return None,
    })
}

/// A color set as the letters a printing would use, for a message someone
/// can hold against Scryfall.
fn color_letters(set: baylee_cards::dsl::ColorSet) -> String {
    use baylee_cards::dsl::Color;
    let mut out = String::new();
    for (color, letter) in [
        (Color::White, 'W'),
        (Color::Blue, 'U'),
        (Color::Black, 'B'),
        (Color::Red, 'R'),
        (Color::Green, 'G'),
    ] {
        if set.contains(color) {
            out.push(letter);
        }
    }
    if out.is_empty() {
        out.push_str("(colorless)");
    }
    out
}

/// The compiled card against the printing: its color identity, the keyword
/// bits it claims, and the mana its abilities offer to make.
///
/// The other half of [`check_code_matches_the_printing`], and separate from
/// it because these three are not literals a `find` can lift out of a source
/// file — they are sets, computed by codegen or written across several
/// fields, and the compiled `CardDef` is the only place they exist as one
/// answer.
///
/// Two of the five fields the plan named are not here, and both for the same
/// reason: the cache holds `ScryfallCard`, a trimmed struct, not Scryfall's
/// whole payload. It has no `keywords` array and no `produced_mana`, so both
/// are read out of the printed text instead — which the cache does keep, and
/// which is what a person checking the card would read anyway. `defense` is
/// absent from all three: the cache, the DSL, and the pool, which has no
/// battle in it.
fn check_card_matches_the_printing(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // Color identity (CR 903.4) decides which decks may play the card, and
    // it is codegen's arithmetic over every face rather than anything a
    // person typed — which is exactly why nothing was checking it.
    if let Some(letters) = payload
        .get("color_identity")
        .and_then(serde_json::Value::as_array)
    {
        let printed = letters
            .iter()
            .filter_map(serde_json::Value::as_str)
            .filter_map(color_of_letter)
            .fold(baylee_cards::dsl::ColorSet::EMPTY, |set, c| {
                set.union(baylee_cards::dsl::ColorSet::of(c))
            });
        tally.identity += 1;
        if printed != def.color_identity {
            println!(
                "{slug}: the printing's color identity is {} and the code's is {}",
                color_letters(printed),
                color_letters(def.color_identity),
            );
            *problems += 1;
        }
    }

    let whole = printed_text(payload).to_lowercase();
    let printed = strip_reminders(&whole);

    // Keywords, in the one direction that is a bug. A bit the code claims
    // has to be printed on the card; a keyword in the text with no bit is
    // ordinary — most keywords are `AbilityDef` data rather than bits, and a
    // partial card is allowed to leave one unimplemented.
    let claimed = def.all_keywords();
    let mut asked = false;
    for (bit, word) in KEYWORD_WORDS {
        if !claimed.contains(*bit) {
            continue;
        }
        asked = true;
        if !mentions_word(&printed, word) {
            println!("{slug}: the code claims {word} and the printed text never says it");
            *problems += 1;
        }
    }
    tally.keywords += usize::from(asked);

    // Reminder text *kept* for the mana check, and taken out for the keyword
    // one above. Both are load-bearing and they pull opposite ways: a Reach
    // creature must not read as having Flash because its own reminder
    // mentions flashback, and a dual land prints its mana ability as nothing
    // but reminder text — Taiga's whole oracle text is
    // "({T}: Add {R} or {G}.)", so stripping it leaves a land that taps for
    // two colors and says nothing at all.
    check_mana_matches_the_printing(slug, def, &whole, tally, problems);
}

/// Each face's type line, against the one the printing puts on the card.
///
/// This is the cheapest check in the family and it was the missing one: what
/// a card *is* was the one printed characteristic nothing ever asked the
/// printing about. `validate` holds the `//!` header against the **code**,
/// the sweeps hold the engine's object against the `CardDef`, and a card
/// whose header and code agree on the wrong thing walks through both — Ondu
/// Cleric said Human in its header and in its `subtypes` for as long as it
/// had existed, and the printing says Kor. Raffine's Tower is the extreme of
/// the same class: a `TypeSet::LAND` with no subtypes at all, so it made no
/// mana (CR 305.6), and it took a second reader of the reference script to find.
///
/// What is compared is [`baylee_cards::pool::type_line`], the **one**
/// renderer there is, rather than a set built here for the purpose. That is
/// the whole trick: it is the string the deckbuilder shows a player, so a
/// pass says the pool prints what the cards print, and it carries supertypes
/// and types along with the subtypes for free — Karakas and Volrath's
/// Stronghold are `Legendary Land` on the card and were plain `Land` in the
/// code, which is the legend rule (CR 704.5j) not applying to two of them.
/// A second table of type words in this file would have been a second
/// classifier to keep in step, which is the mistake `transcode-report` already
/// made once.
///
/// Order is part of the comparison because it is part of the card: the words
/// on a type line are printed in a fixed order, which is why `TypeSet::words`
/// and `SupertypeSet::words` each carry a hand-kept `PRINTED_ORDER` table to
/// reproduce it, and why a `FaceDef`'s `subtypes` is a slice printed in the
/// order it is written rather than a set. Holding the rendered string against
/// the printed one compares that order for free, and against the only
/// authority there is for it — the card. No rule here says what the order
/// *is*, so nothing here can be wrong about it.
///
/// Two tolerances, and both are about *this repo's* limits rather than the
/// cards'. A face count that disagrees skips the card, exactly as
/// [`check_card_matches_the_printing`]'s cost check does — a split card and
/// an adventure print two faces Scryfall's way and are one `FaceDef` here, so
/// a positional pairing would hold a creature's line against an instant's.
/// And a subtype word the catalog does not know skips the face: the catalogs
/// are what `codegen` builds the constants from, so an unknown word is this
/// command's own gap and never a fact about the card.
fn check_type_line_matches_the_printing(
    slug: &str,
    def: &'static baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let printed_lines: Vec<&str> = match payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        Some(faces) => faces
            .iter()
            .filter_map(|f| f.get("type_line").and_then(serde_json::Value::as_str))
            .collect(),
        None => payload
            .get("type_line")
            .and_then(serde_json::Value::as_str)
            .into_iter()
            .collect(),
    };
    if printed_lines.len() != def.faces.len() {
        return;
    }
    for (at, (line, face)) in printed_lines.iter().zip(def.faces).enumerate() {
        if printed_subtypes(line).is_none() {
            continue;
        }
        tally.type_lines += 1;
        let code = baylee_cards::pool::type_line(face);
        if code != line.trim() {
            let where_ = if def.faces.len() > 1 {
                format!("face {at} of {slug}")
            } else {
                slug.to_string()
            };
            println!("{where_}: the printing's type line is {line:?} and the code's is {code:?}");
            *problems += 1;
        }
    }
}

/// The subtypes one printed type line names, or `None` if a word is unknown.
///
/// Everything after the em dash, longest match first, because "Time Lord" is
/// the one subtype in the whole catalog written as two words and a plain
/// whitespace split would read it as two subtypes that do not exist. A line
/// with no dash prints no subtypes and is an empty list rather than a skip:
/// that a card has none is a fact worth comparing, and it is the half of
/// this check that Raffine's Tower needed.
fn printed_subtypes(type_line: &str) -> Option<Vec<baylee_core::ids::SubtypeId>> {
    use baylee_core::generated::subtypes;
    let Some((_, tail)) = type_line.split_once('\u{2014}') else {
        return Some(Vec::new());
    };
    let words: Vec<&str> = tail.split_whitespace().collect();
    let mut out = Vec::new();
    let mut at = 0;
    while at < words.len() {
        if at + 1 < words.len() {
            let pair = format!("{} {}", words[at], words[at + 1]);
            if let Some(id) = subtypes::by_name(&pair) {
                out.push(id);
                at += 2;
                continue;
            }
        }
        out.push(subtypes::by_name(words[at])?);
        at += 1;
    }
    Some(out)
}

/// What the card's mana abilities make, against what its text offers to add.
///
/// Read from "Add" to the end of the line, not from the whole text, because
/// an activation cost is written in the same symbols and `{G}, {T}: …` would
/// otherwise license a green mana the land never makes.
///
/// A clause that says "any color" names no symbol at all and is the card
/// making every color, so such a card is skipped rather than reported: the
/// code is right to claim five and the text is right to print none.
fn check_mana_matches_the_printing(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    printed: &str,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    use baylee_cards::dsl::{AbilityDef, ManaColor};

    let mut claimed: Vec<char> = Vec::new();
    let lists = std::iter::once(def.abilities).chain(def.faces.iter().map(|f| f.abilities));
    for ability in lists.flatten() {
        let (cost, effects) = match ability {
            AbilityDef::Activated {
                cost,
                effects,
                mana_ability: true,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                effects,
                mana_ability: true,
                ..
            } => (cost, *effects),
            _ => continue,
        };
        // `mana_made` refuses anything a planner could not read — a second
        // effect, a board-dependent source, an amount that is not fixed —
        // and every refusal is a card this check has nothing to say about.
        let Some((made, _restricted)) = baylee_cards::dsl::mana_made(cost, effects) else {
            continue;
        };
        for color in made.colors {
            claimed.push(match color {
                ManaColor::White => 'w',
                ManaColor::Blue => 'u',
                ManaColor::Black => 'b',
                ManaColor::Red => 'r',
                ManaColor::Green => 'g',
                ManaColor::Colorless => 'c',
            });
        }
    }
    if claimed.is_empty() {
        return;
    }

    let mut offered: Vec<char> = Vec::new();
    let mut saw_a_clause = false;
    for (at, _) in printed.match_indices("add ") {
        let clause = printed[at..].split('\n').next().unwrap_or_default();
        // "one mana of any color", "mana of any type that a land you
        // control could produce" — a promise with no symbol in it.
        if clause.contains("any color") || clause.contains("any type") {
            return;
        }
        saw_a_clause = true;
        let bytes: Vec<char> = clause.chars().collect();
        for i in 0..bytes.len().saturating_sub(2) {
            if bytes[i] == '{' && bytes[i + 2] == '}' && "wubrgc".contains(bytes[i + 1]) {
                offered.push(bytes[i + 1]);
            }
        }
    }
    if !saw_a_clause {
        println!(
            "{slug}: the code has a mana ability and the printed text never says \"add\"; \
             a land's intrinsic mana comes off the type line (CR 305.6) and needs no ability"
        );
        *problems += 1;
        return;
    }
    tally.mana += 1;
    for color in claimed {
        if !offered.contains(&color) {
            println!(
                "{slug}: the code taps for {{{}}} and the printing never offers it",
                color.to_ascii_uppercase()
            );
            *problems += 1;
        }
    }
}

/// Scryfall's spelling of a costless card is an empty string; the pool's is
/// the one [`code_costs`] already normalises everything else to.
fn normalise_cost(printed: &str) -> String {
    if printed.is_empty() || printed == "{0}" {
        "(no cost)".to_string()
    } else {
        printed.to_string()
    }
}

/// The first `field` number in `face`, as it is written.
fn field_number(face: &str, field: &str) -> Option<String> {
    let rest = &face[face.find(field)? + field.len()..];
    let end = rest.find(')')?;
    Some(rest[..end].to_string())
}

/// The `//!` header, against the `CardDef` the file builds below it.
///
/// Name, cost and the two ids were the whole of it, and the type segment was
/// the gap: thirteen lands said `Land — SWAMP MOUNTAIN` in the constants'
/// spelling rather than the card's, two said `Land` where the card prints
/// `Legendary Land`, and nothing read any of it.
///
/// It closes a triangle rather than adding a check.
/// [`check_type_line_matches_the_printing`] holds the code against the
/// printing, so holding the header against the **code** is what makes the
/// header right about the *card*. Before that check existed this comparison
/// would only have said that a person typed the same thing twice, which is
/// precisely what Ondu Cleric did for as long as it existed — so the order
/// the two checks were written in is the reason either means anything.
///
/// The code's side is [`baylee_cards::pool::type_line`] joined with `" // "`,
/// for the same reason the printing check uses it: one renderer, and it is
/// the one the deckbuilder shows a player. A header names the faces the
/// *file* has rather than the ones Scryfall writes, so the join is exact at
/// both ends of that difference — Conqueror's Galleon prints
/// `Artifact — Vehicle // Land` and renders it, and Emeritus of Woe, the
/// adventure this pool models as one face, compares its one line and passes
/// where the printing check has to skip it.
fn check_header_matches_code(
    slug: &str,
    content: &str,
    def: Option<&'static baylee_cards::dsl::CardDef>,
    header_types: &mut usize,
    problems: &mut usize,
) {
    // First header line: `//! <Name> — <cost or "(no cost)"> — <types>`.
    let Some(header) = content.lines().find(|l| l.starts_with("//! ")) else {
        println!("{slug}: no header line");
        *problems += 1;
        return;
    };
    let header = &header[4..];
    // Three, and the third keeps whatever is left: a double-faced card's
    // segment is `Artifact — Vehicle // Land` and has a dash of its own.
    let mut parts = header.splitn(3, " — ");
    let (head_name, head_cost, head_types) = (
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
    );

    // The first face's name (token statics can appear before CARD, so
    // anchor on the `faces` field). MDFC headers read "Front // Back":
    // either side may headline the file's first face.
    let code_name = content
        .find("faces = &[")
        .and_then(|pos| knob(&content[pos..], "name"))
        .and_then(|v| quoted_value(v, "\""));
    if let Some(code_name) = code_name {
        let matches = head_name.split(" // ").any(|side| side == code_name);
        if !matches {
            println!("{slug}: header name {head_name:?} != code name {code_name:?}");
            *problems += 1;
        }
    }
    // The header cost must be one of the faces' costs.
    let costs = code_costs(content);
    let head_cost_norm = if head_cost == "{0}" {
        "(no cost)"
    } else {
        head_cost
    };
    if !costs.is_empty() && !costs.iter().any(|c| c == head_cost_norm) {
        println!("{slug}: header cost {head_cost:?} matches none of the code costs {costs:?}");
        *problems += 1;
    }
    // The type segment, through the one renderer there is.
    if let Some(def) = def {
        let code_types = def
            .faces
            .iter()
            .map(baylee_cards::pool::type_line)
            .collect::<Vec<_>>()
            .join(" // ");
        *header_types += 1;
        if head_types != code_types {
            println!("{slug}: header type line {head_types:?} != code {code_types:?}");
            *problems += 1;
        }
    }
    // `knob` and never a literal `"<field>: \""`: a card file writes
    // `scryfall_id = "…"`, so the colon spelling matched nought of the 1365
    // and `code_value` was `None` for every card — which this comparison
    // answers by saying nothing, so it ran on every card and compared none.
    // A missing half is *also* reported now, because "the header and the
    // code agree" and "one of them is not there" are different answers and
    // only one of them was being given.
    for (label, field) in [("Scryfall ID", "scryfall_id"), ("Oracle ID", "oracle_id")] {
        let header_has = content
            .lines()
            .find(|l| l.starts_with("//!") && l.contains(label))
            .and_then(|l| l.split(&format!("{label}: ")).nth(1))
            .map(|v| v.split([' ', '|']).next().unwrap_or("").trim());
        let code_value = knob(content, field)
            .and_then(|v| v.strip_prefix('"'))
            .and_then(|v| v.split_once('"'))
            .map(|(id, _)| id);
        match (header_has, code_value) {
            (Some(h), Some(c)) if h != c => {
                println!("{slug}: header {label} {h:?} != code {c:?}");
                *problems += 1;
            }
            (Some(_), None) => {
                println!("{slug}: header carries a {label} and the code writes none");
                *problems += 1;
            }
            _ => {}
        }
    }
}

/// Compares "onto the battlefield **tapped**" in the printed text against the
/// [`Find`](baylee_cards_dsl::effect::Find) the code searches with.
///
/// One word, and it is the difference between two families of land that look
/// identical in a card file: Evolving Wilds puts its basic in tapped and costs
/// nothing, a fetchland puts its dual in untapped and costs a life. Four of
/// the ten fetchlands in the pool shipped with `Find::BATTLEFIELD_TAPPED`
/// while their own header quoted "put it onto the battlefield" and their own
/// comment said `Find::BATTLEFIELD` — nothing compared the two, so the pool
/// disagreed with itself for as long as it had fetchlands in it, and the
/// engine test written from the code rather than from the card made it
/// permanent.
///
/// The reading is deliberately narrow. Only a *put* counts, so a land whose
/// own text says it enters the battlefield tapped is not mistaken for one
/// that taps what it finds, and the claim is presence rather than a count:
/// Cultivate names one destination per sentence and Sword of Hearth and Home
/// names two cards in one, and neither is a drift this check is for.
fn check_search_tapped_matches_text(slug: &str, content: &str, problems: &mut usize) {
    /// The printed phrase both halves of the comparison hang off.
    const ONTO: &str = "onto the battlefield";

    // The code only. A card's comments quote both spellings while explaining
    // which one it is, which is exactly the sentence that went stale.
    let code: String = content
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let finds: String = code
        .match_indices("finds:")
        .filter_map(|(at, _)| {
            let rest = &code[at..];
            rest.find(']').map(|end| &rest[..end])
        })
        .collect::<Vec<_>>()
        .join(" ");
    if finds.is_empty() {
        return;
    }
    let code_tapped = finds.contains("Find::BATTLEFIELD_TAPPED");
    let code_plain = finds
        .match_indices("Find::BATTLEFIELD")
        .any(|(at, _)| !finds[at..].starts_with("Find::BATTLEFIELD_TAPPED"));

    let oracle: String = content
        .lines()
        .filter_map(|l| l.strip_prefix("//! Oracle:"))
        .collect::<Vec<_>>()
        .join(" ");
    let (mut text_tapped, mut text_plain) = (false, false);
    for (at, _) in oracle.match_indices(ONTO) {
        let before: String = oracle[..at]
            .chars()
            .rev()
            .take(60)
            .collect::<String>()
            .to_ascii_lowercase();
        if !before.contains("tup") {
            continue; // "put", read backwards — this is an entry, not a put.
        }
        if oracle[at + ONTO.len()..].starts_with(" tapped") {
            text_tapped = true;
        } else {
            text_plain = true;
        }
    }

    if code_tapped && !text_tapped {
        println!("{slug}: code puts a found card onto the battlefield tapped, the text does not");
        *problems += 1;
    }
    if code_plain && !text_plain {
        println!("{slug}: code puts a found card onto the battlefield untapped, the text does not");
        *problems += 1;
    }
}

/// Validates card-file conventions across the registry.
/// Strips the ownership marker from one generated card, handing the file to
/// whoever asked for it.
///
/// The marker line is rewritten rather than deleted, keeping the summary the
/// reader wrote: what the card does is still true, and what changes is only
/// who may say it from now on. A stub is refused — there is nothing to adopt
/// in a card nobody has implemented, and taking a stub off the machine would
/// freeze it as an `Unimplemented` card codegen can never finish.
fn adopt(root: &Path, name: &str) -> anyhow::Result<()> {
    let cards_dir = root.join("crates/baylee-cards/src/cards");
    let slug = front_face_slug(name);
    let files = card_files(&cards_dir)?;
    let path = files
        .get(&slug)
        .ok_or_else(|| anyhow::anyhow!("no card file for {name} (slug {slug})"))?;
    let text = fs::read_to_string(path)?;
    if text.contains(stubgen::STUB_MARKER) {
        anyhow::bail!("{name} is still a generated stub — implement it first, then adopt it");
    }
    let Some(line) = text.lines().find(|l| l.starts_with(stubgen::OWNED_MARKER)) else {
        anyhow::bail!("{name} is already hand-owned");
    };
    let summary = line
        .strip_prefix(stubgen::OWNED_MARKER)
        .map(|rest| rest.trim_start_matches(':'))
        .unwrap_or_default()
        .trim()
        .trim_end_matches('.');
    let adopted = format!("// IMPLEMENTED \u{2014} {summary}, adopted from `xtask codegen`.");
    fs::write(path, text.replacen(line, &adopted, 1))?;
    println!("adopted {name} -> {}", relative(path, &cards_dir));
    println!("codegen will not write this file again; it is yours now.");
    Ok(())
}

/// Rewrites every card's `//! Oracle:` block from its cached printing.
///
/// The counterpart to [`check_oracle_matches_the_printing`], and it exists
/// for the reason `codegen` exists: the header is derived data, so it is
/// written by whatever holds the source of truth rather than retyped. Forty-
/// eight hand-owned cards disagreed with their own printing on the day this
/// was written, and retyping forty-eight blocks of rules text by hand is the
/// step that produced most of them.
///
/// It reaches hand-owned files, which `codegen` deliberately does not,
/// because the two are writing different things. `codegen` writes what a
/// card *does*, and a hand-owned card is exactly one whose behaviour a
/// person took over; this writes only the sentence the behaviour is measured
/// against, which is nobody's to author. So an errata is taken by running
/// this and then **reading the diff**: a printing that changed usually means
/// the code below it has to change too, and the refreshed header is what
/// makes that visible instead of leaving the card agreeing with a sentence
/// Wizards has since replaced.
fn refresh_oracle(root: &Path, dry_run: bool) -> anyhow::Result<()> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    let cards_dir = root.join("crates/baylee-cards/src/cards");
    let files = card_files(&cards_dir)?;
    let mut changed = 0usize;
    for name in &names {
        let slug = front_face_slug(name);
        let (Some(path), Some(payload)) = (files.get(&slug), cached_printing(root, name)) else {
            continue;
        };
        let text = fs::read_to_string(path)?;
        // Two derived lines, one pass. The Set line rode along here rather
        // than getting a command of its own because it is the same claim as
        // the Oracle block — "this is the printing the card below was built
        // from" — and a card whose printing moved needs both rewritten or
        // the header names one printing and quotes another.
        let mut next = text.clone();
        if let Some(oracle) = with_oracle_header(&next, &printed_text(&payload)) {
            next = oracle;
        }
        if let Some(line) = set_header_line(&payload)
            && let Some(set) = with_set_header(&next, &line)
        {
            next = set;
        }
        if next == text {
            continue;
        }
        changed += 1;
        println!("{}", relative(path, &cards_dir));
        if !dry_run {
            fs::write(path, next)?;
        }
    }
    let verb = if dry_run {
        "would refresh"
    } else {
        "refreshed"
    };
    println!("{verb} {changed} of {} headers", names.len());
    Ok(())
}

/// The `//! Set:` line the payload calls for, or `None` when the payload is
/// missing a piece of it.
///
/// Spelled by [`baylee_cards_codegen::stubgen::set_line`] and not here, so
/// the line a refresh writes into a hand-owned file is byte-for-byte the one
/// `codegen` writes into a machine-owned one.
fn set_header_line(payload: &serde_json::Value) -> Option<String> {
    let field = |k: &str| payload.get(k).and_then(serde_json::Value::as_str);
    Some(baylee_cards_codegen::stubgen::set_line(
        field("set")?,
        field("collector_number")?,
        field("set_name")?,
        field("id")?,
        field("oracle_id").unwrap_or_default(),
    ))
}

/// `text` with its `//! Set:` line replaced by `line`, or `None` when it
/// already says exactly that.
///
/// Replaced where it stands rather than moved: the Oracle block above is
/// rewritten by dropping and re-emitting it, because its *length* changes
/// with the printing, and this line's does not.
fn with_set_header(text: &str, line: &str) -> Option<String> {
    let old = text.lines().find(|l| l.starts_with("//! Set:"))?;
    (old != line.trim_end()).then(|| text.replacen(old, line.trim_end(), 1))
}

/// `text` with its `//! Oracle:` block replaced by `printed`, or `None` when
/// it already says exactly that.
///
/// The block is rewritten in place rather than edited line by line: the old
/// lines are dropped wherever they sat and the new ones go straight after
/// the name line, which is where `stubgen` puts them, so a hand-written file
/// ends up with the header a generated one would have had.
fn with_oracle_header(text: &str, printed: &str) -> Option<String> {
    let lines: Vec<&str> = printed
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let mut out = String::with_capacity(text.len() + 256);
    let mut written = false;
    for line in text.lines() {
        if line.starts_with("//! Oracle:") {
            continue; // the old block, dropped wherever it sat
        }
        out.push_str(line);
        out.push('\n');
        if !written && line.starts_with("//!") {
            written = true;
            for oracle in &lines {
                out.push_str("//! Oracle: ");
                out.push_str(oracle);
                out.push('\n');
            }
        }
    }
    (out != text).then_some(out)
}

/// Cards whose printed "you control" or "an opponent controls" is not a
/// filter on any object, with the reason.
///
/// Every entry here is a sentence the DSL says another way, and naming the
/// way is the point: an exception with no reason is a card nobody looked at.
const SCOPE_EXCEPTIONS: &[(&str, &str)] = &[
    ("Bleachbone Verge", "an Condition, not a filter"),
    ("Mox Opal", "metalcraft is an Condition"),
    ("Fierce Guardianship", "an AlternativeCost condition"),
    (
        "Deadly Rollick",
        "the same cycle, the same AlternativeCost condition",
    ),
    (
        "Deflecting Swat",
        "the third of that cycle, and the same condition again",
    ),
    (
        "Strength of the Harvest",
        "ModifyPTPerCount already counts the effect controller's side only",
    ),
    (
        "Reflecting Pool",
        "\"any type a land you control could produce\" is its own mana rule",
    ),
    ("Exotic Orchard", "the same rule, read across the table"),
    ("Fellwar Stone", "the same rule, read across the table"),
    (
        "Opposition Agent",
        "\"you control your opponents\" is the verb, not the zone",
    ),
    ("Urza's Saga", "the clause is printed on the token it makes"),
    (
        "Ashiok, Dream Render",
        "a player-wide static with no object to filter",
    ),
    (
        "Karn, the Great Creator",
        "the lock is a player rule; see karns_lock_spares_a_teammate",
    ),
];

/// What the card says about *whose* permanents it reaches, against what it
/// does.
///
/// The owner's question, in two directions: an effect that should only touch
/// your own side must say so, and one that reaches across the table must not
/// be able to come back. Both are invisible in a duel played once — a wrong
/// filter still points at *something* — and both are decidable by reading
/// the printed sentence beside the filters the card was built from.
///
/// Reminder text is stripped first. It is printed in parentheses, it is not
/// rules the card carries, and hexproof's own reminder ends "…spells or
/// abilities your opponents control" on every card that has it.
fn check_scope_matches_the_text(
    slug: &str,
    name: &str,
    def: &baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    problems: &mut usize,
) {
    if !matches!(def.coverage, baylee_cards::dsl::Coverage::Implemented) {
        return;
    }
    if SCOPE_EXCEPTIONS.iter().any(|(card, _)| *card == name) {
        return;
    }
    let printed = strip_reminders(&printed_text(payload)).to_lowercase();

    // The filters the card was built from, read off the one rendering that
    // cannot go stale as the DSL grows a variant.
    let built = format!("{def:?}");
    let says_you = printed.contains("you control");
    let says_theirs =
        printed.contains("opponent controls") || printed.contains("opponents control");
    // `Condition::ControlCount` is the second spelling of "you control", and
    // not a filter at all: `eval::condition_holds` counts only permanents
    // whose `controller == you` and hands the filter each candidate's own id,
    // so the filter inside it says *what* to count and never whose. A card
    // whose only such clause is a count — "activate only if you control an
    // Island", "if you control three or more artifacts" — would otherwise be
    // asked for a `ControlledByYou` that would change nothing, and eighteen
    // generated lands landed in one commit with exactly that shape.
    let filters_you = built.contains("ControlledByYou") || built.contains("ControlCount(");
    let filters_theirs = built.contains("ControlledByOpponent");

    if says_you && !filters_you {
        println!(
            "{slug}: the text says \"you control\" and no filter does; use `Filter::ControlledByYou`, or add an entry to SCOPE_EXCEPTIONS saying why not"
        );
        *problems += 1;
    }
    if says_theirs && !filters_theirs {
        println!(
            "{slug}: the text says an opponent controls it and no filter does; use `Filter::ControlledByOpponent`, or add an entry to SCOPE_EXCEPTIONS"
        );
        *problems += 1;
    }
    if filters_theirs && !printed.contains("opponent") {
        println!(
            "{slug}: reaches only an opponent's permanents and the text never says opponent; \"you don't control\" is `Filter::Not(&Filter::ControlledByYou)`, which a teammate's permanent matches too"
        );
        *problems += 1;
    }
}

/// Oracle text with its reminder text taken out.
///
/// One implementation, in `lines`, because the sentence reader there needs
/// exactly this and a second copy of it would be free to drift from the
/// one the generated table is built with.
fn strip_reminders(text: &str) -> String {
    lines::without_reminder(text)
}

/// Fills the Scryfall payload cache and reports what it cost.
///
/// The same two steps `codegen` takes before it generates anything — one bulk
/// download for a cold cache, then one request per card the feed did not
/// carry — with none of the generation behind them. That split is the point:
/// a scheduled job can keep the cache warm without the card-script corpus,
/// and without failing on the stale files a pool change legitimately leaves
/// behind.
fn scryfall_cache(root: &Path, cache: &Path) -> anyhow::Result<()> {
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
    scryfall::fill_from_bulk(&names, &agent, &cache);
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
    Ok(())
}

fn validate(root: &Path) -> anyhow::Result<()> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    // The same set `codegen` writes. Reading only the acceptance decks here
    // meant every card added for its own sake was generated and then never
    // checked — the header-vs-code comparison below is the whole point of
    // this command, and it silently skipped them.
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    let mut problems = 0usize;
    let mut stubs = 0usize;
    let mut tally = PrintingTally::default();
    // Cards whose header type segment was held against the code's. It is not
    // in `PrintingTally` because it is not a comparison against a printing —
    // it holds two things a person wrote against each other — but it is
    // counted for the reason all of those are: the check skips a card whose
    // name resolves to no `CardDef`, and a silent skip that grows is how a
    // sweep stops reaching the pool without saying so.
    let mut header_types = 0usize;
    // Who owns each finished card, which is the number to watch: a machine-
    // owned card is a reader's output and is corrected by fixing the reader,
    // a hand-owned one is somebody's and codegen never touches it.
    let mut machine = 0usize;
    // `validate` reads every card's header off disk, so it has to find the
    // files the taxonomy filed rather than the ones a flat directory used to
    // hold. A walk that found nothing would report a clean pool.
    let files = card_files(&root.join("crates/baylee-cards/src/cards"))?;
    // The compiled pool beside the files: the scope check reads the filters
    // a card was built from, which no amount of reading its source text
    // gives you.
    let by_name: BTreeMap<&str, &'static baylee_cards::dsl::CardDef> =
        baylee_cards::all().map(|def| (def.name(), def)).collect();
    for name in &names {
        let slug = front_face_slug(name);
        let Some(content) = files.get(&slug).and_then(|p| fs::read_to_string(p).ok()) else {
            println!("MISSING FILE: {slug}");
            problems += 1;
            continue;
        };
        // A stub has nothing to claim: `CardDef::DEFAULT` is
        // `Unimplemented`, and writing the line out would be restating a
        // default. The header is still checked, because that is what the
        // person who finishes the card reads.
        let is_stub = content.contains(stubgen::STUB_MARKER);
        stubs += usize::from(is_stub);
        machine += usize::from(!is_stub && content.contains(stubgen::OWNED_MARKER));
        for check in [
            ("header name", content.contains("//!")),
            ("set line", content.contains("Set:")),
            ("scryfall id", content.contains("Scryfall ID:")),
            ("oracle id", content.contains("Oracle ID:")),
            (
                "coverage flag",
                is_stub || knob(&content, "coverage").is_some_and(|v| v.starts_with("Coverage::")),
            ),
        ] {
            if !check.1 {
                println!("{slug}: missing {}", check.0);
                problems += 1;
            }
        }
        check_search_tapped_matches_text(&slug, &content, &mut problems);
        let def = by_name.get(name.split(" // ").next().unwrap_or(name));
        check_header_matches_code(
            &slug,
            &content,
            def.copied(),
            &mut header_types,
            &mut problems,
        );
        // One read for the three checks that need it, and the tally counts
        // the card here rather than inside one of them: "the payload was
        // found" is a fact about the card, not about whichever check
        // happened to look first.
        let Some(payload) = cached_printing(root, name) else {
            continue;
        };
        tally.payloads += 1;
        if let Some(def) = def {
            check_scope_matches_the_text(&slug, def.name(), def, &payload, &mut problems);
            check_card_matches_the_printing(&slug, def, &payload, &mut tally, &mut problems);
            check_type_line_matches_the_printing(&slug, def, &payload, &mut tally, &mut problems);
            check_optional_clauses_are_offered(&slug, def, &payload, &mut tally, &mut problems);
        }
        check_code_matches_the_printing(&slug, &content, &payload, &mut tally, &mut problems);
        check_oracle_matches_the_printing(&slug, &content, &payload, &mut tally, &mut problems);
        check_set_line_matches_the_printing(&slug, &content, &payload, &mut tally, &mut problems);
        check_target_counts_match_the_printing(
            &slug,
            &content,
            &payload,
            &mut tally,
            &mut problems,
        );
        check_player_targets_match_the_printing(
            &slug,
            &content,
            &payload,
            &mut tally,
            &mut problems,
        );
    }
    check_no_name_is_claimed_twice(&mut problems);
    report_what_the_sweeps_reached(&tally, header_types, &mut problems);
    if problems > 0 {
        anyhow::bail!("{problems} convention problem(s) found");
    }
    println!(
        "validate: {} cards conform ({} finished \u{2014} {} hand-owned, {machine} \
         machine-owned \u{2014} and {stubs} stubs)",
        names.len(),
        names.len() - stubs,
        names.len() - stubs - machine
    );
    Ok(())
}

/// Every count, then every floor — before the bail rather than after it,
/// because a floor that failed is only readable beside the counts that
/// failed it.
fn report_what_the_sweeps_reached(
    tally: &PrintingTally,
    header_types: usize,
    problems: &mut usize,
) {
    println!(
        "validate: against the printings \u{2014} {} payloads, {} loyalty, {} identity, \
         {} keyword, {} mana, {} oracle, {} cost, {} player target, {} target count, \
         {} optional clause, {} printing, {} type line",
        tally.payloads,
        tally.loyalty,
        tally.identity,
        tally.keywords,
        tally.mana,
        tally.oracle,
        tally.costs,
        tally.player_targets,
        tally.targets,
        tally.optional_clauses,
        tally.printings,
        tally.type_lines
    );
    check_printing_floors(tally, problems);
    println!("validate: {header_types} header type lines against the code");
    if header_types < HEADER_TYPE_FLOOR {
        println!(
            "only {header_types} header type lines were held against the code and the floor \
             is {HEADER_TYPE_FLOOR}; the check is not reaching the pool"
        );
        *problems += 1;
    }
}

/// Holds every printing check to what it reached last time.
fn check_printing_floors(tally: &PrintingTally, problems: &mut usize) {
    for (what, seen, floor) in [
        ("payloads", tally.payloads, PRINTING_FLOOR.payloads),
        ("loyalty", tally.loyalty, PRINTING_FLOOR.loyalty),
        ("color identity", tally.identity, PRINTING_FLOOR.identity),
        ("keyword", tally.keywords, PRINTING_FLOOR.keywords),
        ("mana", tally.mana, PRINTING_FLOOR.mana),
        ("oracle text", tally.oracle, PRINTING_FLOOR.oracle),
        ("face cost", tally.costs, PRINTING_FLOOR.costs),
        (
            "player target",
            tally.player_targets,
            PRINTING_FLOOR.player_targets,
        ),
        ("target count", tally.targets, PRINTING_FLOOR.targets),
        (
            "optional clause",
            tally.optional_clauses,
            PRINTING_FLOOR.optional_clauses,
        ),
        ("printing", tally.printings, PRINTING_FLOOR.printings),
        ("type line", tally.type_lines, PRINTING_FLOOR.type_lines),
    ] {
        if seen < floor {
            println!(
                "only {seen} {what} comparisons were made against the printings and the \
                 floor is {floor}; the sweep is not reaching the pool"
            );
            *problems += 1;
        }
    }
}

/// Writes every compiled `CardDef` to `out`, one `Debug` rendering per card.
///
/// The point is the diff, not the content: a refactor that is supposed to
/// change no rules produces the same bytes, and one that slipped produces a
/// hunk with the card's name in it. That is how the macro/prelude refactor of
/// the whole pool was held to "not one rule moved".
fn pool_dump(out: &Path) -> anyhow::Result<()> {
    use std::fmt::Write as _;
    let mut text = String::new();
    for def in baylee_cards::all() {
        let _ = writeln!(text, "{def:#?}");
    }
    fs::write(out, &text)?;
    println!(
        "pool dump: {} cards -> {}",
        baylee_cards::count(),
        out.display()
    );
    Ok(())
}

fn explain(root: &Path, name: &str, scripts_dir: &Path, cache: &Path) -> anyhow::Result<()> {
    let cache = root.join(cache);
    let agent = ureq::Agent::new_with_defaults();
    let card = scryfall::fetch_named(name, &agent, &cache)?;
    println!(
        "== Scryfall ==\n{} — {} — {}\n{}\n",
        card.name,
        card.mana_cost.as_deref().unwrap_or(""),
        card.type_line.as_deref().unwrap_or(""),
        card.oracle_text.as_deref().unwrap_or("")
    );
    let index_path = root.join("data/script-index.json");
    if index_path.exists() {
        let index: BTreeMap<String, String> =
            serde_json::from_str(&fs::read_to_string(&index_path)?)?;
        if let Some(rel) = index.get(name) {
            let script = scripts_root(root, scripts_dir).join(rel);
            println!("== card-script reference ({}) ==", script.display());
            let text = fs::read_to_string(&script).unwrap_or_default();
            println!("{text}");
            // What the transcoder makes of it, which is the question a
            // person opening this tool on a stub is actually asking. It was
            // answerable only by a corpus-wide `transcode-report` run,
            // whose ranking is about the corpus and not about this card.
            let mut cats = catalog::SubtypeCatalogs {
                creature: scryfall::fetch_catalog("creature-types", &agent, &cache)?,
                artifact: scryfall::fetch_catalog("artifact-types", &agent, &cache)?,
                enchantment: scryfall::fetch_catalog("enchantment-types", &agent, &cache)?,
                land: scryfall::fetch_catalog("land-types", &agent, &cache)?,
                planeswalker: scryfall::fetch_catalog("planeswalker-types", &agent, &cache)?,
                spell: scryfall::fetch_catalog("spell-types", &agent, &cache)?,
            };
            cats.normalize();
            let parsed = scriptgen::parse(&text);
            println!("== transcoder ==");
            if scriptgen::transcode(&parsed, &cats).is_some() {
                println!("read in full");
            } else {
                println!("refused: {}", refusal_cause(&parsed, &cats));
            }
        } else {
            println!("card-script reference: no script found for {name:?}");
        }
    } else {
        println!("note: data/script-index.json missing; run `cargo xtask codegen`");
    }
    Ok(())
}

// ------------------------------------------------------------- dev table

/// The dev account. Fixed, so a repeated run reuses one account and one deck
/// rather than filling the store with strangers.
const DEV_EMAIL: &str = "dev@baylee.local";
/// The dev account's password. This account exists only on a developer's own
/// gateway and owns nothing worth taking.
const DEV_PASSWORD: &str = "dev-password-dev-password";
/// The dev account's display name.
const DEV_NAME: &str = "dev";

/// POSTs JSON and returns `(status, body)`. A refusal is a body, not an
/// error: several steps here expect one (an account that already exists).
fn post(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
    body: &serde_json::Value,
) -> anyhow::Result<(u16, String)> {
    let mut req = agent.post(url).header("content-type", "application/json");
    if let Some(token) = token {
        req = req.header("authorization", &format!("Bearer {token}"));
    }
    match req.send_json(body) {
        Ok(mut resp) => Ok((resp.status().as_u16(), resp.body_mut().read_to_string()?)),
        Err(ureq::Error::StatusCode(code)) => Ok((code, String::new())),
        Err(e) => Err(anyhow::anyhow!("{url}: {e}")),
    }
}

/// PUTs JSON and returns the status. Same shape as [`post`], and the same
/// reason for returning a refusal rather than raising it.
fn put(
    agent: &ureq::Agent,
    url: &str,
    token: &str,
    body: &serde_json::Value,
) -> anyhow::Result<u16> {
    let req = agent
        .put(url)
        .header("content-type", "application/json")
        .header("authorization", &format!("Bearer {token}"));
    match req.send_json(body) {
        Ok(resp) => Ok(resp.status().as_u16()),
        Err(ureq::Error::StatusCode(code)) => Ok(code),
        Err(e) => Err(anyhow::anyhow!("{url}: {e}")),
    }
}

/// GETs JSON and returns the body.
fn get(agent: &ureq::Agent, url: &str, token: &str) -> anyhow::Result<String> {
    let mut resp = agent
        .get(url)
        .header("authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| anyhow::anyhow!("{url}: {e}"))?;
    Ok(resp.body_mut().read_to_string()?)
}

/// Pulls a string field out of a JSON object body.
fn field(body: &str, name: &str) -> anyhow::Result<String> {
    let value: serde_json::Value = serde_json::from_str(body)?;
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("no `{name}` in {body}"))
}

/// The acceptance file's deck, as the `POST /decks` body wants it.
fn acceptance_deck(root: &Path, name: &str) -> anyhow::Result<serde_json::Value> {
    let text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&text).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut main = Vec::new();
    let mut side = Vec::new();
    // `POST /decks` takes one commander by name, and takes it *beside* the
    // rows — `LoadedDeck` seats a leader that has no row of its own. This used
    // to drop the row entirely, under a comment saying the engine did not run
    // the format; it has since, and the dev table was the one seat at a
    // commander table playing without a commander.
    let mut commander = None;
    for row in rows.iter().filter(|r| r.deck == name) {
        let line = format!("{} {}", row.count, row.name);
        match row.zone {
            acceptance::Zone::Main => main.push(line),
            acceptance::Zone::Sideboard => side.push(line),
            acceptance::Zone::Commander => commander = Some(row.name.clone()),
        }
    }
    anyhow::ensure!(!main.is_empty(), "no deck called `{name}` in the file");
    Ok(serde_json::json!({
        "name": name,
        "cards": main,
        "sideboard": side,
        "commander": commander,
    }))
}

/// Arranges a room's chairs and starts it.
///
/// Split out of [`dev_table`] because it is the half that talks to the lobby
/// as a *host*: the AI chairs, the sides and the two statements a start takes.
fn arrange_room(
    agent: &ureq::Agent,
    gateway: &str,
    token: &str,
    game_id: &str,
    seats: usize,
    ai: &str,
    teams: &[u8],
) -> anyhow::Result<()> {
    // A room's other chairs still have to be handed over; the two-seat path
    // already came back with its house AI seated, and reaching into that
    // chair would be a `409`.
    if seats > 2 {
        for seat in 1..seats {
            let url = format!("{gateway}/lobby/games/{game_id}/seats/{seat}");
            let (status, body) = post(
                agent,
                &url,
                Some(token),
                &serde_json::json!({ "kind": "ai", "ai": ai }),
            )?;
            anyhow::ensure!(
                status == 200,
                "seat the AI in chair {seat}: {status} {body}"
            );
        }
    }

    // Sides, once every chair is arranged: the host says who plays with whom,
    // a side being the format rather than a preference.
    for (seat, team) in teams.iter().enumerate() {
        let url = format!("{gateway}/lobby/games/{game_id}/seats/{seat}");
        let (status, body) = post(
            agent,
            &url,
            Some(token),
            &serde_json::json!({ "team": team }),
        )?;
        anyhow::ensure!(
            status == 200,
            "put chair {seat} on team {team}: {status} {body}"
        );
    }

    // A room does not start itself — that takes two statements by two people
    // (`ready` is the player's, `start` is the host's), and here the dev
    // account is both. An AI chair is ready as soon as it is configured.
    if seats > 2 {
        let (status, body) = post(
            agent,
            &format!("{gateway}/lobby/games/{game_id}/ready"),
            Some(token),
            &serde_json::json!({ "ready": true }),
        )?;
        anyhow::ensure!(status == 200, "say ready: {status} {body}");
        let (status, body) = post(
            agent,
            &format!("{gateway}/lobby/games/{game_id}/start"),
            Some(token),
            &serde_json::json!({}),
        )?;
        anyhow::ensure!(status == 200, "start the table: {status} {body}");
    }
    Ok(())
}

/// Seats the dev account at a table and prints or plays its ticket.
///
/// Every step is a real request to a real gateway: the only thing skipped is
/// having to type them into the lobby.
fn dev_table(
    root: &Path,
    gateway: &str,
    seats: usize,
    ai: &str,
    deck_name: &str,
    teams: &[u8],
    play: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        (2..=8).contains(&seats),
        "a table seats between two and eight"
    );
    anyhow::ensure!(
        teams.is_empty() || seats > 2,
        "--teams needs three chairs or more; a duel is already two sides"
    );
    anyhow::ensure!(
        teams.is_empty() || teams.len() == seats,
        "--teams needs one side per chair ({seats} of them)"
    );
    anyhow::ensure!(
        teams.is_empty() || teams.iter().any(|t| *t != teams[0]),
        "a table needs two sides; every chair is on team {}",
        teams.first().copied().unwrap_or(0)
    );
    let agent = ureq::Agent::new_with_defaults();

    // An account. A second run finds it already there, which is not an error.
    let _ = post(
        &agent,
        &format!("{gateway}/auth/register"),
        None,
        &serde_json::json!({
            "email": DEV_EMAIL, "display_name": DEV_NAME, "password": DEV_PASSWORD,
        }),
    )?;
    let (status, body) = post(
        &agent,
        &format!("{gateway}/auth/login"),
        None,
        &serde_json::json!({ "email": DEV_EMAIL, "password": DEV_PASSWORD }),
    )?;
    anyhow::ensure!(status == 200, "sign in as {DEV_NAME}: {status} {body}");
    let token = field(&body, "token")?;

    // A deck. The account survives between runs, so one is usually already
    // stored — and it is *rewritten* rather than reused, because
    // `data/acceptance-decks.txt` is what a dev table is supposed to be
    // playing. A deck saved by an older build simply stayed as it was, which
    // is how seat 0 kept sitting down at a commander table with no commander
    // for a while after the file had one.
    let body = acceptance_deck(root, deck_name)?;
    let decks: serde_json::Value =
        serde_json::from_str(&get(&agent, &format!("{gateway}/decks"), &token)?)?;
    let existing = decks.as_array().and_then(|list| {
        list.iter()
            .find(|d| d.get("name").and_then(serde_json::Value::as_str) == Some(deck_name))
            .and_then(|d| d.get("id").and_then(serde_json::Value::as_str))
            .map(ToString::to_string)
    });
    let deck_id = if let Some(id) = existing {
        let status = put(&agent, &format!("{gateway}/decks/{id}"), &token, &body)?;
        anyhow::ensure!(
            status == 204,
            "refresh the {deck_name} deck from the file: {status}"
        );
        id
    } else {
        let (status, body) = post(&agent, &format!("{gateway}/decks"), Some(&token), &body)?;
        anyhow::ensure!(status == 200, "save the {deck_name} deck: {status} {body}");
        field(&body, "deck_id")?
    };

    // The table. Two chairs is the one-tap game against the house; more is a
    // room whose other chairs go to the AI, which is what starts it.
    let create = if seats == 2 {
        serde_json::json!({ "deck_id": deck_id, "mode": "ai" })
    } else {
        serde_json::json!({ "deck_id": deck_id, "seats": seats, "name": "dev table" })
    };
    let (status, body) = post(
        &agent,
        &format!("{gateway}/lobby/games"),
        Some(&token),
        &create,
    )?;
    anyhow::ensure!(status == 200, "open a table: {status} {body}");
    let game_id = field(&body, "game_id")?;
    let seat_token = field(&body, "seat_token")?;

    arrange_room(&agent, gateway, &token, &game_id, seats, ai, teams)?;

    let opponents = seats - 1;
    println!("table ready: {seats} chairs, {opponents} × {ai} AI, playing {deck_name}");
    if !teams.is_empty() {
        let sides: Vec<String> = teams.iter().map(ToString::to_string).collect();
        println!("sides, in seat order: {}", sides.join(", "));
    }
    if !play {
        println!(
            "\nBAYLEE_GATEWAY={gateway} \\\n  BAYLEE_GAME={game_id} \\\n  BAYLEE_SEAT_TOKEN={seat_token} \\\n  cargo run -p baylee-client"
        );
        return Ok(());
    }
    let status = std::process::Command::new("cargo")
        .args(["run", "-p", "baylee-client"])
        .current_dir(root)
        .env("BAYLEE_GATEWAY", gateway)
        .env("BAYLEE_GAME", &game_id)
        .env("BAYLEE_SEAT_TOKEN", &seat_token)
        .status()?;
    anyhow::ensure!(status.success(), "the client exited with {status}");
    Ok(())
}

/// Counts how many reference scripts the transcoder reads in full.
///
/// The number is the honest ceiling on what `codegen` can generate from the
/// rules reference: a script it refuses becomes an ordinary stub, so this is
/// also the list of rules worth adding next.
fn transcode_report(
    root: &Path,
    scripts_dir: &Path,
    samples: usize,
    stubs: bool,
    reason: Option<&str>,
    top: usize,
) -> anyhow::Result<()> {
    let dir = scripts_root(root, scripts_dir);
    let cache = root.join("data/scryfall-cache");
    let agent = ureq::Agent::new_with_defaults();
    let mut cats = catalog::SubtypeCatalogs {
        creature: scryfall::fetch_catalog("creature-types", &agent, &cache)?,
        artifact: scryfall::fetch_catalog("artifact-types", &agent, &cache)?,
        enchantment: scryfall::fetch_catalog("enchantment-types", &agent, &cache)?,
        land: scryfall::fetch_catalog("land-types", &agent, &cache)?,
        planeswalker: scryfall::fetch_catalog("planeswalker-types", &agent, &cache)?,
        spell: scryfall::fetch_catalog("spell-types", &agent, &cache)?,
    };
    cats.normalize();
    let mut files = Vec::new();
    collect_scripts(&dir, &mut files)?;
    files.sort();
    let wanted: Option<(BTreeSet<String>, usize)> = if stubs {
        Some(stub_names(&root.join("crates/baylee-cards/src/cards"))?)
    } else {
        None
    };
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
        if scriptgen::transcode(&script, &cats).is_some() {
            read += 1;
        } else {
            refused += 1;
            let cause = refusal_cause(&script, &cats);
            let wanted_cause = reason.is_none_or(|want| cause.contains(want));
            *causes.entry(cause).or_insert(0usize) += 1;
            if shown < samples && wanted_cause {
                shown += 1;
                println!("--- refused: {}\n{text}", path.display());
            }
        }
    }
    let total = read + refused;
    println!(
        "transcoder: {read} / {total} scripts read in full ({}%)",
        (read * 100).checked_div(total).unwrap_or(0)
    );
    if let Some((_, cards)) = &wanted {
        println!(
            "  over our own stubs: {} of {cards} have a reference script",
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

/// Chooses the cards that would teach the engine the most, and says what
/// each one asks for.
///
/// One refused reference script, reduced to the mechanics it uses.
struct Card {
    name: String,
    atoms: Vec<String>,
}

/// Picks `count` cards, each time taking the one whose still-uncovered atoms
/// block the most other scripts. `max_new` keeps a card that would drag in a
/// dozen unrelated mechanics out of the plan — it is a worklist, and an item
/// nobody can finish is not one.
fn greedy_pick(
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
fn coverage_set(
    root: &Path,
    scripts_dir: &Path,
    count: usize,
    max_new: usize,
) -> anyhow::Result<()> {
    let dir = scripts_root(root, scripts_dir);
    let cache = root.join("data/scryfall-cache");
    let agent = ureq::Agent::new_with_defaults();
    let mut cats = catalog::SubtypeCatalogs {
        creature: scryfall::fetch_catalog("creature-types", &agent, &cache)?,
        artifact: scryfall::fetch_catalog("artifact-types", &agent, &cache)?,
        enchantment: scryfall::fetch_catalog("enchantment-types", &agent, &cache)?,
        land: scryfall::fetch_catalog("land-types", &agent, &cache)?,
        planeswalker: scryfall::fetch_catalog("planeswalker-types", &agent, &cache)?,
        spell: scryfall::fetch_catalog("spell-types", &agent, &cache)?,
    };
    cats.normalize();
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
        if scriptgen::transcode(&script, &cats).is_some() {
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
fn stub_names(cards_dir: &Path) -> anyhow::Result<(BTreeSet<String>, usize)> {
    let mut out = BTreeSet::new();
    let mut cards = 0usize;
    // Through `card_files` and never `read_dir`: `cards/` is a tree now, and
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

fn collect_scripts(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
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
fn refusal_cause(script: &scriptgen::CardScript, cats: &catalog::SubtypeCatalogs) -> String {
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
    if let Some(why) = scriptgen::refusal_reason(script, cats) {
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

// ---------------------------------------------------------------------------
// cross-read: the transcoder as a second reader of a hand-written card
// ---------------------------------------------------------------------------

/// What a card's abilities look like, in the coarsest vocabulary both
/// readers can speak.
///
/// The transcoder produces Rust *source*, not a `CardDef`, so the two sides
/// cannot be compared field by field without compiling one of them. What
/// they can both state is the **shape**: how many abilities of each kind a
/// card has. That is coarse on purpose — it is the level at which a
/// disagreement is always worth reading, and below which it never is. A
/// filter written two equivalent ways would differ textually and mean the
/// same thing; a card with a triggered ability one reader never saw is a
/// card to open.
#[derive(PartialEq, Eq)]
struct Shape {
    spell: usize,
    activated: usize,
    mana: usize,
    triggered: usize,
    statics: usize,
    /// Whether the static count means anything on this side.
    ///
    /// False where the script has an `R:` line. A replacement effect comes
    /// from a static ability (CR 614.1) and is one on the card side, but at
    /// this depth an `R:` is equally likely to be "enters tapped", which is
    /// no ability at all — one prefix over two different things, and no
    /// count of the prefix can separate them.
    statics_known: bool,
    /// Everything the transcoder has no rule for at all. Counted on the
    /// hand-written side only, and never compared — it is the measure of
    /// what this report structurally cannot see.
    beyond: usize,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            spell: 0,
            activated: 0,
            mana: 0,
            triggered: 0,
            statics: 0,
            statics_known: true,
            beyond: 0,
        }
    }
}

impl Shape {
    /// The shape the hand-written card actually builds.
    ///
    /// Read over **every** face. A script is one file per card and carries
    /// both halves of a modal double-faced land, so asking face 0 alone
    /// reported all three Pathways for building one mana ability against a
    /// script that has two.
    fn of_card(def: &baylee_cards::dsl::CardDef) -> Self {
        use baylee_cards::dsl::AbilityDef as A;
        let mut out = Self::default();
        for ability in (0..def.faces.len()).flat_map(|f| def.abilities_for_face(f)) {
            match ability {
                // A modal spell and a modal triggered ability are one spell
                // and one trigger that offer a choice, not a shape of their
                // own — the corpus writes both as a single `Charm` line. That is
                // the class this report exists for: the four cards whose
                // only spell ability is `ModalSpell` were being offered a
                // cast with no mode at all, and `beyond` is where that hid.
                A::Spell { .. } | A::ModalSpell { .. } => out.spell += 1,
                A::Triggered { .. } | A::ModalTriggered { .. } => out.triggered += 1,
                // `ActivatedConditional` is the same ability with a
                // condition on when it may be activated, and the transcoder
                // has one macro for both. Folding them is what keeps this
                // from reporting every equipment in the pool — and the
                // `mana_ability` flag has to travel with it, or Mox Opal's
                // metalcraft mana and a verge land's second colour read as
                // activations.
                A::Activated { mana_ability, .. }
                | A::ActivatedConditional { mana_ability, .. } => {
                    if *mana_ability {
                        out.mana += 1;
                    } else {
                        out.activated += 1;
                    }
                }
                // CR 614.1: a replacement effect is generated by a static
                // ability, and the corpus writes one as the `S:` line it is.
                A::Static(_) | A::Replacement(_) => out.statics += 1,
                // A loyalty ability *is* an activated ability (CR 606.1),
                // and the corpus writes one as an `A:AB$` line whose cost adds or
                // removes loyalty counters. Leaving it in `beyond` reported
                // every planeswalker in the pool as a card that builds
                // nothing.
                A::Loyalty { .. } => out.activated += 1,
                _ => out.beyond += 1,
            }
        }
        // CR 305.6: a land with a basic land type has the matching mana
        // ability intrinsically, and it is printed nowhere — Taiga's text
        // box holds reminder text and the corpus writes no `A:` line for it. The
        // DSL has no intrinsic anything, so the card spells the ability out
        // and every dual, shock and basic in the pool disagreed with a
        // script that is silent by design. One such ability per face is what
        // a type line grants, however many types it names.
        for face in def.faces {
            use baylee_core::generated::subtypes::land;
            const BASIC: [baylee_core::ids::SubtypeId; 5] = [
                land::PLAINS,
                land::ISLAND,
                land::SWAMP,
                land::MOUNTAIN,
                land::FOREST,
            ];
            if face.subtypes.iter().any(|s| BASIC.contains(s)) {
                out.mana = out.mana.saturating_sub(1);
            }
        }
        out
    }

    /// The shape the **script** has, read off the parser's own line kinds.
    ///
    /// This is the second reading depth and the one that reaches the whole
    /// hand-written pool. [`Self::of_body`] needs the transcoder to have
    /// read every clause, which by construction it almost never has here —
    /// a card is hand-written precisely *because* a reader could not write
    /// it, so only 22 of 207 overlap and that number shrinks as the
    /// transcoder grows. Counting lines needs no such thing: a script that
    /// is refused for one unclaimed parameter still says plainly how many
    /// abilities it has and of what kind.
    ///
    /// Every classification here is the parser's or the transcoder's own —
    /// `rules` is already typed `A`/`T`/`S`/`R`, `AB$` versus `SP$` is the
    /// same test [`scriptgen`] makes to pick between `activated!` and
    /// `spell!`, and mana-ness comes from [`scriptgen::apis_used`] following
    /// the `SubAbility$` chain. Nothing is re-derived with a regex: a second
    /// classifier living here is exactly how `transcode-report` came to rank its
    /// refusal causes wrongly.
    /// `None` where the script cannot be counted at all — see the rule under
    /// "one unread clause" below.
    fn of_script(script: &baylee_cards_codegen::scriptgen::CardScript) -> Result<Self, String> {
        use baylee_cards_codegen::scriptgen;
        let mut out = Self::default();
        for (kind, body) in &script.rules {
            match kind {
                'A' if body.starts_with("AB$") => {
                    // CR 605.1a: a mana ability is one that could add mana
                    // and does nothing else, which is exactly "the chain
                    // reaches no API but `Mana`". That makes the answer
                    // depend on every API in the chain being one the
                    // transcoder has a rule for: Exotic Orchard's
                    // `ManaReflected` is a mana ability nothing here can
                    // recognise as one, and guessing would have reported the
                    // card for building the ability it prints.
                    let apis = scriptgen::apis_used(body, &script.svars);
                    if apis.is_empty() {
                        return Err("api:<unparsed>".to_string());
                    }
                    if let Some(api) = apis.iter().find(|api| !scriptgen::is_supported_api(api)) {
                        return Err(format!("api:{api}"));
                    }
                    if apis.iter().all(|api| api == "Mana") {
                        out.mana += 1;
                    } else {
                        out.activated += 1;
                    }
                }
                'A' if body.starts_with("SP$") => out.spell += 1,
                'T' => out.triggered += 1,
                // Two `S:` modes are costs rather than abilities, and the
                // DSL carries both on the face — `alternative_costs` for the
                // first (Force of Will's pitch) and `mandatory_additional_
                // costs`/`additional_costs` for the second (Spirit Water
                // Revival's waterbend). Without this the five free spells in
                // the pool all reported a static they do not have.
                'S' if body.starts_with("Mode$ AlternativeCost")
                    || body.starts_with("Mode$ OptionalCost") => {}
                'S' => out.statics += 1,
                'R' => out.statics_known = false,
                // `A:ST$ …`, a bare `A:Mode$ …`: a line shape this does not
                // model, and an uncounted line is an uncountable script.
                _ => {
                    let head = body.split(['$', ' ']).next().unwrap_or("?");
                    return Err(format!("line:{kind}:{head}"));
                }
            }
        }
        for keyword in &script.keywords {
            let word = keyword.split([' ', ':']).next().unwrap_or_default();
            match word {
                // Two keywords are activated abilities wearing a keyword's
                // clothes: equip (CR 702.6a) and cycling (CR 702.29a). The
                // DSL has no bit for either, so a hand-written card spells
                // them as the activations they are, and without this every
                // equipment in the pool disagreed.
                "Equip" | "Cycling" => out.activated += 1,
                // And one is a *spell* wearing them: enchant is a static
                // ability of the Aura spell (CR 702.5b), so a hand-written
                // Aura says it as the `spell!` that targets what it will
                // enchant and arrives attached to it.
                "Enchant" => out.spell += 1,
                // And one is a static ability wearing them: "you may choose
                // not to untap" has no parameters, so the reference files it
                // as a keyword, while the DSL writes it as the
                // `static_ability!` CR 613.11 makes it.
                _ if scriptgen::keyword_static_of(keyword).is_some() => out.statics += 1,
                // And one is no ability at all: `K:etbCounter:P1P1:2` is a
                // replacement effect the DSL carries on the *face*, as
                // `enter_modifiers`. `Shape` counts abilities, so it counts
                // this as nothing — which is the same nothing a hand-written
                // card's face contributes, and so is symmetric rather than
                // merely quiet.
                _ if scriptgen::keyword_enter_modifier_of(keyword, &script.svars).is_some() => {}
                // **One unread clause and the script is not counted.** This
                // is the transcoder's own honesty rule at a shallower depth,
                // and it is what separates a report from a guess. The corpus
                // folds whole abilities into keywords whose payload is an
                // `SVar` chain — `K:ETBReplacement:Copy:DBCopy` is where
                // Progenitor Mimic's granted upkeep trigger lives, and no
                // count of `T:` lines can see it. A card whose script says
                // something this cannot read is a counted skip, never a
                // finding: reporting Progenitor Mimic for building a trigger
                // its script "does not have" would be this tool inventing a
                // defect out of its own blind spot.
                _ if scriptgen::keyword_const_of(keyword).is_none() => {
                    return Err(format!("kw:{word}"));
                }
                _ => {}
            }
        }
        Ok(out)
    }

    /// The five counts, for a message.
    fn tell(&self) -> String {
        let statics = if self.statics_known {
            format!("{} static", self.statics)
        } else {
            "an uncounted number of statics".to_string()
        };
        format!(
            "{} spell, {} activated, {} mana, {} triggered, {statics}",
            self.spell, self.activated, self.mana, self.triggered
        )
    }

    /// Whether the comparable counts agree.
    ///
    /// `beyond` is excluded: the hand-written side is allowed to say things
    /// the transcoder cannot. Statics are compared as **presence** and never
    /// as a count, because the two sides do not count the same unit — a
    /// printed sentence is usually several layers (CR 613.1), so Sword of
    /// Hearth and Home's one "gets +2/+2 and has protection from green and
    /// from white" is one `S:` line and three `AbilityDef::Static`s, and a
    /// count comparison reports every equipment and every anthem in the pool
    /// forever. A report nobody can read past is worth nothing.
    fn agrees_with(&self, other: &Self) -> bool {
        if (self.spell, self.activated, self.mana, self.triggered)
            != (other.spell, other.activated, other.mana, other.triggered)
        {
            return false;
        }
        if !self.statics_known || !other.statics_known {
            return true;
        }
        (self.statics > 0) == (other.statics > 0)
    }
}

/// The keyword bit a `KeywordSet::FLYING`-shaped constant names.
///
/// Read off [`KEYWORD_WORDS`] rather than written a second time: that table
/// already pairs every bit with its printed spelling, and a constant's name
/// is that spelling shouted. Two tables of the same thing is how one of them
/// goes stale.
fn keyword_bit(const_name: &str) -> Option<baylee_cards::dsl::KeywordSet> {
    let want = const_name.strip_prefix("KeywordSet::")?;
    KEYWORD_WORDS
        .iter()
        .find(|(_, word)| word.to_uppercase().replace(' ', "_") == want)
        .map(|(bit, _)| *bit)
}

/// Reads every hand-written card a second way and prints the disagreements.
#[allow(clippy::too_many_lines)] // one paragraph per skip bucket; splitting hides the census
fn cross_read(root: &Path, scripts_dir: &Path, samples: usize) -> anyhow::Result<()> {
    // A disagreement never fails this command — that is the tier's whole
    // stance. What *does* fail it is the reader losing sight of which cards
    // it is reading, because that is the failure this command has already
    // had twice in one afternoon: a retyped ownership marker read 383
    // machine-owned cards as hand-written, and `faces[0].keywords` read every
    // single-faced card as claiming no keywords at all. Both left a report
    // that ran, printed, and said nothing true.
    //
    // The population is bounded on **both** sides for exactly that reason.
    // Floors alone would have caught neither: the marker bug made the
    // hand-written population *larger* (336 rather than 207), so a minimum
    // passed it while the report filled with a rule disagreeing with itself.
    // These are the reach measured on 2026-09-17 — 245 cards read, 181 of
    // them counted, 26 transcoded in full — with room either way for
    // ordinary movement in the pool and none for a whole population
    // appearing or vanishing.
    //
    // The window is re-centred rather than widened, and it is worth saying
    // why it had to move at all: the previous one was measured at 204 cards
    // on 2026-09-11 and the hand-written pool has since grown past its
    // ceiling by ordinary work. That is the bound doing its job — it asked a
    // person to look — and the answer is a new measurement, never a ceiling
    // raised far enough not to ask again.
    const READ_FLOOR: usize = 225;
    const READ_CEILING: usize = 295;
    const COUNTED_FLOOR: usize = 165;
    const IN_FULL_FLOOR: usize = 22;

    let cache = root.join("data/scryfall-cache");
    let agent = ureq::Agent::new_with_defaults();
    let mut cats = catalog::SubtypeCatalogs {
        creature: scryfall::fetch_catalog("creature-types", &agent, &cache)?,
        artifact: scryfall::fetch_catalog("artifact-types", &agent, &cache)?,
        enchantment: scryfall::fetch_catalog("enchantment-types", &agent, &cache)?,
        land: scryfall::fetch_catalog("land-types", &agent, &cache)?,
        planeswalker: scryfall::fetch_catalog("planeswalker-types", &agent, &cache)?,
        spell: scryfall::fetch_catalog("spell-types", &agent, &cache)?,
    };
    cats.normalize();

    let index_path = root.join("data/script-index.json");
    let script_index: BTreeMap<String, String> =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap_or_default())
            .unwrap_or_default();
    if script_index.is_empty() {
        println!("note: data/script-index.json is missing or empty; run `cargo xtask codegen`");
        return Ok(());
    }

    let files = card_files(&root.join("crates/baylee-cards/src/cards"))?;
    let (mut machine, mut not_implemented, mut no_script) = (0, 0, 0);
    // Two drops that used to be anonymous. A `continue` with no
    // counter is how a reader loses a whole population and still
    // prints a report, so both are named and both are reported.
    let (mut unnamed, mut unregistered) = (0usize, 0usize);
    let (mut compared, mut agreed, mut in_full) = (0usize, 0usize, 0usize);
    let (mut shapes, mut uncountable) = (0usize, 0usize);
    let mut unreadable: BTreeMap<String, usize> = BTreeMap::new();
    let mut disagreements: Vec<String> = Vec::new();
    let mut shown = 0usize;

    for path in files.values() {
        let text = fs::read_to_string(path)?;
        // A file carrying either marker is a rule's output, and comparing a
        // rule against itself says nothing. The predicate is `stubgen`'s own
        // and not a pair of retyped literals: the first draft of this line
        // spelled the ownership marker without the backticks it is written
        // with, so all 383 machine-owned cards were read as hand-written and
        // the report's loudest finding — eleven artifact "bridges" whose
        // indestructible the transcoder had itself just written — was the
        // transcoder agreeing with itself.
        if baylee_cards_codegen::stubgen::is_machine_owned(&text) {
            machine += 1;
            continue;
        }
        // `knob` and not a literal match, for the reason CLAUDE.md gives
        // about every textual reader of this pool: a card file writes
        // `oracle_id = "…"` and this line spelled it `oracle_id: "…"`, which
        // nought of the 1365 card files have ever contained. Every card fell
        // through the `continue` below it, silently, and the report went on
        // printing — it was the population bound above that said so, which
        // is the third time a reader here has answered a question it could
        // not see.
        let Some(oracle_id) = knob(&text, "oracle_id")
            .and_then(|v| v.strip_prefix('"'))
            .and_then(|v| v.split_once('"'))
            .map(|(id, _)| id)
        else {
            unnamed += 1;
            continue;
        };
        let Some(def) = baylee_cards::by_oracle_id(oracle_id) else {
            unregistered += 1;
            continue;
        };
        if !matches!(def.coverage, baylee_cards::dsl::Coverage::Implemented) {
            not_implemented += 1;
            continue;
        }
        let Some(rel) = script_index.get(def.name()) else {
            no_script += 1;
            continue;
        };
        let script_path = scripts_root(root, scripts_dir).join(rel);
        let Ok(script_text) = fs::read_to_string(&script_path) else {
            no_script += 1;
            continue;
        };
        let script = scriptgen::parse(&script_text);
        // The transcoder is read at whichever of its two depths this script
        // reaches. Transcoding is the deeper one and needs every clause
        // claimed, which a hand-written card's script almost never offers —
        // it is hand-written *because* a reader could not write it. Parsing
        // is the shallower one and never refuses: a script stopped on one
        // unclaimed parameter still says how many abilities it has.
        let body = scriptgen::transcode(&script, &cats);
        compared += 1;
        in_full += usize::from(body.is_some());

        let mut found: Vec<String> = Vec::new();
        let mine = Shape::of_card(def);
        match Shape::of_script(&script) {
            Ok(theirs) => {
                shapes += 1;
                if !mine.agrees_with(&theirs) {
                    found.push(format!(
                        "the card builds {} and the script reads {}",
                        mine.tell(),
                        theirs.tell()
                    ));
                }
            }
            // A skip is counted *and named*. The reason is what makes this
            // half a worklist rather than a shrug: teaching `keyword_const`
            // one more row, or the transcoder one more API, is a number of
            // cards this report can then read.
            Err(reason) => {
                uncountable += 1;
                *unreadable.entry(reason).or_insert(0usize) += 1;
            }
        }
        // Keywords are the half that *is* comparable exactly: both sides
        // name a bit, and a bit is a bit. `keyword_const_of` is the
        // transcoder's own `K:` reader, and the two directions are not
        // symmetric. A bit the script prints and the card does not claim is
        // always a finding. A bit the card claims is one only if the script
        // was read *whole*: `keyword_const` has no row for daybound, so five
        // werewolves were reported for claiming the keyword their script
        // prints on its own line.
        let mut script_bits = baylee_cards::dsl::KeywordSet::EMPTY;
        let mut every_keyword_read = true;
        for line in &script.keywords {
            match scriptgen::keyword_const_of(line).and_then(keyword_bit) {
                Some(bit) => script_bits = script_bits.union(bit),
                // Read, and deliberately not a bit: a `K:` line the
                // transcoder turns into a static ability or into an
                // as-it-enters modifier says nothing about this card's
                // `KeywordSet` either way.
                None if scriptgen::keyword_static_of(line).is_some() => {}
                None if scriptgen::keyword_enter_modifier_of(line, &script.svars).is_some() => {}
                // And nor does one that is a whole ability the rules define
                // for the word. The test is the word and not whether the
                // transcoder read the line, because the question here is
                // "is this line about bits" — `K:Equip:1:Creature.Legendary`
                // is refused over its restriction and is still no bit, and
                // suppressing the comparison over it would take every
                // Equipment and every Aura in the pool out of the half of
                // `cross-read` that finds a card claiming a keyword its
                // printing never gave it.
                None if line.starts_with("Equip:") || line.starts_with("Enchant:") => {}
                None => every_keyword_read = false,
            }
        }
        // `faces[0].keywords` is the *override*, empty on every single-faced
        // card, which states its keywords once at card level. Reading it
        // directly said "the card claims it nowhere" about eleven bridges
        // that claim indestructible on their first line.
        // A script is one file per card and prints a transforming card's back
        // keywords in the same `K:` block, so the card side is the union too.
        let card_bits = (0..def.faces.len()).map(|f| def.keywords_for_face(f)).fold(
            baylee_cards::dsl::KeywordSet::EMPTY,
            baylee_cards::dsl::KeywordSet::union,
        );
        for (bit, word) in KEYWORD_WORDS {
            let card_has = card_bits.contains(*bit);
            let script_has = script_bits.contains(*bit);
            match (card_has, script_has) {
                (false, true) => {
                    found.push(format!(
                        "the script prints {word} and the card claims it nowhere"
                    ));
                }
                (true, false) if every_keyword_read => {
                    found.push(format!(
                        "the card claims {word} and the script does not print it"
                    ));
                }
                _ => {}
            }
        }
        // How a permanent enters is the one check that stays on the deep
        // path. It is written as an `R:` replacement whose payload is an
        // `SVar` chain, so counting the line says nothing at all about what
        // the line does — only a transcoding does.
        if let Some(body) = &body
            && body.enter_modifiers.len() != def.faces[0].enter_modifiers.len()
        {
            found.push(format!(
                "the card enters under {} modifier(s) and the script under {}",
                def.faces[0].enter_modifiers.len(),
                body.enter_modifiers.len()
            ));
        }

        if found.is_empty() {
            agreed += 1;
            continue;
        }
        for line in &found {
            disagreements.push(format!("{}: {line}", def.name()));
        }
        if shown < samples {
            shown += 1;
            println!(
                "--- {} ({})\n{script_text}",
                def.name(),
                script_path.display()
            );
        }
    }

    disagreements.sort();
    for line in &disagreements {
        println!("{line}");
    }
    println!(
        "cross-read: {compared} hand-written cards read twice, {agreed} agreed, {} \
         disagreed on {} point(s)",
        compared.saturating_sub(agreed),
        disagreements.len()
    );
    println!(
        "  depth: {shapes} scripts counted line by line, {in_full} of those also \
         transcoded in full; {uncountable} say something no count can read"
    );
    let mut ranked: Vec<(&String, &usize)> = unreadable.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (reason, count) in ranked.iter().take(8) {
        println!("    {count:>3}  {reason}");
    }
    println!(
        "  skipped: {machine} machine-owned, {not_implemented} not implemented, \
         {no_script} with no reference script, {unnamed} with no readable \
         `oracle_id`, {unregistered} the registry does not carry"
    );

    if !(READ_FLOOR..=READ_CEILING).contains(&compared)
        || shapes < COUNTED_FLOOR
        || in_full < IN_FULL_FLOOR
    {
        anyhow::bail!(
            "cross-read read a different pool than it can: {compared} cards read \
             (expected {READ_FLOOR}..={READ_CEILING}), {shapes} counted (floor \
             {COUNTED_FLOOR}), {in_full} transcoded in full (floor {IN_FULL_FLOOR}). \
             This is a fault in the reader, not in the cards — a comparison that skips \
             a population says nothing about it and still exits green, and one that \
             takes in a population it should not skip fills up with a rule disagreeing \
             with itself."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::printed_subtypes;
    use baylee_core::generated::subtypes;

    /// The branch the pool does not reach, and the reason it is written.
    ///
    /// [`printed_subtypes`] is the honesty gate on
    /// [`check_type_line_matches_the_printing`]: a word the catalog does not
    /// know skips the face, so what counts as "known" decides which faces are
    /// compared at all. "Time Lord" is the one subtype in the whole catalog
    /// spelled as two words, no card in this pool prints it, and deleting the
    /// longest-match pass therefore changes nothing `validate` says today —
    /// which is exactly why the pass needs a test rather than a mutant. The
    /// first Doctor Who card in the pool would otherwise read as two words
    /// that name no subtype, and the whole face would go unchecked.
    #[test]
    fn a_two_word_subtype_is_one_subtype() {
        assert_eq!(
            printed_subtypes("Legendary Creature \u{2014} Time Lord"),
            Some(vec![subtypes::creature::TIME_LORD]),
        );
    }

    /// A type line with no dash prints no subtypes, and that is an answer.
    ///
    /// Not a refusal: "this card has none" is a line worth comparing, and it
    /// is the half Raffine's Tower needed — a `TypeSet::LAND` with an empty
    /// subtype list against a printing that names three.
    #[test]
    fn a_type_line_with_no_dash_names_no_subtypes() {
        assert_eq!(printed_subtypes("Artifact"), Some(Vec::new()));
    }

    /// A word the catalog does not know refuses the whole line.
    ///
    /// The catalogs are what `codegen` builds the constants from, so an
    /// unknown word is this command's own gap and never a fact about the
    /// card. `PrintingTally::type_lines` is what keeps a refusal from being
    /// silent.
    #[test]
    fn an_unknown_word_refuses_the_line_rather_than_guessing() {
        assert_eq!(printed_subtypes("Creature \u{2014} Wizard Nonesuch"), None);
    }
}

// ------------------------------------------------------ ability lines

/// Why one ability found no sentence.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Miss {
    /// The card prints no line of that shape at all — an ability whose
    /// sentence is printed as a *keyword* (Mulldrifter's evoke sacrifice,
    /// Lightning Greaves' `Equip {0}`).
    NoLineOfThatShape,
    /// A line of the right shape is there and none of them matches what
    /// the ability says about itself. This is the bucket that matters: it
    /// is where a swapped pair and a wrong cost both land.
    NoLineThatSaysThat,
}

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
fn ledger_cmd(root: &Path, corpus: &Path, check: bool, reseed: bool) -> anyhow::Result<()> {
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

/// Does the compiled ability list line up with the printed sentences?
///
/// `docs/client.md` says codegen "can emit a per-card table of ordered
/// sentences alongside the registry", and that the `//! Oracle:` header
/// lines are "ordered to match the ability list". Nothing has ever counted
/// that, and the whole of the stack-text feature rests on it: if an ability
/// can be given the index of the sentence it came from, the client can
/// print a localized loyalty ability's actual text instead of "+1".
///
/// So this is a **measurement**, not a check. It reports how many cards
/// line up, and names every one that does not, because the shape of the
/// answer decides the shape of the table: a clean sweep means one
/// `Option<u8>` per ability, and a long tail means a hand-kept override
/// column beside it.
///
/// It matches on shape **and** on content, and the second half is not
/// optional. Shape alone cannot tell a walker's `+1` from its `−3` or a
/// `Dies` trigger from an `Enters` one, so it walks a card whose abilities
/// are in each other's places and reports it as aligned — an upper bound
/// wearing a count's clothes.
fn ability_lines(root: &Path) -> anyhow::Result<()> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    let by_name: BTreeMap<&str, &'static baylee_cards::dsl::CardDef> =
        baylee_cards::all().map(|def| (def.name(), def)).collect();

    let mut considered = 0usize;
    let mut abilities_seen = 0usize;
    let mut aligned = 0usize;
    let mut out_of_order = 0usize;
    let mut lineless = 0usize;
    let mut ambiguous = 0usize;
    let mut coinflips: Vec<String> = Vec::new();
    let mut misses: Vec<String> = Vec::new();
    let mut disorder: Vec<String> = Vec::new();
    let mut why: BTreeMap<(Miss, lines::LineShape), usize> = BTreeMap::new();

    for name in &names {
        let Some(def) = by_name.get(name.split(" // ").next().unwrap_or(name)) else {
            continue;
        };
        // A `Partial` card's abilities go on the stack like anyone else's,
        // so the table has to cover it — only a stub has nothing to say.
        if matches!(def.coverage, baylee_cards::dsl::Coverage::Unimplemented) {
            continue;
        }
        let Some(payload) = cached_printing(root, name) else {
            continue;
        };
        // **One face at a time**, because that is the unit the table has to
        // be in: a back face's abilities are its own (`abilities_for_face`)
        // and a client draws one face's text, so an index into the two
        // joined together points at the face nobody is reading. Sheoldred
        // is the card that says so — its three saga chapters are on the
        // back and were not in the walk at all.
        let texts = face_texts(&payload);
        for face in 0..def.faces.len() {
            let Some(printed) = texts.get(face) else {
                continue;
            };
            let abilities = def.abilities_for_face(face);
            let sentences: Vec<&str> = lines::sentences(printed).collect();
            // Only the abilities that can *be* a stack entry are in
            // question. A static never goes on the stack, and one printed
            // sentence is several of them by design ("gets +1/+1 and has
            // flying" is layers 7c and 6), so asking a static which
            // sentence it came from is asking the wrong question. A mana
            // ability is placed but does not use the stack (CR 605.1), so
            // it is out of this count too — see `LineShape::Mana`.
            let stackable: Vec<(usize, lines::LineShape)> = abilities
                .iter()
                .map(lines::ability_shape)
                .enumerate()
                .filter(|(_, s)| s.stackable())
                .collect();
            if stackable.is_empty() {
                continue;
            }
            considered += 1;
            abilities_seen += stackable.len();

            // The mapping itself is `lines::map`, the same code the table
            // will be generated from — a report that matched a second way
            // would be measuring itself rather than the thing that ships.
            let mapping = lines::map(abilities, printed);
            ambiguous += mapping.ambiguous;
            if mapping.ambiguous > 0 {
                coinflips.push(format!("{} ({})", def.name(), mapping.ambiguous));
            }
            let mut found_all = true;
            for (i, want) in stackable.iter().copied() {
                if mapping.lines[i].is_some() {
                    continue;
                }
                found_all = false;
                let miss = if sentences.iter().any(|l| lines::line_shape(l) == want) {
                    Miss::NoLineThatSaysThat
                } else {
                    Miss::NoLineOfThatShape
                };
                *why.entry((miss, want)).or_default() += 1;
                misses.push(format!("{}: ability {i} ({want:?}) — {miss:?}", def.name()));
            }
            // `found_all` is asked first. `in_order` says only that the
            // walk never went backwards, and an ability with no sentence at
            // all never moves it — so a card whose *first* ability is
            // lineless is perfectly "in order" and must still be counted as
            // the miss it is.
            match (found_all, mapping.in_order) {
                (false, _) => lineless += 1,
                (true, true) => aligned += 1,
                (true, false) => {
                    out_of_order += 1;
                    disorder.push(def.name().to_string());
                }
            }
        }
    }

    println!("ability lines: {considered} faces ({abilities_seen} stack-capable abilities)");
    println!("  {aligned} line up sentence-for-ability, in printed order");
    println!("  {out_of_order} find every ability a sentence, but not in the code's order");
    println!("  {lineless} have an ability no sentence fits");
    println!("  {ambiguous} abilities fit more than one sentence equally well");
    if !coinflips.is_empty() {
        println!("    {}", coinflips.join(", "));
    }
    println!("\nwhy an ability found no sentence:");
    let mut pairs: Vec<_> = why.into_iter().collect();
    pairs.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for ((miss, want), n) in &pairs {
        println!("  {want:?}: {miss:?} — {n}");
    }
    println!("\nout of order ({}):", disorder.len());
    for card in &disorder {
        println!("  {card}");
    }
    println!("\nabilities with no sentence ({}):", misses.len());
    for line in &misses {
        println!("  {line}");
    }
    Ok(())
}

/// Renders `crates/baylee-cards/src/generated_names.rs`: the pool's names
/// placed in a perfect hash, so resolving one costs a hash and a string
/// compare instead of a walk over 1365 cards.
///
/// The names come from the pool **compiled into this binary** — the same
/// source stage 4 reads and for the same reason: `CardDef::name()` is what
/// `by_name` has to answer for, so taking the names from anywhere else
/// would build a table about a different set of strings than the one being
/// looked up. Two-phase like stage 4, and `codegen --check` is the guard.
fn render_name_table() -> anyhow::Result<String> {
    let entries: Vec<(&str, u32)> = baylee_cards::all()
        .map(|def| (def.name(), def.index.get()))
        .collect();
    Ok(names::render(&entries)?)
}

/// Renders `crates/baylee-cards/src/generated_tokens.rs`: the token ledger.
///
/// Reads the table it is about to rewrite, which is what makes the ids
/// append-only, and then adds whatever is new — the same shape
/// `xtask ledger` has for the card index, and safe for the same reason:
/// assignment only ever appends, and the build is what checks what came out.
///
/// The hand-written half is found by reading `tokens.rs` for the statics it
/// declares, with a floor under how many it must find. A textual reader of
/// this pool that answers an empty list is not a hypothetical — eleven of
/// them have now been caught doing it — and here an empty list would not
/// fail, it would quietly report every hand-written token as an orphan.
fn render_token_ledger(root: &Path) -> anyhow::Result<String> {
    /// The fourteen that existed when the ledger was seeded. The list only
    /// grows, so anything under this is a reader that has stopped reading.
    const HAND_WRITTEN_FLOOR: usize = 14;

    let tokens = root.join("crates/baylee-cards/src/tokens.rs");
    let text = fs::read_to_string(&tokens)?;
    let hand: Vec<String> = text
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub static ")?
                .strip_suffix(": TokenDef = TokenDef {")
                .map(str::to_string)
        })
        .collect();
    anyhow::ensure!(
        hand.len() >= HAND_WRITTEN_FLOOR,
        "read {} hand-written tokens out of {}, expected at least {HAND_WRITTEN_FLOOR}",
        hand.len(),
        tokens.display()
    );

    let path = root.join("crates/baylee-cards/src/generated_tokens.rs");
    let existing = match fs::read_to_string(&path) {
        Ok(text) => tokenledger::parse(&text)?,
        // No ledger at all is the first run, and only the first run: every
        // later one has the file this is about to write.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    let before = existing.len();
    let entries = tokenledger::assign(existing, &hand, &[])?;
    println!(
        "token ledger: {} entries ({} new), {} hand-written",
        entries.len(),
        entries.len() - before,
        hand.len()
    );
    Ok(tokenledger::render(&entries))
}

/// Two cards in the pool printing the same English name.
///
/// A name is what a deck list carries, so a name claimed twice is a deck row
/// with no answer: `decks::by_name` has to pick one and whichever it picks is
/// wrong for somebody. `codegen` already refuses to build the name table over
/// such a pair — the perfect hash cannot separate two identical keys — but
/// that failure arrives while generating rather than while reading, and this
/// command is the one a person runs to ask whether the pool is sound.
fn check_no_name_is_claimed_twice(problems: &mut usize) {
    let mut seen: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for def in baylee_cards::all() {
        seen.entry(def.name()).or_default().push(def.oracle_id);
    }
    for (name, oracle_ids) in seen.iter().filter(|(_, ids)| ids.len() > 1) {
        eprintln!(
            "{name}: {} cards print this name ({})",
            oracle_ids.len(),
            oracle_ids.join(", ")
        );
        *problems += 1;
    }
}
