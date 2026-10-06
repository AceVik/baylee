//! The compiled pool's tables: ability lines, names, sides and the token
//! ledger.

use crate::{
    BTreeMap, Path, acceptance, cached_printing, face_texts, fs, lines, names, scryfall, tokengen,
    tokenledger,
};

/// Why one ability found no sentence.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum Miss {
    /// The card prints no line of that shape at all — an ability whose
    /// sentence is printed as a *keyword* (Mulldrifter's evoke sacrifice,
    /// Lightning Greaves' `Equip {0}`).
    NoLineOfThatShape,
    /// A line of the right shape is there and none of them matches what
    /// the ability says about itself. This is the bucket that matters: it
    /// is where a swapped pair and a wrong cost both land.
    NoLineThatSaysThat,
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
pub(crate) fn ability_lines(root: &Path) -> anyhow::Result<()> {
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
pub(crate) fn render_name_table() -> anyhow::Result<String> {
    use anyhow::Context as _;

    let entries: Vec<(&str, u32)> = baylee_cards::all()
        .map(|def| (def.name(), def.index.get()))
        .collect();

    // The second spelling, and the pool cannot supply it. Joining a card's
    // own face names reproduces Scryfall's whole name for 120 of the 121
    // that need one — and the 121st is Emeritus of Woe, which this build
    // implements with a single face while the printing has two, so the join
    // would silently write `Emeritus of Woe` and the whole spelling would
    // still miss. The ledger has every card's whole name whatever this pool
    // did with it, so the ledger is the source.
    let mut whole: Vec<(&str, u32)> = Vec::new();
    for def in baylee_cards::all() {
        let index = def.index.get();
        let row = baylee_cards_index::ROWS
            .get(index as usize)
            .filter(|row| row.index == def.index)
            .with_context(|| format!("{} sits at no ledger row {index}", def.name()))?;
        if row.name == def.name() {
            continue;
        }
        // The only difference a second spelling may be is a face split. A
        // ledger rename would otherwise walk into this table as a key that
        // resolves a name this build does not have, which is the opposite
        // of what the table is for — so it stops the run with the card
        // named instead.
        anyhow::ensure!(
            row.name
                .strip_prefix(def.name())
                .is_some_and(|rest| rest.starts_with(" // ")),
            "{} is called {:?} in the ledger, which is not {:?} plus a \
             second face — a rename, and this table may not guess at it",
            index,
            row.name,
            def.name()
        );
        whole.push((row.name, index));
    }
    Ok(names::render(&entries, &whole)?)
}

/// Renders `crates/baylee-cards/src/generated_sides.rs`: which cards have a
/// back to show, and which are double-faced in the sense CR 712.1 gives it.
///
/// # Why a table and not a field
///
/// Both answers are facts about the **printing**, and the compiled pool does
/// not carry them — which is how `PoolCard::two_faced` came to be
/// `def.faces.len() > 1`, a count of what this build happened to compile,
/// standing in for two different questions and answering neither (#115). A
/// field on `CardDef` or `FaceDef` would reach the 328 hand-written cards
/// only through `..DEFAULT`, so a default of "no back" on a hand-written
/// transform card would be wrong in the direction nothing catches. A table
/// keyed by index is written for every card whoever owns the file.
///
/// A per-face property could not say it either: `Emeritus of Woe` prints two
/// faces and compiles one, so a fact about the second face has nowhere to
/// live on a card that has no second face here.
///
/// # Two-phase, and it refuses rather than skips
///
/// Built from the pool **compiled into this binary** read against the cached
/// printings, like stages 5 and 6, so a card added by this run gets its row
/// from the next one and `codegen --check` is the guard. Unlike stage 5 it
/// treats a missing payload as fatal: a skipped card here is not a card with
/// no ability line, it is a card silently recorded as having no back.
pub(crate) fn render_sides_table(root: &Path) -> anyhow::Result<String> {
    use anyhow::Context as _;
    use std::fmt::Write as _;

    let decks_text = fs::read_to_string(root.join("data/acceptance-decks.txt"))?;
    let rows = acceptance::parse_decks(&decks_text)?;
    let pool_text = fs::read_to_string(root.join("data/card-pool.txt")).unwrap_or_default();
    // The pool's own spelling, because that is what the printing is cached
    // under — the same key stage 5 reads by.
    let names = acceptance::all_names(&rows, &pool_text);
    let by_name: BTreeMap<&str, &'static baylee_cards::dsl::CardDef> =
        baylee_cards::all().map(|def| (def.name(), def)).collect();

    let mut back = Vec::new();
    let mut dfc = Vec::new();
    let mut walked = 0usize;
    for name in &names {
        let Some(def) = by_name.get(name.split(" // ").next().unwrap_or(name)) else {
            // Not compiled yet: the two-phase gap, and the next run closes it.
            continue;
        };
        let payload = cached_printing(root, name)
            .with_context(|| format!("{name}: no cached printing, so its sides cannot be read"))?;
        let card: scryfall::ScryfallCard = serde_json::from_value(payload)
            .with_context(|| format!("{name}: cached printing unreadable"))?;
        let sides = card.sides().map_err(anyhow::Error::msg)?;
        walked += 1;
        if sides.back_image {
            back.push(def.index.get());
        }
        if sides.double_faced {
            dfc.push(def.index.get());
        }
    }
    // The bound that makes this a reader rather than a green run over
    // nothing: eleven textual readers of this pool have been found answering
    // a question they could not see, and the one that said so was the one
    // carrying a floor. A pool that has shrunk below this has a bigger
    // problem than the table.
    anyhow::ensure!(
        walked > 2000,
        "read only {walked} printings out of {} — this table would be written from almost nothing",
        names.len()
    );
    // Both are strict subsets of a set the pool already knows, and the
    // arithmetic is the whole finding of #115: a back is not a second name.
    anyhow::ensure!(
        !back.is_empty() && !dfc.is_empty(),
        "no card has a back image ({}) or is double-faced ({}) — the reader has stopped reading",
        back.len(),
        dfc.len()
    );
    back.sort_unstable();
    dfc.sort_unstable();

    let mut out = String::new();
    out.push_str(
        "// GENERATED by `cargo xtask codegen` \u{2014} do not edit by hand.\n\
         //\n\
         // Two questions a client asks about a card's sides, and they are not\n\
         // the same question. `BACK_IMAGE` is whether Scryfall serves a second\n\
         // picture for the printing this pool pinned, read off `image_uris` \u{2014}\n\
         // which Scryfall puts at exactly one of two levels, on the card for one\n\
         // piece of cardboard and on each face for two. `DOUBLE_FACED` is\n\
         // CR 712.1: a card with a face on each side and no Magic card back,\n\
         // in three kinds — nonmodal, modal, and meld.\n\
         //\n\
         // They differ, and the difference is why there are two tables: a meld\n\
         // card is double-faced and has no back image, because Scryfall models\n\
         // a meld back as a card of its own rather than as a face. Neither is\n\
         // `faces.len() > 1`, which is a count of what this build compiled and\n\
         // is the bug this table replaces (#115).\n\
         //\n\
         // Sorted, so the reader in `baylee_cards::sides` can binary-search.\n\
         //\n\
         // Source: the compiled pool, `baylee_cards::all()`, read against the\n\
         // cached Scryfall printings — the pool knows which cards exist and\n\
         // only the printing knows what is on the back of one.\n\
         #![allow(missing_docs, clippy::all, clippy::pedantic)]\n\n\
         use baylee_core::ids::CardIndex;\n\n",
    );
    for (name, doc, rows) in [
        (
            "BACK_IMAGE",
            "/// Cards whose pinned printing has a second picture.\n",
            &back,
        ),
        (
            "DOUBLE_FACED",
            "/// Cards that are double-faced cards under CR 712.1.\n",
            &dfc,
        ),
    ] {
        out.push_str(doc);
        let _ = writeln!(out, "pub static {name}: [CardIndex; {}] = [", rows.len());
        for index in rows {
            let _ = writeln!(out, "    CardIndex::new({index}),");
        }
        out.push_str("];\n\n");
    }
    Ok(out)
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
pub(crate) fn render_token_ledger(
    root: &Path,
    generated: &[tokengen::TokenBody],
) -> anyhow::Result<String> {
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
    let entries = tokenledger::assign(existing, &hand, generated)?;
    // The two halves are counted out of the *ledger*, not out of what was
    // offered to it: a read token whose constant a hand-written one already
    // claims is filed once, so `generated.len()` here would print a total
    // that does not add up to the entry count.
    let read = entries
        .iter()
        .filter(|e| matches!(e.body, tokenledger::Body::Generated { .. }))
        .count();
    println!(
        "token ledger: {} entries ({} new), {} hand-written, {read} written by the reader",
        entries.len(),
        entries.len() - before,
        hand.len(),
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
pub(crate) fn check_no_name_is_claimed_twice(problems: &mut usize) {
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
