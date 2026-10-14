//! One catalogue query per deck, with explicit printing choices preserved.
use std::{
    collections::BTreeMap,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use std::fmt::Write as _;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use super::lists::Row;

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Printing {
    #[serde(rename = "set", alias = "set_code")]
    pub set: String,
    #[serde(alias = "collector_number")]
    pub number: String,
    pub lang: String,
    pub nonfoil: bool,
    pub year: String,
    #[serde(default)]
    pub set_lost: bool,
}

#[derive(Serialize)]
pub(super) struct Resolved {
    #[serde(flatten)]
    pub row: Row,
    pub resolved: Option<Printing>,
}

#[derive(Deserialize)]
struct Hit {
    i: usize,
    #[serde(flatten)]
    printing: Printing,
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sql(rows: &[Row], lang: &str, oldest: bool, nonfoil: bool) -> String {
    let values = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            format!(
                "({i},{},{},{})",
                quote(&row.name),
                row.set.as_deref().map_or_else(|| "NULL".to_owned(), quote),
                row.number
                    .as_deref()
                    .map_or_else(|| "NULL".to_owned(), quote)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let order = format!(
        "(lower(s.names->>'en') = lower(w.name)) DESC, (c.lang = {}) DESC, (c.lang = 'en') DESC, {} c.released_at {}, c.collector_number, c.scryfall_id",
        quote(lang),
        if nonfoil {
            "(c.finishes LIKE '%nonfoil%') DESC,"
        } else {
            ""
        },
        if oldest { "ASC" } else { "DESC" }
    );
    format!(
        "SET standard_conforming_strings = on; SET statement_timeout = '45s';\nWITH want(i,name,setc,num) AS (VALUES {values}), hit AS (SELECT DISTINCT ON (w.i) w.i, c.set_code AS \"set\", c.collector_number AS number, c.lang, c.finishes LIKE '%nonfoil%' AS nonfoil, to_char(c.released_at,'YYYY') AS year, false AS set_lost FROM want w JOIN card_search s ON s.face_index = 0 AND lower(s.names->>'en') IN (lower(w.name),lower(split_part(w.name,' // ',1))) JOIN cards c ON c.oracle_id = s.oracle_id WHERE (w.setc IS NULL OR lower(c.set_code) = lower(w.setc)) AND (w.num IS NULL OR c.collector_number = w.num) ORDER BY w.i, {order}) SELECT row_to_json(hit) FROM hit ORDER BY i;"
    )
}

fn parse_hits(text: &str, rows: &[Row]) -> Result<BTreeMap<usize, Printing>> {
    let mut hits = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let hit: Hit = serde_json::from_str(line).context("invalid catalogue result row")?;
        ensure!(hit.i < rows.len(), "catalogue returned unknown row");
        let row = &rows[hit.i];
        ensure!(
            row.set
                .as_ref()
                .is_none_or(|set| set.eq_ignore_ascii_case(&hit.printing.set)),
            "catalogue changed an explicit set"
        );
        ensure!(
            row.number
                .as_ref()
                .is_none_or(|number| number == &hit.printing.number),
            "catalogue changed an explicit collector number"
        );
        ensure!(!hit.printing.set_lost, "catalogue lost explicit printing");
        for field in [
            &hit.printing.set,
            &hit.printing.number,
            &hit.printing.lang,
            &hit.printing.year,
        ] {
            ensure!(
                !field.is_empty() && !field.chars().any(|c| c.is_control() || "()[]".contains(c)),
                "invalid catalogue printing field"
            );
        }
        ensure!(
            hits.insert(hit.i, hit.printing).is_none(),
            "duplicate catalogue row"
        );
    }
    Ok(hits)
}

pub(super) fn resolve(
    root: &Path,
    rows: Vec<Row>,
    lang: &str,
    oldest: bool,
    nonfoil: bool,
    leave_open: bool,
) -> Result<Vec<Resolved>> {
    ensure!(
        !lang.is_empty() && lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "invalid catalogue language"
    );
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let query = sql(&rows, lang, oldest, nonfoil);
    let mut input = tempfile::tempfile()?;
    std::io::Write::write_all(&mut input, query.as_bytes())?;
    std::io::Seek::rewind(&mut input)?;
    let mut output = tempfile::tempfile()?;
    let mut child = Command::new("docker")
        .args([
            "compose",
            "exec",
            "-T",
            "postgres",
            "psql",
            "-X",
            "-q",
            "-U",
            "baylee",
            "-d",
            "baylee",
            "-tA",
            "-v",
            "ON_ERROR_STOP=1",
        ])
        .current_dir(root)
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::null())
        .spawn()
        .context("cannot start catalogue query (docker compose postgres)")?;
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline || output.metadata()?.len() > 16 * 1024 * 1024 {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("catalogue query exceeded deadline or output limit");
        }
        thread::sleep(Duration::from_millis(25));
    };
    ensure!(
        status.success(),
        "catalogue query failed; database stderr suppressed"
    );
    std::io::Seek::rewind(&mut output)?;
    let mut text = String::new();
    output
        .take(16 * 1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    ensure!(text.len() <= 16 * 1024 * 1024, "catalogue output too large");
    let mut found = parse_hits(&text, &rows)?;
    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let resolved = if leave_open && row.set.is_none() {
                None
            } else {
                found.remove(&i)
            };
            Resolved { row, resolved }
        })
        .collect())
}

pub(super) fn deck_row(row: &Resolved) -> String {
    let mut line = format!(
        "{} {}",
        row.row.count,
        row.row.name.split(" // ").next().unwrap_or_default()
    );
    if let Some(printing) = &row.resolved {
        write!(
            line,
            " ({}) {}",
            printing.set.to_uppercase(),
            printing.number
        )
        .expect("writing to a String cannot fail");
        if printing.lang != "en" {
            write!(line, " [{}]", printing.lang).expect("writing to a String cannot fail");
        }
        if row.row.foil && !printing.nonfoil {
            line.push_str(" *F*");
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_sql_and_retains_explicit_printing_filters() {
        let rows = super::super::lists::friend_list("1x O'Name (ABC:12)").unwrap();
        let query = sql(&rows, "de", true, true);
        assert!(query.contains("'O''Name'"));
        assert!(query.contains("WHERE (w.setc IS NULL OR lower(c.set_code) = lower(w.setc))"));
        assert!(query.contains("c.collector_number = w.num"));
        assert!(query.contains("(c.lang = 'en') DESC"));
        assert!(query.contains("released_at ASC"));
    }
    #[test]
    fn rejects_missing_fields_duplicates_and_changed_printings() {
        let rows = super::super::lists::friend_list("1x Front / Back (ABC:12) f").unwrap();
        let valid = r#"{"i":0,"set":"abc","number":"12","lang":"de","nonfoil":true,"year":"2001","set_lost":false}"#;
        let mut hits = parse_hits(valid, &rows).unwrap();
        assert_eq!(
            deck_row(&Resolved {
                row: rows[0].clone(),
                resolved: hits.remove(&0)
            }),
            "1 Front (ABC) 12 [de]"
        );
        assert!(parse_hits(&valid.replace("\"12\"", "\"13\""), &rows).is_err());
        assert!(parse_hits(&format!("{valid}\n{valid}"), &rows).is_err());
        assert!(parse_hits("{}", &rows).is_err());
    }
}
