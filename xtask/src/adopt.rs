//! `adopt` and `refresh-oracle`: taking a card off the machine, and
//! rewriting the headers people must not type.

use crate::{
    Path, Pinned, acceptance, cached_printing, card_files, front_face_slug, fs, pinned_printing,
    printed_text, relative, stubgen,
};

/// Transfers one unfinished stub through codegen without changing coverage.
pub(crate) fn adopt_stub_card(root: &Path, name: &str) -> anyhow::Result<()> {
    let cards_dir = root.join("crates/baylee-cards/src/cards");
    let files = card_files(&cards_dir)?;
    let slug = front_face_slug(name);
    let path = files
        .get(&slug)
        .ok_or_else(|| anyhow::anyhow!("no card file for {name}"))?;
    let text = fs::read_to_string(path)?;
    let adopted = stubgen::adopt_stub(&text)
        .ok_or_else(|| anyhow::anyhow!("{name} is not an unfinished generated stub"))?;
    fs::write(path, adopted)?;
    println!("adopted unfinished {name}; implement and test it before claiming coverage");
    Ok(())
}

/// Strips the ownership marker from one generated card, handing the file to
/// whoever asked for it.
///
/// The marker line is rewritten rather than deleted, keeping the summary the
/// reader wrote: what the card does is still true, and what changes is only
/// who may say it from now on. A stub is refused — there is nothing to adopt
/// in a card nobody has implemented, and taking a stub off the machine would
/// freeze it as an `Unimplemented` card codegen can never finish.
pub(crate) fn adopt(root: &Path, name: &str) -> anyhow::Result<()> {
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
pub(crate) fn refresh_oracle(root: &Path, dry_run: bool) -> anyhow::Result<()> {
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
        // from" — and the two disagreeing is a header naming one printing
        // and quoting another. What it does *not* do is change which printing
        // that is: re-pinning a card is an editorial act, and this command
        // refreshes what a printing says.
        let mut next = text.clone();
        if let Some(oracle) = with_oracle_header(&next, &printed_text(&payload)) {
            next = oracle;
        }
        // From the card's **own** printing, so a refresh never re-pins it.
        // Writing the default's line here would move the header to another
        // piece of cardboard and leave `scryfall_id` in the body naming the
        // old one — trading a set-line finding for an identity one, on a
        // card that was right all along.
        let pinned = pinned_printing(root, &text, &payload);
        let printing = match &pinned {
            Pinned::Default(card) => Some(*card),
            Pinned::Other(card) => Some(card),
            Pinned::Missing => None,
        };
        if let Some(printing) = printing
            && let Some(line) = set_header_line(printing)
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
pub(crate) fn set_header_line(payload: &serde_json::Value) -> Option<String> {
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
pub(crate) fn with_set_header(text: &str, line: &str) -> Option<String> {
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
pub(crate) fn with_oracle_header(text: &str, printed: &str) -> Option<String> {
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
