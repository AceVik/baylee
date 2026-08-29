//! xtask — baylee development tasks (codegen, card explanation, …).

use baylee_cards_codegen::{acceptance, catalog, forge, scryfall, stubgen};
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
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
        Cmd::Explain { name, forge, cache } => explain(&root, &name, &forge, &cache),
        Cmd::CardBatch {
            cards,
            out,
            forge,
            cache,
        } => card_batch(&root, cards.as_deref(), &out, &forge, &cache),
        Cmd::Validate => validate(&root),
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

    // 2. Acceptance decks → per-card stubs + registry.
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let names = acceptance::unique_names(&rows);
    println!("acceptance suite: {} unique cards", names.len());
    let mut stubs = Vec::with_capacity(names.len());
    for (i, name) in names.iter().enumerate() {
        let card = scryfall::fetch_named(name, &agent, &cache)?;
        let (info, content) = stubgen::render_stub(&card, i as u32, &cats)?;
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
        write_or_check(check, &stub_path, &content, &mut changed)?;
        stubs.push(info);
    }
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/cards/mod.rs"),
        &stubgen::render_cards_mod(&stubs),
        &mut changed,
    )?;
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated.rs"),
        &stubgen::render_registry(&stubs),
        &mut changed,
    )?;

    // 3. forge-reference index.
    let forge_dir = root.join(forge_dir);
    if forge_dir.exists() {
        let index = forge::build_index(&forge_dir)?;
        write_or_check(
            check,
            &root.join("data/forge_index.json"),
            &serde_json::to_string_pretty(&index)?,
            &mut changed,
        )?;
        println!("forge index: {} scripts", index.len());
    } else {
        println!(
            "note: forge-reference not found at {}, skipping index",
            forge_dir.display()
        );
    }

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
    let names = acceptance::unique_names(&rows);
    let forge_index: BTreeMap<String, String> = serde_json::from_str(
        &fs::read_to_string(root.join("data/forge_index.json")).unwrap_or_default(),
    )?;
    let wanted: Vec<String> = if let Some(list) = cards {
        list.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        names
            .iter()
            .filter(|name| {
                let slug = baylee_cards_codegen::stubgen::slug(name);
                let path = root.join(format!("crates/baylee-cards/src/cards/{slug}.rs"));
                fs::read_to_string(&path).is_ok_and(|c| c.contains("Coverage::Unimplemented"))
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
        let slug = baylee_cards_codegen::stubgen::slug(name);
        let dir = out.join(&slug);
        fs::create_dir_all(&dir)?;
        // 1. Current stub.
        let stub_path = root.join(format!("crates/baylee-cards/src/cards/{slug}.rs"));
        let stub = fs::read_to_string(&stub_path)?;
        fs::write(dir.join("STUB.rs"), &stub)?;
        // 2. Forge script (ground truth).
        if let Some(rel) = forge_index.get(name) {
            let script = root.join(forge_dir).join(rel);
            if script.exists() {
                fs::write(dir.join("FORGE.txt"), fs::read_to_string(script)?)?;
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
        let prompt = format!(
            "# Implement `{name}` for baylee\n\n\
             Follow crates/baylee-cards/AGENTS.md and docs/card-dsl.md exactly.\n\n\
             - STUB.rs: the generated stub to complete (edit it into the final card).\n\
             - FORGE.txt: the forge-reference script (rules ground truth).\n\
             - SCRYFALL.json: card metadata.\n\
             - EXEMPLAR.rs: an implemented card of the same type — match its style.\n\n\
             Rules: preserve index/oracle_id/scryfall_id/faces data; implement every\n\
             oracle sentence or use Coverage::Partial + NOT SUPPORTED comments;\n\
             write tests; `cargo check -p baylee-cards` and `cargo test -p baylee-cards`\n\
             must pass. Output: the complete final contents of crates/baylee-cards/src/cards/{slug}.rs\n",
        );
        fs::write(dir.join("PROMPT.md"), prompt)?;
    }
    Ok(())
}

/// Validates card-file conventions across the registry.
fn validate(root: &Path) -> anyhow::Result<()> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let names = acceptance::unique_names(&rows);
    let mut problems = 0usize;
    for name in &names {
        let slug = baylee_cards_codegen::stubgen::slug(name);
        let path = root.join(format!("crates/baylee-cards/src/cards/{slug}.rs"));
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                println!("MISSING FILE: {slug}");
                problems += 1;
                continue;
            }
        };
        for check in [
            ("header name", content.contains("//!")),
            ("set line", content.contains("Set:")),
            ("scryfall id", content.contains("Scryfall ID:")),
            ("oracle id", content.contains("Oracle ID:")),
            ("coverage flag", content.contains("coverage: Coverage::")),
            ("tests module", content.contains("mod tests")),
        ] {
            if !check.1 {
                println!("{slug}: missing {}", check.0);
                problems += 1;
            }
        }
    }
    if problems > 0 {
        anyhow::bail!("{problems} convention problem(s) found");
    }
    println!("validate: {} cards conform", names.len());
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
