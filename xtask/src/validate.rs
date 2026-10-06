//! `validate`: every committed card against its printing, and the floors
//! that say the sweeps reached what they should.

use crate::{
    BTreeMap, Path, acceptance, cached_printing, card_files, check_code_matches_the_printing,
    check_def_against_the_printing, check_header_matches_code, check_no_name_is_claimed_twice,
    check_oracle_matches_the_printing, check_player_targets_match_the_printing,
    check_search_tapped_matches_text, check_set_line_matches_the_printing,
    check_target_counts_match_the_printing, front_face_slug, fs, knob, precons, stubgen,
    unplayable_ids,
};

/// What the printing checks actually compared.
///
/// Every one of them skips quietly — no cached payload, no field in it, no
/// symbol in the text, no comparison — so the whole family can go silent
/// without a single line of output changing. These counts are what says it
/// did not.
#[derive(Default)]
pub(crate) struct PrintingTally {
    /// Cards with a cached payload at all.
    pub(crate) payloads: usize,
    /// Cards whose starting loyalty was compared.
    pub(crate) loyalty: usize,
    /// Cards whose color identity was compared.
    pub(crate) identity: usize,
    /// Cards claiming at least one keyword bit that has a printed spelling.
    pub(crate) keywords: usize,
    /// Cards whose mana abilities were read and held against the text.
    pub(crate) mana: usize,
    /// Cards whose Oracle header was held against the printed text.
    pub(crate) oracle: usize,
    /// Face costs compared against the printing, over the whole pool.
    pub(crate) costs: usize,
    /// Printed activation costs held against the cost the code pays.
    pub(crate) activation_costs: usize,
    /// Cards whose printing names a player or an opponent as a target.
    pub(crate) player_targets: usize,
    /// Cards whose printing states a target count of "up to".
    pub(crate) targets: usize,
    /// Cards whose printing says "you may" outside reminder text.
    pub(crate) optional_clauses: usize,
    /// Cards whose `Set:` header line was held against the printing.
    pub(crate) printings: usize,
    /// Faces whose type line was held against the printed one.
    pub(crate) type_lines: usize,
    /// Printed power/toughness values that are a sentence and not a number.
    pub(crate) defined_pt: usize,
    /// Faces whose counted enters-tapped bound was held against the print.
    pub(crate) enters_tapped: usize,
    /// Back faces whose castability was held against the printed layout.
    pub(crate) back_faces: usize,
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
/// Activation cost is **172**, measured 2026-09-17 over 159 implemented
/// cards — every printed `{…}…:` price whose symbols parse. It is the one
/// count here that a *finished card* moves up rather than a new card: the
/// four `Partial` cards refusing a printed ability by name (Kenrith, Lotleth
/// Troll, Urza, Yawgmoth) each rejoin the population the day their missing
/// ability exists.
///
/// Ability-defined P/T is **6**, measured 2026-09-18: three cards print a
/// power and a toughness that are a sentence rather than a number — Ashaya,
/// Lumra and Unlicensed Hearse, each "equal to the number of …". The floor
/// is 4 rather than 5 because the population is three cards and one leaving
/// the pool takes two occurrences with it. It is the smallest count here and
/// the one most likely to be read as noise, so what it is for is worth
/// saying: it is not a measure of the pool, it is the guard on a branch that
/// otherwise *silently stops comparing* — the day `printed.parse()` starts
/// failing for a reason that is not CR 208.2, this is the number that says
/// so.
///
/// The branch is a failed `parse`, so it also catches `1+*`, `X` and an
/// empty string — and the empty one would be a payload the check cannot
/// read, counted as if it were a card that defines its own power. Across
/// the 1623 cached payloads there are three non-numeric values and all
/// three are `*`; none is empty. That is what makes the floor a bound on
/// the three cards rather than on whatever the cache happened to be
/// missing.
///
/// Type line is **1473** and that number is exact rather than measured: the
/// pool prints 1475 faces (1255 cards of one face and 110 of two), and the
/// only card whose printed and code face counts disagree is Emeritus of Woe,
/// an adventure Scryfall writes as two faces and this pool as one. Its two
/// are the two that are skipped, so the check reaches every face it can.
/// Nothing is skipped for an unknown subtype word today — every word the
/// pool's 1475 type lines print is in the catalog — which is the number to
/// watch if it ever drops below 1473 without a card leaving the pool.
///
/// Enters-tapped bound is **38**, measured 2026-09-22 at **41** over a pool
/// of 2716. The number is deliberately not the 31 a grep for the printed
/// word "other" finds: the parser asks only for a counted bound, so a
/// battle land's "unless you control two or more basic lands" is in the
/// population and "other" is not part of the sentence being read. Counting
/// it by one instrument and bounding it by another is how a threshold comes
/// to guard nothing, so this floor is set from the check's own line.
pub(crate) const PRINTING_FLOOR: PrintingTally = PrintingTally {
    payloads: 1300,
    loyalty: 6,
    identity: 1300,
    keywords: 45,
    mana: 340,
    oracle: 1300,
    costs: 1340,
    activation_costs: 160,
    player_targets: 20,
    targets: 10,
    optional_clauses: 40,
    printings: 1300,
    type_lines: 1400,
    defined_pt: 4,
    enters_tapped: 38,
    back_faces: 55,
};

/// What [`check_header_matches_code`]'s type segment reached, less a margin.
///
/// It is the whole pool and nothing is skipped: every name in the acceptance
/// list resolves to a compiled `CardDef`, so the count is 1365 — and unlike
/// every floor above it this one needs no printing, which is why it is not a
/// field of [`PrintingTally`]. A card that stopped resolving would be a gap
/// in this command rather than in the pool, and would otherwise be silent.
pub(crate) const HEADER_TYPE_FLOOR: usize = 1360;

pub(crate) fn validate(root: &Path) -> anyhow::Result<()> {
    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    // An unplayable card is never built, so one in the pool is a card that
    // somebody built by mistake.
    let unplayable = unplayable_ids(root)?;
    for def in baylee_cards::all() {
        if let Some(name) = unplayable.get(def.oracle_id) {
            anyhow::bail!("{name} is listed in data/unplayable.tsv and is in the pool");
        }
    }
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
    // Cards whose header names a printing this cache does not hold. Not a
    // problem and not a tallied comparison: it is the one thing the
    // `printings` floor cannot say on its own, which is *why* a run compared
    // fewer cards than it could have.
    let mut unpinned = 0usize;
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
            check_def_against_the_printing(&slug, def, &payload, &mut tally, &mut problems);
        }
        check_code_matches_the_printing(&slug, &content, &payload, &mut tally, &mut problems);
        check_oracle_matches_the_printing(&slug, &content, &payload, &mut tally, &mut problems);
        check_set_line_matches_the_printing(
            root,
            &slug,
            &content,
            &payload,
            &mut tally,
            &mut unpinned,
            &mut problems,
        );
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
    report_the_cache_age(root);
    report_what_the_sweeps_reached(&tally, header_types, unpinned, &mut problems);
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

/// How old the payload cache is, said out loud rather than assumed.
///
/// Every printing check above reads `data/scryfall-cache`, and `fetch_named`
/// answers from disk without ever refetching, so what this command compares a
/// card against is not "the printing" but "the printing as of the day the
/// cache was filled". A developer therefore validates against a snapshot while
/// CI, which starts cold, validates against live Scryfall — and the two
/// disagree the moment Scryfall ships a set that reprints a pool card. That is
/// not hypothetical: `validate` went red on CI at 0f450764 over 32 headers
/// whose default printing moved to *Reality Fracture Commander*, with every
/// local gate green (#50).
///
/// It is a line of output and not a failure, because a stale cache is the
/// normal state of a working checkout and a gate that goes red for a missing
/// download teaches people to stop running it.
pub(crate) fn report_the_cache_age(root: &Path) {
    let cache = root.join("data/scryfall-cache");
    let Ok(entries) = fs::read_dir(&cache) else {
        return;
    };
    // The **oldest** entry, not the newest. A cache is filled card by card as
    // the pool grows, so its newest file is whatever was added last — which
    // is 0 days old on the very run that added a card and says nothing at all
    // about the 1615 payloads beside it. What bounds this command's answer is
    // the stalest one.
    let oldest = entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .min();
    let Some(oldest) = oldest else {
        return;
    };
    let Ok(age) = std::time::SystemTime::now().duration_since(oldest) else {
        return;
    };
    println!(
        "validate: the oldest cached payload is {} day(s) old \u{2014} CI reads live Scryfall",
        age.as_secs() / 86_400
    );
}

/// Every count, then every floor — before the bail rather than after it,
/// because a floor that failed is only readable beside the counts that
/// failed it.
pub(crate) fn report_what_the_sweeps_reached(
    tally: &PrintingTally,
    header_types: usize,
    unpinned: usize,
    problems: &mut usize,
) {
    println!(
        "validate: against the printings \u{2014} {} payloads, {} loyalty, {} identity, \
         {} keyword, {} mana, {} oracle, {} cost, {} activation cost, {} player target, \
         {} target count, {} optional clause, {} printing, {} type line, \
         {} ability-defined P/T, {} enters-tapped bound, {} back face",
        tally.payloads,
        tally.loyalty,
        tally.identity,
        tally.keywords,
        tally.mana,
        tally.oracle,
        tally.costs,
        tally.activation_costs,
        tally.player_targets,
        tally.targets,
        tally.optional_clauses,
        tally.printings,
        tally.type_lines,
        tally.defined_pt,
        tally.enters_tapped,
        tally.back_faces
    );
    check_printing_floors(tally, problems);
    if unpinned > 0 {
        println!(
            "validate: {unpinned} header(s) name a printing this cache does not hold \u{2014} \
             run `cargo run -p xtask -- scryfall-cache` to fetch them"
        );
    }
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
pub(crate) fn check_printing_floors(tally: &PrintingTally, problems: &mut usize) {
    for (what, seen, floor) in [
        ("payloads", tally.payloads, PRINTING_FLOOR.payloads),
        ("back face", tally.back_faces, PRINTING_FLOOR.back_faces),
        ("loyalty", tally.loyalty, PRINTING_FLOOR.loyalty),
        ("color identity", tally.identity, PRINTING_FLOOR.identity),
        ("keyword", tally.keywords, PRINTING_FLOOR.keywords),
        ("mana", tally.mana, PRINTING_FLOOR.mana),
        ("oracle text", tally.oracle, PRINTING_FLOOR.oracle),
        ("face cost", tally.costs, PRINTING_FLOOR.costs),
        (
            "activation cost",
            tally.activation_costs,
            PRINTING_FLOOR.activation_costs,
        ),
        (
            "player target",
            tally.player_targets,
            PRINTING_FLOOR.player_targets,
        ),
        ("target count", tally.targets, PRINTING_FLOOR.targets),
        (
            "ability-defined P/T",
            tally.defined_pt,
            PRINTING_FLOOR.defined_pt,
        ),
        (
            "optional clause",
            tally.optional_clauses,
            PRINTING_FLOOR.optional_clauses,
        ),
        ("printing", tally.printings, PRINTING_FLOOR.printings),
        ("type line", tally.type_lines, PRINTING_FLOOR.type_lines),
        (
            "enters-tapped bound",
            tally.enters_tapped,
            PRINTING_FLOOR.enters_tapped,
        ),
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
/// Read a deck file and report what this pool can do with it.
///
/// The round-trip is the load-bearing half and is checked per line rather
/// than per file: `deckrow` promises that writing a parsed row reproduces
/// it, so the two spellings disagreeing is a defect in whichever side wrote
/// the file — and a report that only counted rows would pass while every
/// printing quietly sat inside a card name.
///
/// Both deck dialects are read: the `[deck:…]` sections of the house decks,
/// and Baylee text (`docs/deck-format.md`), whose `CMD:`/`SB:`/`MB:` prefix
/// names a row's zone and whose commander is a row like any other — the form
/// `data/decks/precon/` is written in. The last line is the verdict the
/// precon status unlocks a deck by (`precons::verdict`): every card in the
/// pool, `Implemented`, and named in the engine's test code.
/// With `--verbose` it also says how and where a test names each working
/// card of the deck.
#[allow(clippy::too_many_lines)] // one pass over the rows owns every tally it reports
pub(crate) fn deck_check(root: &Path, file: &Path, verbose: bool) -> anyhow::Result<()> {
    use anyhow::Context as _;
    use baylee_cards::dsl::Coverage;
    use baylee_core::deckrow;

    let working =
        baylee_train::working::Working::scan(root).context("reading the engine's test code")?;
    // Each working card of the deck, with how and where a test names it.
    let mut tested_by: Vec<String> = Vec::new();
    let path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        root.join(file)
    };
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;

    let mut section = "deck";
    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    let mut rows = 0usize;
    let mut bad_round_trip = Vec::new();
    let mut unknown = Vec::new();
    let mut partial = Vec::new();
    let mut stubs = Vec::new();
    let mut names: Vec<String> = Vec::new();

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            section = match rest.split([':', ']']).next() {
                Some("sideboard") => "sideboard",
                Some("commander") => "commander",
                _ => "deck",
            };
            continue;
        }
        let (section, line, prefixed) = match line.split_once(": ") {
            Some(("CMD", row)) => ("commander", row, true),
            Some(("SB", row)) => ("sideboard", row, true),
            Some(("MB", row)) => ("maybeboard", row, true),
            _ => (section, line, false),
        };
        rows += 1;
        // A commander is stored as a **bare card name** and never as a row:
        // the column is read with `decks::by_name`, an exact-spelling lookup,
        // and `from_lines` silently drops a leader it cannot resolve. So a
        // `[commander]` line written the way a deck row is written seats
        // nobody and says nothing about it — which is exactly the shape this
        // reader has to be able to refuse. A `CMD:` row is Baylee text's
        // commander, a row by design, which the import turns into both.
        let name = if section == "commander" && !prefixed {
            if let Ok(row) = deckrow::parse(line)
                && row.to_string() == line
            {
                bad_round_trip.push(format!("{line}  (a leader is a bare card name)"));
                continue;
            }
            *counts.entry(section).or_default() += 1;
            line.to_string()
        } else {
            let row = match deckrow::parse(line) {
                Ok(row) => row,
                Err(err) => {
                    bad_round_trip.push(format!("{line}  ({err:?})"));
                    continue;
                }
            };
            let written = row.to_string();
            if written != line {
                bad_round_trip.push(format!("{line}  -> {written}"));
            }
            *counts.entry(section).or_default() += row.count;
            row.name.clone()
        };
        if !names.contains(&name) {
            names.push(name.clone());
        }
        match baylee_cards::decks::by_name(&name).and_then(baylee_cards::by_index) {
            None => unknown.push(name.clone()),
            Some(def) => match def.coverage {
                Coverage::Implemented => {
                    if let Some(e) = working.tested.get(&def.index) {
                        let line = format!("{name} — {} in {}", e.how, e.file);
                        if !tested_by.contains(&line) {
                            tested_by.push(line);
                        }
                    }
                }
                Coverage::Partial(why) => partial.push(format!("{name} — {why}")),
                Coverage::Unimplemented => stubs.push(name.clone()),
            },
        }
    }

    println!("{}", path.display());
    for (section, n) in &counts {
        println!("  {section}: {n} cards");
    }
    println!(
        "  {rows} rows, {} not round-tripping, {} unknown to the pool, \
{} partial, {} stubs",
        bad_round_trip.len(),
        unknown.len(),
        partial.len(),
        stubs.len()
    );
    for bad in &bad_round_trip {
        println!("  ROUND TRIP  {bad}");
    }
    for name in &unknown {
        println!("  NOT IN POOL {name}");
    }
    if verbose {
        for line in &tested_by {
            println!("  TESTED      {line}");
        }
        for name in &partial {
            println!("  PARTIAL     {name}");
        }
        for name in &stubs {
            println!("  STUB        {name}");
        }
    }
    let refused: Vec<(String, precons::Why)> = names
        .iter()
        .filter_map(|name| {
            precons::verdict(name, &working)
                .err()
                .map(|why| (name.clone(), why))
        })
        .collect();
    for (name, why) in &refused {
        if *why == precons::Why::Untested {
            let tested = working.tested.len();
            println!("  UNTESTED    {name}  (no engine test names it; {tested} cards are named)");
        }
    }
    println!(
        "  playable (every card implemented and named in engine test code): {}",
        if refused.is_empty() {
            "yes".to_string()
        } else {
            format!("no, {} cards", refused.len())
        }
    );
    if bad_round_trip.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("{} row(s) do not round-trip", bad_round_trip.len())
    }
}
