//! Read and validate the ledger, core set counts and card pool at one revision.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;

pub(super) const LEDGER: &str = "crates/baylee-cards-index/src/generated.rs";
const CORE_INDEX: &str = "crates/baylee-core/src/generated/index";
const CARDS: &str = "crates/baylee-cards/src/cards";
const UNPLAYABLE: &str = "data/unplayable.tsv";

/// Everything the card status is measured from: a commit that changes none
/// of these has the status of its ancestor (`card_status::fresh`).
pub(super) const INPUTS: [&str; 4] = [LEDGER, CORE_INDEX, CARDS, UNPLAYABLE];
pub(super) const STATUSES: [&str; 6] = [
    "implemented",
    "generated",
    "partial",
    "unimplemented",
    "absent",
    "never",
];

#[derive(Clone, Debug)]
pub(super) struct Row {
    pub index: usize,
    pub oracle: String,
    pub set: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub(super) struct Card {
    pub oracle: String,
    pub printing: Option<String>,
    pub status: String,
    pub note: Option<String>,
    pub path: String,
}

pub(super) struct Catalog {
    pub rows: Vec<Row>,
    pub pool: BTreeMap<String, Card>,
    pub never: BTreeMap<String, String>,
}

pub(super) fn tree(root: &Path, revision: &str) -> Result<(String, BTreeMap<String, String>)> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{revision}^{{commit}}"),
        ])
        .output()?;
    ensure!(output.status.success(), "cannot resolve catalog revision");
    let rev = String::from_utf8(output.stdout)?.trim().to_owned();
    let mut child = Command::new("git")
        .current_dir(root)
        .args([
            "archive",
            "--format=tar",
            &rev,
            "--",
            LEDGER,
            CORE_INDEX,
            CARDS,
            UNPLAYABLE,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let read = (|| -> Result<_> {
        let mut archive = tar::Archive::new(child.stdout.take().context("git archive stdout")?);
        let mut files = BTreeMap::new();
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.header().entry_type().is_file() {
                let path = entry
                    .path()?
                    .to_str()
                    .context("non UTF-8 catalog path")?
                    .to_owned();
                let mut text = String::new();
                entry.read_to_string(&mut text)?;
                files.insert(path, text);
            }
        }
        Ok(files)
    })();
    if read.is_err() {
        let _ = child.kill();
    }
    let output = child.wait_with_output()?;
    ensure!(output.status.success(), "cannot archive catalog revision");
    Ok((rev, read?))
}

pub(super) fn rust_string(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\n') => {
                while chars
                    .peek()
                    .is_some_and(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
                {
                    chars.next();
                }
            }
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(c) => out.push(c),
            None => out.push('\\'),
        }
    }
    out
}

fn partial_reason(text: &str) -> Result<Option<String>> {
    let open = text
        .find('(')
        .context("Partial coverage has no opening parenthesis")?;
    let mut depth = 0usize;
    let mut parts = Vec::new();
    let mut chars = text[open..].chars();
    while let Some(c) = chars.next() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(Some(parts.join(" ")).filter(|s| !s.is_empty()));
                }
            }
            '"' => {
                let mut body = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '"' {
                        closed = true;
                        break;
                    }
                    body.push(c);
                    if c == '\\' {
                        body.push(chars.next().context("truncated Partial string")?);
                    }
                }
                ensure!(closed, "unterminated Partial string");
                parts.push(rust_string(&body).trim().to_owned());
            }
            _ => {}
        }
    }
    bail!("Partial coverage has no closing parenthesis")
}

pub(super) fn read_card(path: &str, text: &str) -> Result<Card> {
    static PATTERNS: OnceLock<[Regex; 3]> = OnceLock::new();
    let [oracle_re, printing_re, coverage_re] = PATTERNS.get_or_init(|| {
        [
            Regex::new(r#"\boracle_id\s*=\s*"([0-9a-f-]{36})""#).expect("static oracle regex"),
            Regex::new(r#"\bscryfall_id\s*=\s*"([0-9a-f-]{36})""#).expect("static printing regex"),
            Regex::new(
                r"\bcoverage\s*=\s*Coverage::(Implemented|Generated|Unimplemented|Partial)\b",
            )
            .expect("static coverage regex"),
        ]
    });
    let oracle = oracle_re
        .captures(text)
        .with_context(|| format!("{path}: no oracle_id"))?[1]
        .to_owned();
    let covs: Vec<_> = coverage_re.captures_iter(text).collect();
    ensure!(covs.len() <= 1, "{path}: {} coverage fields", covs.len());
    let (status, note) = if let Some(cov) = covs.first() {
        match &cov[1] {
            "Implemented" => ("implemented", None),
            "Generated" => ("generated", None),
            "Partial" => (
                "partial",
                partial_reason(&text[cov.get(0).unwrap().end()..])?,
            ),
            _ => ("unimplemented", None),
        }
    } else {
        ensure!(
            text.contains("GENERATED STUB"),
            "{path}: no coverage and no stub marker; read it by hand"
        );
        ("unimplemented", None)
    };
    Ok(Card {
        oracle,
        printing: printing_re.captures(text).map(|c| c[1].to_owned()),
        status: status.into(),
        note,
        path: path.into(),
    })
}

// Ledger, pool and exclusion consistency are validated together before any output.
#[allow(clippy::too_many_lines)]
pub(super) fn parse(files: &BTreeMap<String, String>) -> Result<Catalog> {
    let ledger = files.get(LEDGER).context("ledger missing from revision")?;
    let row_re = Regex::new(
        r#"Row \{ index: CardIndex::new\((\d+)\), oracle_id: "([0-9a-f-]{36})", constant: "[A-Z0-9_]+", set: "([a-z0-9]+)", name: "((?:[^"\\]|\\.)*)" \}"#,
    )?;
    let declared_re = Regex::new(r"pub static ROWS: \[Row; (\d+)\]")?;
    let declared: usize = declared_re
        .captures(ledger)
        .context("ledger declaration missing")?[1]
        .parse()?;
    let mut rows = row_re
        .captures_iter(ledger)
        .map(|c| {
            Ok(Row {
                index: c[1].parse()?,
                oracle: c[2].to_owned(),
                set: c[3].to_owned(),
                name: rust_string(&c[4]),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        rows.len() == declared,
        "ledger: read {} rows, it declares {declared}",
        rows.len()
    );
    rows.sort_by_key(|r| r.index);
    ensure!(
        rows.iter().enumerate().all(|(i, r)| i == r.index),
        "ledger: indices are not 0..n"
    );
    let ids: BTreeSet<_> = rows.iter().map(|r| r.oracle.as_str()).collect();
    ensure!(ids.len() == rows.len(), "ledger: duplicate oracle ids");
    let header_re = Regex::new(r"(?m)^// (\d+) cards? first printed in `([a-z0-9]+)`\.")?;
    let mut headers = BTreeMap::new();
    let mut pool = BTreeMap::<String, Card>::new();
    for (path, text) in files {
        if path.starts_with("crates/baylee-core/src/generated/index/set_") {
            let c = header_re
                .captures(text)
                .with_context(|| format!("{path}: missing set header"))?;
            ensure!(
                headers
                    .insert(c[2].to_owned(), c[1].parse::<usize>()?)
                    .is_none(),
                "duplicate core set header"
            );
        }
        if path.starts_with("crates/baylee-cards/src/cards/")
            && Path::new(path).extension().is_some_and(|ext| ext == "rs")
            && text.contains("card!(")
        {
            let card = read_card(path, text)?;
            ensure!(
                ids.contains(card.oracle.as_str()),
                "{path}: oracle id absent from ledger"
            );
            if let Some(previous) = pool.get(&card.oracle) {
                bail!("{path}: oracle id also in {}", previous.path);
            }
            pool.insert(card.oracle.clone(), card);
        }
    }
    let mut never = BTreeMap::new();
    for line in files
        .get(UNPLAYABLE)
        .context("unplayable list missing")?
        .lines()
    {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let cells: Vec<_> = line.split('\t').collect();
        ensure!(cells.len() == 3, "unplayable: expected three TSV fields");
        let oid = cells[0];
        ensure!(ids.contains(oid), "unplayable {oid} not in ledger");
        ensure!(
            !pool.contains_key(oid),
            "unplayable {} has a card file",
            cells[1]
        );
        ensure!(
            never.insert(oid.to_owned(), cells[2].to_owned()).is_none(),
            "duplicate unplayable oracle id"
        );
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for row in &rows {
        *counts.entry(row.set.clone()).or_default() += 1;
    }
    ensure!(
        headers == counts,
        "sets/counts differ between ledger and core index"
    );
    Ok(Catalog { rows, pool, never })
}
