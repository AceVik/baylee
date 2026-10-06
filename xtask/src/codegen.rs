//! `codegen`: the card files, the registry, the index tree and the tables
//! the compiled pool reads.

use crate::files::format_rust_many;
use crate::{
    BTreeMap, BTreeSet, Path, PathBuf, acceptance, card_files, cardindex, catalog, fs, layout,
    ledger, refresh_payload_cache, relative, render_ability_lines, render_name_table,
    render_oracle, render_sides_table, render_token_ledger, scriptgen, scripts, scripts_root,
    scryfall, stubgen, token_ledger, tokengen, write_or_check, write_verbatim,
};

/// What this pool is and how a card in it is read: the names, the subtype
/// catalogs every printing is decoded against, and the reference scripts when
/// a checkout is at hand.
///
/// One value rather than three parameters because two stages now need all of
/// it — the cards and the token ledger written before them — and a pool read
/// twice is a pool that can be read two different ways.
pub(crate) struct Pool<'a> {
    pub(crate) names: &'a [String],
    pub(crate) cats: &'a catalog::SubtypeCatalogs,
    pub(crate) scripts: Option<&'a scriptgen::ScriptLookup>,
    pub(crate) tokens: Option<&'a tokengen::TokenLookup>,
}

/// Every card the registry should hold: the acceptance decks (the architecture
/// proof, which says exactly what it says) plus `data/card-pool.txt` (a card
/// implemented for its own sake). Writes the stubs, the module list, the
/// `CardIndex` ledger and the registry tables.
pub(crate) fn cards(
    root: &Path,
    check: bool,
    agent: &ureq::Agent,
    cache: &Path,
    pool: &Pool,
    changed: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let Pool {
        names,
        cats,
        scripts,
        tokens,
    } = *pool;
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
    refresh_payload_cache(names, agent, cache);

    let mut stubs = Vec::with_capacity(names.len());
    let mut machine_owned = Vec::with_capacity(names.len());
    for name in names {
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
        let (info, content) =
            stubgen::render_stub(&card, row, &ledger, cats, scripts, tokens, &cycles)?;
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
        machine_owned.push((stub_path, content));
        stubs.push(info);
    }
    // Formatted together, in parallel, and written in the order they came:
    // one rustfmt per card was nearly the whole run (`format_rust_many`).
    let (paths, sources): (Vec<PathBuf>, Vec<String>) = machine_owned.into_iter().unzip();
    let formatted = format_rust_many(&root.join("target"), &sources)?;
    for (path, content) in paths.iter().zip(&formatted) {
        write_verbatim(check, path, content, changed)?;
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
pub(crate) fn write_index_tree(
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

pub(crate) fn codegen(
    root: &Path,
    check: bool,
    tables_only: bool,
    scripts_dir: &Path,
    cache: &Path,
) -> anyhow::Result<()> {
    let cache = root.join(cache);
    let agent = ureq::Agent::new_with_defaults();
    let mut changed = Vec::new();

    // The pool, read once. It used to be read inside the card stage, which
    // was right while the card stage was the only thing that knew which cards
    // exist; the token ledger has to know too, and it is written first.
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    let names = acceptance::all_names(&rows, &pool_text);
    // Before stage 1, not beside the card stage: the token ledger is written
    // first, so a refusal down there would already have written a table.
    refuse_twin_names(&names)?;
    let from_decks = acceptance::unique_names(&rows).len();
    println!(
        "card pool: {} cards ({from_decks} from the acceptance decks, {} from the pool file)",
        names.len(),
        names.len() - from_decks
    );

    // Everything from here to stage 5 reads the card-script reference, and
    // on a machine with none checked out it rewrites every machine-owned
    // card as an honest stub. `--tables` is the way past it: the two tables
    // below are built from the compiled pool alone.
    if tables_only {
        // 3, the hand-written half. A token written into `tokens.rs` for a
        // card finished by hand needs its id before a card may name it, and
        // that half reads nothing from the corpus: the ledger keeps every
        // row it already has (generated ones included, bodies and all) and
        // appends what `tokens.rs` declares that it lacks. With nothing new
        // the file comes out byte for byte as it went in.
        write_or_check(
            check,
            &root.join("crates/baylee-cards/src/generated_tokens.rs"),
            &render_token_ledger(root, &[])?,
            &mut changed,
        )?;
    } else {
        // 1. Subtype catalogs → generated subtypes.rs.
        //
        // The table is its own ledger (#43): what this binary compiled against
        // *is* the committed assignment, so a name keeps its id and a new one
        // takes the next free number. Nothing here renumbers anything, which
        // is what a `SubtypeSet` on the wire requires.
        let cats = scryfall::fetch_subtype_catalogs(&agent, &cache)?;
        write_or_check(
            check,
            &root.join("crates/baylee-core/src/generated/subtypes.rs"),
            &catalog::render_subtypes_rs(&cats, &catalog::PriorSubtypes::from_compiled_table()),
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

        // The reference's token scripts, found beside its card scripts. Read
        // before the ledger because both stages below want them: the ledger to
        // learn which tokens exist, the card stage to write the constant a
        // `DB$ Token` names.
        let tokens = tokengen::TokenLookup::beside(&scripts_dir)?;
        let pool = Pool {
            names: &names,
            cats: &cats,
            scripts: lookup.as_ref(),
            tokens: tokens.as_ref(),
        };

        // 3. Which id every token there is was assigned → generated_tokens.rs.
        token_ledger(root, check, &pool, &mut changed)?;

        // 4. The card pool → per-card stubs + registry.
        cards(root, check, &agent, &cache, &pool, &mut changed)?;
    }

    // 5. Which printed sentence each ability came from → generated_lines.rs.
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

    // 6. Which card a printed English name is → generated_names.rs.
    //    Beside stage 5 and not beside the registry, for its reason: this
    //    is built from the **compiled** pool, so it is two-phase in the
    //    same way and `--check` is what makes the second run a build
    //    failure rather than a name that silently resolves to nothing.
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated_names.rs"),
        &render_name_table()?,
        &mut changed,
    )?;

    // 7. Which cards have a back, and which are double-faced → generated_sides.rs.
    //    Beside stages 5 and 6 because it is the same pair of sources as
    //    stage 5 — the compiled pool read against the cached printings — and
    //    two-phase for the same reason. It is here rather than on `CardDef`
    //    because both answers are facts about the *printing*, which is what
    //    `def.faces.len() > 1` was standing in for and getting wrong in both
    //    directions (#115).
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated_sides.rs"),
        &render_sides_table(root)?,
        &mut changed,
    )?;

    // 8. Each card's English Oracle text, per face → generated_oracle.rs.
    //    The text stage 5 counted its sentences in, from the same payloads,
    //    so a client can draw the sentence an index names when the player's
    //    language has none that pairs with it. Two-phase like stage 5.
    write_or_check(
        check,
        &root.join("crates/baylee-cards/src/generated_oracle.rs"),
        &render_oracle(root)?,
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
pub(crate) fn refuse_orphans<'a>(
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

/// Refuses two pool lines that are the same card written two ways.
///
/// A card can be named by its front face (`Hengegate Pathway`) or the way
/// Scryfall and the ledger name it (`Hengegate Pathway // Mistgate Pathway`).
/// Both slug to one file, so both produce `pub mod hengegate_pathway;` and the
/// crate does not compile — and because `xtask` links `baylee-cards`, `codegen`
/// cannot then be run to repair what it just wrote. The two lines have to come
/// out of `cards/mod.rs` by hand first. So it is asked before stage 1 — ahead
/// of even the token ledger, which is written before the cards — for the same
/// reason the orphan check is asked before the first rename.
///
/// It is not hypothetical arithmetic: 300 ledger cards drawn at random into
/// the pool collided once, and `data/card-pool.txt` is meant to grow towards
/// the ledger's 33 694, where the same card being written both ways stops
/// being an accident.
pub(crate) fn refuse_twin_names(names: &[String]) -> anyhow::Result<()> {
    let mut by_slug: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for name in names {
        by_slug
            .entry(front_face_slug(name))
            .or_default()
            .push(name.as_str());
    }
    let twins: Vec<String> = by_slug
        .into_iter()
        .filter(|(_, ns)| ns.len() > 1)
        .map(|(slug, ns)| format!("{slug} ({})", ns.join(" | ")))
        .collect();
    if twins.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "{} card(s) named more than once across the acceptance decks and \
         data/card-pool.txt: {}",
        twins.len(),
        twins.join(", ")
    )
}

/// Refuses a cycle-map entry naming a card the pool does not have.
///
/// The failure it catches is silent by construction: the map is additive, so
/// a name that matches nothing simply leaves its land filed one level
/// shallower. Ten pathways sat unfiled for a whole commit that way, the map
/// holding front-face names while Scryfall hands over `A // B`.
pub(crate) fn refuse_stale_cycles(
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

pub(crate) fn front_face_slug(name: &str) -> String {
    baylee_cards_codegen::stubgen::slug(name.split(" // ").next().unwrap_or(name))
}
