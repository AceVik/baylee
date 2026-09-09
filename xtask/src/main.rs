//! xtask — baylee development tasks (codegen, card explanation, …).

use baylee_cards_codegen::{
    acceptance, catalog, forge, forgegen, landgen, layout, ledger, scryfall, stubgen,
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
    /// Regenerate subtype constants, card stubs, registry, and the forge index.
    Codegen {
        /// Verify generated files are up to date instead of writing (CI).
        #[arg(long)]
        check: bool,
        /// Path to the forge-reference cardsfolder.
        #[arg(
            long,
            default_value = "../mtg/forge-reference/forge-gui/res/cardsfolder"
        )]
        forge: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
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
    /// Report how much of the forge-reference corpus the transcoder reads.
    ForgeReport {
        /// Path to the forge-reference cardsfolder.
        #[arg(
            long,
            default_value = "../mtg/forge-reference/forge-gui/res/cardsfolder"
        )]
        forge: PathBuf,
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
    },
    /// Rank the land sentences `landgen` cannot read yet.
    ///
    /// The counterpart to `forge-report`, and worth its own command for the
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
        /// Path to the forge-reference cardsfolder.
        #[arg(
            long,
            default_value = "../mtg/forge-reference/forge-gui/res/cardsfolder"
        )]
        forge: PathBuf,
    },
    /// Show Scryfall + forge-reference data for a card side by side.
    Explain {
        /// Exact card name.
        #[arg(long)]
        name: String,
        /// Path to the forge-reference cardsfolder.
        #[arg(
            long,
            default_value = "../mtg/forge-reference/forge-gui/res/cardsfolder"
        )]
        forge: PathBuf,
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
        /// Path to the forge-reference cardsfolder.
        #[arg(
            long,
            default_value = "../mtg/forge-reference/forge-gui/res/cardsfolder"
        )]
        forge: PathBuf,
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
    /// `landgen` or `forgegen` reaches every card that rule wrote, at once.
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
            forge,
            cache,
        } => codegen(&root, check, &forge, &cache),
        Cmd::PoolDump { out } => pool_dump(&out),
        Cmd::ForgeReport {
            forge,
            samples,
            stubs,
            reason,
        } => forge_report(&root, &forge, samples, stubs, reason.as_deref()),
        Cmd::LandReport {
            samples,
            worklist,
            tail,
            cache,
        } => land_report(&root, &cache, samples, worklist.as_deref(), tail),
        Cmd::CoverageSet {
            count,
            max_new,
            forge,
        } => coverage_set(&root, &forge, count, max_new),
        Cmd::Explain { name, forge, cache } => explain(&root, &name, &forge, &cache),
        Cmd::CardBatch {
            cards,
            out,
            forge,
            cache,
        } => card_batch(&root, cards.as_deref(), &out, &forge, &cache),
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
    forge: Option<&forgegen::ForgeLookup>,
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
    let ledger_path = root.join("data/card-index.tsv");
    let mut ledger =
        ledger::IndexLedger::parse(&fs::read_to_string(&ledger_path).unwrap_or_default())?;
    let before = ledger.entries().len();
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

    let mut stubs = Vec::with_capacity(names.len());
    for name in &names {
        let card = scryfall::fetch_named(name, agent, cache)?;
        let oracle_id = card.oracle_id.clone().unwrap_or_default();
        let index = ledger.assign(&oracle_id, &card.name);
        let (info, content) = stubgen::render_stub(&card, index, cats, forge, &cycles)?;
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
    if ledger.entries().len() > before {
        println!(
            "card-index ledger: {} new index/indices assigned",
            ledger.entries().len() - before
        );
    }
    let slots = ledger.slots();
    write_or_check(check, &ledger_path, &ledger.render(), changed)?;
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated.rs"),
        &stubgen::render_registry(&stubs, slots),
        changed,
    )?;
    Ok(())
}

fn codegen(root: &Path, check: bool, forge_dir: &Path, cache: &Path) -> anyhow::Result<()> {
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

    // 2. forge-reference index. Built before the stubs, because a stub is
    //    transcoded from the rules reference when one is checked out locally
    //    (read as an automated lookup, never copied).
    let forge_dir = root.join(forge_dir);
    let lookup = if forge_dir.exists() {
        let index = forge::build_index(&forge_dir)?;
        write_or_check(
            check,
            &root.join("data/forge_index.json"),
            &serde_json::to_string_pretty(&index)?,
            &mut changed,
        )?;
        println!("forge index: {} scripts", index.len());
        Some(forgegen::ForgeLookup::new(forge_dir.clone(), index))
    } else {
        println!(
            "note: forge-reference not found at {}, skipping index",
            forge_dir.display()
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

/// Builds per-card task packages (stub + forge script + exemplar + prompt).
fn card_batch(
    root: &Path,
    cards: Option<&str>,
    out: &Path,
    forge_dir: &Path,
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
    let forge_index: BTreeMap<String, String> = serde_json::from_str(
        &fs::read_to_string(root.join("data/forge_index.json")).unwrap_or_default(),
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
        // 2. Forge script (ground truth).
        let mut has_forge = false;
        if let Some(rel) = forge_index.get(name) {
            let script = root.join(forge_dir).join(rel);
            if script.exists() {
                fs::write(dir.join("FORGE.txt"), fs::read_to_string(script)?)?;
                has_forge = true;
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
            card_prompt(name, &rel, &dir, has_forge),
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
fn card_prompt(name: &str, file: &str, package: &Path, has_forge: bool) -> String {
    let prompt = format!(
        "# Implement `{name}` in this repository\n\n\
         Edit exactly one file: `{file}`.\n\
         Touch nothing else — not `src/generated.rs`, not `src/cards/mod.rs`,\n\
         not another card, not the DSL.\n\n\
         Read first, in this order:\n\
         - `crates/baylee-cards/AGENTS.md` — the playbook you are bound by.\n\
         - `docs/card-dsl.md` — the authoring contract and the full vocabulary.\n\
         {forge_line}\
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
        forge_line = if has_forge {
            format!(
                "- `{}/FORGE.txt` — the forge-reference script; rules ground truth.\n",
                package.display()
            )
        } else {
            "There is no forge-reference script for this card. The oracle text in \
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

/// Extracts the first `"`-quoted value after `key` (e.g. `name: "…"`).
fn quoted_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let start = content.find(key)? + key.len();
    let rest = &content[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// All `mana_cost:` literals in the file (one per face), normalized:
/// `{0}` and `ManaCost::ZERO` are the same thing.
fn code_costs(content: &str) -> Vec<String> {
    content.split("face! {").skip(1).map(face_cost).collect()
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
    let Some(pos) = face.find("mana_cost: ") else {
        return NONE.to_string();
    };
    let rest = &face[pos + "mana_cost: ".len()..];
    let Some(rest) = rest.strip_prefix("baylee_core::mana!(\"") else {
        return NONE.to_string();
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

    let Some(face) = content.find("faces: &[").map(|pos| &content[pos..]) else {
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
    // is skipped, because a `FaceDef` there carries the cost the face is
    // actually cast for: Ghastly Mimicry's `mana_cost` is its **disturb**
    // cost, which Scryfall writes in the oracle text and not in
    // `card_faces[1].mana_cost`. That is why this check would not have found
    // the `{5}{U}` it was built at — [`check_oracle_matches_the_printing`]
    // did, by comparing the sentence.
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
    if printed_costs.len() == code_costs.len() {
        for (at, (printed, code)) in printed_costs.iter().zip(&code_costs).enumerate() {
            if at > 0 && printed.is_empty() {
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
        ("power", "power: Some("),
        ("toughness", "toughness: Some("),
        ("loyalty", "loyalty: Some("),
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
    /// Cards whose printing states a target count of "up to".
    targets: usize,
}

/// The floor under each count in [`PrintingTally`].
///
/// Absolute numbers rather than a fraction of whatever happened to be on
/// disk, because `data/scryfall-cache` is **tracked**: a fresh checkout has
/// the same payloads CI does, so there is no honest reason for the count to
/// drop. Each is the measured number with slack for cards leaving the pool.
/// The shape is `baylee_cards::lints`' own — "the sweep is not reaching the
/// pool" — and it exists for the same reason: a checker that silently stops
/// checking reports a clean pool.
/// Measured 2026-09-09 over a pool of 1365: **1365** payloads, 7 loyalty,
/// 1365 identity, 49 keyword, 361 mana, **1365** oracle, 1370 cost. Payloads,
/// identity and oracle are now every card in the pool rather than the 1263
/// the lookup used to find, so the floor under them is a real bound and not a
/// record of a gap: nothing but a card leaving the pool can move it down.
///
/// Cost is 1370 rather than 1365 + 109, and the arithmetic is exact: seven
/// back faces in the pool print a mana cost, two of those cards (Emeritus of
/// Woe and Twining Twins, both adventures) are one `FaceDef` here against
/// Scryfall's two and are skipped whole, so 1363 fronts + 7 backs = 1370.
/// The other 102 back faces are transform sides that print no cost at all —
/// a `FaceDef` there carries whatever the face is really cast for, and
/// comparing it to an empty string would fail every one of them.
///
/// The two that did not move are the two the new cards had nothing to add
/// to. Loyalty is 7 and the pool holds exactly seven planeswalker faces, so
/// that check was already whole. Mana is 361 because 97 of the pool's 109
/// multi-faced files are still `// GENERATED STUB`, and a stub writes no
/// mana ability for the check to read.
const PRINTING_FLOOR: PrintingTally = PrintingTally {
    payloads: 1300,
    loyalty: 6,
    identity: 1300,
    keywords: 45,
    mana: 340,
    oracle: 1300,
    costs: 1340,
    targets: 10,
};

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

fn check_header_matches_code(slug: &str, content: &str, problems: &mut usize) {
    // First header line: `//! <Name> — <cost or "(no cost)"> — <types>`.
    let Some(header) = content.lines().find(|l| l.starts_with("//! ")) else {
        println!("{slug}: no header line");
        *problems += 1;
        return;
    };
    let header = &header[4..];
    let mut parts = header.splitn(3, " — ");
    let (head_name, head_cost) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));

    // The first face's name (token statics can appear before CARD, so
    // anchor on the `faces` field). MDFC headers read "Front // Back":
    // either side may headline the file's first face.
    let code_name = content
        .find("faces: &[")
        .and_then(|pos| quoted_value(&content[pos..], "name: \""));
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
    for (label, key) in [
        ("Scryfall ID", "scryfall_id: \""),
        ("Oracle ID", "oracle_id: \""),
    ] {
        let header_has = content
            .lines()
            .find(|l| l.starts_with("//!") && l.contains(label))
            .and_then(|l| l.split(&format!("{label}: ")).nth(1))
            .map(|v| v.split([' ', '|']).next().unwrap_or("").trim());
        let code_value = quoted_value(content, key);
        if let (Some(h), Some(c)) = (header_has, code_value)
            && h != c
        {
            println!("{slug}: header {label} {h:?} != code {c:?}");
            *problems += 1;
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
        let Some(next) = with_oracle_header(&text, &printed_text(&payload)) else {
            continue;
        };
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
    ("Bleachbone Verge", "an ActivationCondition, not a filter"),
    ("Mox Opal", "metalcraft is an ActivationCondition"),
    ("Fierce Guardianship", "an AlternativeCost condition"),
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
    let filters_you = built.contains("ControlledByYou");
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
fn strip_reminders(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
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
                is_stub || content.contains("coverage: Coverage::"),
            ),
        ] {
            if !check.1 {
                println!("{slug}: missing {}", check.0);
                problems += 1;
            }
        }
        check_header_matches_code(&slug, &content, &mut problems);
        check_search_tapped_matches_text(&slug, &content, &mut problems);
        let def = by_name.get(name.split(" // ").next().unwrap_or(name));
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
        }
        check_code_matches_the_printing(&slug, &content, &payload, &mut tally, &mut problems);
        check_oracle_matches_the_printing(&slug, &content, &payload, &mut tally, &mut problems);
        check_target_counts_match_the_printing(
            &slug,
            &content,
            &payload,
            &mut tally,
            &mut problems,
        );
    }
    // Before the bail, not after it: a floor that failed is only readable
    // beside the counts that failed it.
    println!(
        "validate: against the printings \u{2014} {} payloads, {} loyalty, {} identity, \
         {} keyword, {} mana, {} oracle, {} cost, {} target count",
        tally.payloads,
        tally.loyalty,
        tally.identity,
        tally.keywords,
        tally.mana,
        tally.oracle,
        tally.costs,
        tally.targets
    );
    check_printing_floors(&tally, &mut problems);
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
        ("target count", tally.targets, PRINTING_FLOOR.targets),
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

fn explain(root: &Path, name: &str, forge_dir: &Path, cache: &Path) -> anyhow::Result<()> {
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
    let index_path = root.join("data/forge_index.json");
    if index_path.exists() {
        let index: BTreeMap<String, String> =
            serde_json::from_str(&fs::read_to_string(&index_path)?)?;
        if let Some(rel) = index.get(name) {
            let script = root.join(forge_dir).join(rel);
            println!("== forge-reference ({}) ==", script.display());
            println!("{}", fs::read_to_string(&script).unwrap_or_default());
        } else {
            println!("forge-reference: no script found for {name:?}");
        }
    } else {
        println!("note: data/forge_index.json missing; run `cargo xtask codegen`");
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

/// Counts how many forge-reference scripts the transcoder reads in full.
///
/// The number is the honest ceiling on what `codegen` can generate from the
/// rules reference: a script it refuses becomes an ordinary stub, so this is
/// also the list of rules worth adding next.
fn forge_report(
    root: &Path,
    forge_dir: &Path,
    samples: usize,
    stubs: bool,
    reason: Option<&str>,
) -> anyhow::Result<()> {
    let dir = root.join(forge_dir);
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
    let wanted: Option<BTreeSet<String>> = if stubs {
        Some(stub_names(&root.join("crates/baylee-cards/src/cards"))?)
    } else {
        None
    };
    let (mut read, mut refused) = (0usize, 0usize);
    let mut causes: BTreeMap<String, usize> = BTreeMap::new();
    let mut shown = 0usize;
    for path in &files {
        let text = fs::read_to_string(path)?;
        if let Some(wanted) = &wanted {
            let name = text
                .lines()
                .find_map(|l| l.strip_prefix("Name:"))
                .unwrap_or_default()
                .trim();
            if !wanted.contains(name) {
                continue;
            }
        }
        let script = forgegen::parse(&text);
        if forgegen::transcode(&script, &cats).is_some() {
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
        "forge transcoder: {read} / {total} scripts read in full ({}%)",
        (read * 100).checked_div(total).unwrap_or(0)
    );
    let mut ranked: Vec<(&String, &usize)> = causes.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1));
    println!("what the refused scripts need next:");
    for (cause, n) in ranked.into_iter().take(30) {
        println!("  {n:>6}  {cause}");
    }
    Ok(())
}

/// Chooses the cards that would teach the engine the most, and says what
/// each one asks for.
///
/// One refused forge script, reduced to the mechanics it uses.
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
fn coverage_set(root: &Path, forge_dir: &Path, count: usize, max_new: usize) -> anyhow::Result<()> {
    let dir = root.join(forge_dir);
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
        let script = forgegen::parse(&text);
        let atoms = forgegen::atoms(&script);
        if forgegen::transcode(&script, &cats).is_some() {
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
fn stub_names(cards_dir: &Path) -> anyhow::Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    // Through `card_files` and never `read_dir`: `cards/` is a tree now, and
    // a non-recursive walk over it would find nothing at all and report an
    // empty worklist as an answer rather than as a failure.
    for path in card_files(cards_dir)?.values() {
        let text = fs::read_to_string(path)?;
        if !text.contains("// GENERATED STUB") {
            continue;
        }
        // The header's first line is `//! <name> — <cost> — <types>`.
        if let Some(head) = text.lines().next().and_then(|l| l.strip_prefix("//! ")) {
            let name = head.split(" \u{2014} ").next().unwrap_or(head).trim();
            out.insert(name.to_string());
        }
    }
    Ok(out)
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
fn refusal_cause(script: &forgegen::ForgeScript, cats: &catalog::SubtypeCatalogs) -> String {
    if let Some(line) = script.unknown_lines.first() {
        let head = line.split(':').next().unwrap_or(line);
        return format!("unmodelled line kind `{head}:`");
    }
    for line in &script.keywords {
        if forgegen::keyword_const_of(line).is_none() {
            let head = line.split(':').next().unwrap_or(line);
            let head = head.split(' ').next().unwrap_or(head);
            return format!("keyword `{head}`");
        }
    }
    // Ask the transcoder before guessing. It knows which line it stopped
    // on and why; re-reading the script here only knows what *this* function
    // recognises, which is how every unexplained refusal used to be filed
    // under a label that named the wrong work.
    if let Some(why) = forgegen::refusal_reason(script, cats) {
        return why;
    }
    for (kind, spec) in &script.rules {
        for api in forgegen::apis_used(spec, &script.svars) {
            if !forgegen::is_supported_api(&api) {
                return format!("effect `{api}`");
            }
        }
        if *kind == 'R' {
            return "replacement effect (R:)".to_string();
        }
    }
    "refused with no reason recorded".to_string()
}
