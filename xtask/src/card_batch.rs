//! `card-batch`: a prompt per card for a writer, with an exemplar beside it,
//! and the land report.

use crate::{
    BTreeMap, Path, acceptance, card_files, front_face_slug, fs, landgen, scripts_root, scryfall,
};

pub(crate) fn exemplar_for(type_line: &str) -> &'static str {
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
pub(crate) fn card_batch(
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
pub(crate) fn card_prompt(name: &str, file: &str, package: &Path, has_script: bool) -> String {
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
pub(crate) fn land_report(
    root: &Path,
    cache: &Path,
    samples: usize,
    worklist: Option<&Path>,
    tail: usize,
) -> anyhow::Result<()> {
    let agent = ureq::Agent::new_with_defaults();
    let cache = root.join(cache);
    let cats = scryfall::fetch_subtype_catalogs(&agent, &cache)?;
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
pub(crate) fn shape_of(line: &str) -> String {
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
