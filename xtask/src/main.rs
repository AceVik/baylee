//! xtask — baylee development tasks (codegen, card explanation, …).

use baylee_cards_codegen::{acceptance, catalog, forge, forgegen, ledger, scryfall, stubgen};
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
        /// Only these cards (comma-separated names); default: all
        /// unimplemented acceptance cards.
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
    let mut stubs = Vec::with_capacity(names.len());
    for name in &names {
        let card = scryfall::fetch_named(name, agent, cache)?;
        let oracle_id = card.oracle_id.clone().unwrap_or_default();
        let index = ledger.assign(&oracle_id, &card.name);
        let (info, content) = stubgen::render_stub(&card, index, cats, forge)?;
        let stub_path = root.join(format!("crates/baylee-cards/src/cards/{}.rs", info.slug));
        // Implemented cards are hand-owned: only touch files that are
        // missing or still carry the GENERATED STUB marker.
        let implemented = fs::read_to_string(&stub_path)
            .is_ok_and(|existing| !existing.contains("// GENERATED STUB"));
        if implemented {
            if check {
                println!("skip (implemented): {}", info.slug);
            }
            stubs.push(info);
            continue;
        }
        write_or_check(check, &stub_path, &content, changed)?;
        stubs.push(info);
    }
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
    let wanted: Vec<String> = if let Some(list) = cards {
        list.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        names
            .iter()
            .filter(|name| {
                let path = root.join(format!(
                    "crates/baylee-cards/src/cards/{}.rs",
                    front_face_slug(name)
                ));
                // `// GENERATED STUB` and not `Coverage::Unimplemented`. A
                // stub does not write that line at all — `CardDef::DEFAULT`
                // is already `Unimplemented`, and restating a default is the
                // one thing the card DSL forbids outright. Filtering on it
                // matched nothing in the whole pool, so this command's
                // default selection silently prepared zero packages.
                fs::read_to_string(&path).is_ok_and(|c| c.contains("// GENERATED STUB"))
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
        let stub_path = root.join(format!("crates/baylee-cards/src/cards/{slug}.rs"));
        let stub = fs::read_to_string(&stub_path)?;
        fs::write(dir.join("STUB.rs"), &stub)?;
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
        let exemplar_path = root.join(format!("crates/baylee-cards/src/cards/{exemplar}.rs"));
        if exemplar_path.exists() {
            fs::write(dir.join("EXEMPLAR.rs"), fs::read_to_string(exemplar_path)?)?;
        }
        // 5. Prompt.
        //
        // Written for an agent working *in the repository* (it has file and
        // shell tools and reads the package itself), not for one being handed
        // pasted text: `SCRYFALL.json` alone would dominate the budget, and
        // most of it is printing metadata the card does not care about.
        let prompt = format!(
            "# Implement `{name}` in this repository\n\n\
             Edit exactly one file: `crates/baylee-cards/src/cards/{slug}.rs`.\n\
             Touch nothing else — not `src/generated.rs`, not `src/cards/mod.rs`,\n\
             not another card, not the DSL.\n\n\
             Read first, in this order:\n\
             - `crates/baylee-cards/AGENTS.md` — the playbook you are bound by.\n\
             - `docs/card-dsl.md` — the authoring contract and the full vocabulary.\n\
             {forge_line}\
             - `{package}/EXEMPLAR.rs` — an implemented card of the same type; match its style.\n\
             - `{package}/SCRYFALL.json` — metadata, if you need the printed details.\n\n\
             Hard rules:\n\
             1. `index`, `oracle_id`, `scryfall_id` and the `faces` literals are\n\
                generated facts. Do not edit them. You may edit only `coverage`,\n\
                `keywords` and `abilities`.\n\
             2. Never restate a default. The macros in `baylee-cards-dsl/src/build.rs`\n\
                supply them, and the defaults are *rules* defaults.\n\
             3. Do not invent `Effect`, `Modifier` or `Filter` variants. If the\n\
                DSL cannot say what the card says, STOP and refuse — see below.\n\
             4. Every oracle sentence is implemented, or the card is refused. A\n\
                card that is nearly right is worse than a stub: the deckbuilder\n\
                offers implemented cards as playable.\n\
             5. `cargo check -p baylee-cards` and `cargo test -p baylee-cards`\n\
                must pass before you finish.\n\n\
             Refusing is a correct outcome, not a failure. If any clause is\n\
             inexpressible, revert your edits to `{slug}.rs` so it stays the\n\
             generated stub, and report `status: \"refused\"`.\n\n\
             When you report a refusal, `cannot_say` must name **what the DSL\n\
             cannot express**, not which mechanic you think is missing, and\n\
             `nearest_existing` must name the closest variant that does exist.\n\
             Those two together are the whole value of a refusal: the last time\n\
             a blocker was read as a missing subsystem, the subsystem was\n\
             already there and one variant that could say \"the target\" was all\n\
             it needed.\n",
            package = dir.display(),
            // Named only when it is there. Roughly one card in eight has no
            // script under its printed name, and pointing an agent at a file
            // that does not exist spends a turn and teaches it that the
            // package's promises are approximate.
            forge_line = if has_forge {
                format!(
                    "- `{}/FORGE.txt` — the forge-reference script; rules ground truth.\n",
                    dir.display()
                )
            } else {
                "There is no forge-reference script for this card. The oracle text in \
                 the stub header is all the ground truth there is; if that leaves a \
                 clause genuinely ambiguous, refuse rather than guess.\n"
                    .to_string()
            },
        );
        fs::write(dir.join("PROMPT.md"), prompt)?;
    }
    Ok(())
}

/// A card's file stem.
///
/// Multi-face cards are filed under their front face, the way `codegen` slugs
/// them: "Zof Consumption // Zof Bloodbog" is one file called
/// `zof_consumption`. Slugging the whole printed name instead produces a path
/// that does not exist, so the card is read as implemented and skipped.
fn front_face_slug(name: &str) -> String {
    baylee_cards_codegen::stubgen::slug(name.split(" // ").next().unwrap_or(name))
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
    let mut costs = Vec::new();
    let mut rest = content;
    while let Some(pos) = rest.find("mana_cost: ") {
        rest = &rest[pos + "mana_cost: ".len()..];
        if rest.starts_with("ManaCost::ZERO") {
            costs.push("(no cost)".to_string());
        } else if rest.starts_with("baylee_core::mana!(\"") {
            rest = &rest["baylee_core::mana!(\"".len()..];
            if let Some(end) = rest.find('"') {
                let lit = &rest[..end];
                costs.push(if lit == "{0}" {
                    "(no cost)".to_string()
                } else {
                    lit.to_string()
                });
            }
        }
    }
    // A costless face writes no `mana_cost` line at all — `FaceDef::DEFAULT`
    // supplies `ManaCost::ZERO` and the authoring rule is never to restate a
    // default. So a face the loop above did not see *is* a free face, which
    // is how a land on the front of a modal double-faced card gets its
    // "(no cost)" back. Reading only the written lines made the checker
    // demand that a Land carry the cost of the Sorcery on its other side.
    let faces = content.matches("face! {").count();
    if faces > costs.len() {
        costs.push("(no cost)".to_string());
    }
    costs
}

/// Compares the mandatory human-readable header against the `CardDef`
/// data — the header is the safety net against generation drift, and it
/// is only a net if something checks it (two cost fixes once shipped
/// with stale headers).
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

/// Validates card-file conventions across the registry.
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
    for name in &names {
        let slug = front_face_slug(name);
        let path = root.join(format!("crates/baylee-cards/src/cards/{slug}.rs"));
        let Ok(content) = fs::read_to_string(&path) else {
            println!("MISSING FILE: {slug}");
            problems += 1;
            continue;
        };
        // A stub has nothing to claim: `CardDef::DEFAULT` is
        // `Unimplemented`, and writing the line out would be restating a
        // default. The header is still checked, because that is what the
        // person who finishes the card reads.
        let is_stub = content.contains("// GENERATED STUB");
        stubs += usize::from(is_stub);
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
    }
    if problems > 0 {
        anyhow::bail!("{problems} convention problem(s) found");
    }
    println!(
        "validate: {} cards conform ({} finished, {stubs} stubs)",
        names.len(),
        names.len() - stubs
    );
    Ok(())
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
    for row in rows.iter().filter(|r| r.deck == name) {
        let line = format!("{} {}", row.count, row.name);
        match row.zone {
            acceptance::Zone::Main => main.push(line),
            acceptance::Zone::Sideboard => side.push(line),
            // The engine does not run the commander format yet, so a
            // commander row would only be a deck the gateway refuses.
            acceptance::Zone::Commander => {}
        }
    }
    anyhow::ensure!(!main.is_empty(), "no deck called `{name}` in the file");
    Ok(serde_json::json!({ "name": name, "cards": main, "sideboard": side }))
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

    // A deck. Reused when a previous run already saved it, so the card pool
    // is not re-validated on every launch.
    let decks: serde_json::Value =
        serde_json::from_str(&get(&agent, &format!("{gateway}/decks"), &token)?)?;
    let existing = decks.as_array().and_then(|list| {
        list.iter()
            .find(|d| d.get("name").and_then(serde_json::Value::as_str) == Some(deck_name))
            .and_then(|d| d.get("id").and_then(serde_json::Value::as_str))
            .map(ToString::to_string)
    });
    let deck_id = if let Some(id) = existing {
        id
    } else {
        let (status, body) = post(
            &agent,
            &format!("{gateway}/decks"),
            Some(&token),
            &acceptance_deck(root, deck_name)?,
        )?;
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
    for entry in fs::read_dir(cards_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path)?;
            if !text.contains("// GENERATED STUB") {
                continue;
            }
            // The header's first line is `//! <name> — <cost> — <types>`.
            if let Some(head) = text.lines().next().and_then(|l| l.strip_prefix("//! ")) {
                let name = head.split(" \u{2014} ").next().unwrap_or(head).trim();
                out.insert(name.to_string());
            }
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
