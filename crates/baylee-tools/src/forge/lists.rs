//! Deck request parsers and deck-row writers.
use std::{collections::BTreeMap, fmt::Write as _};

use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(super) struct Row {
    pub count: u32,
    pub name: String,
    pub foil: bool,
    pub set: Option<String>,
    pub number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<String>,
}

fn validate(row: &Row) -> Result<()> {
    ensure!(row.count > 0, "card count must be positive");
    ensure!(
        row.name.len() <= 1024
            && row.set.as_ref().is_none_or(|s| s.len() <= 64)
            && row.number.as_ref().is_none_or(|s| s.len() <= 128),
        "card request field exceeds size limit"
    );
    ensure!(
        !row.name.is_empty() && !row.name.chars().any(char::is_control),
        "invalid card name"
    );
    for value in [&row.set, &row.number].into_iter().flatten() {
        ensure!(
            !value.chars().any(|c| c.is_control() || "()[]|".contains(c)),
            "invalid printing field"
        );
    }
    Ok(())
}

pub(super) fn forge_row(text: &str) -> Result<Row> {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = PATTERN.get_or_init(|| {
        Regex::new(r"^(\d+) (.+?)(\+?)\|([^|]*)\|\[([^\]]*)\](?:\|\$\{(.*)\})?$")
            .expect("constant regex")
    });
    let m = re
        .captures(text)
        .context("row is not a composed card request")?;
    let row = Row {
        count: m[1].parse()?,
        name: m[2].to_owned(),
        foil: !m[3].is_empty(),
        set: Some(m[4].to_owned()),
        number: Some(m[5].to_owned()),
        flags: m
            .get(6)
            .filter(|s| !s.as_str().is_empty())
            .map(|s| s.as_str().to_owned()),
    };
    validate(&row)?;
    Ok(row)
}

pub(super) fn friend_list(text: &str) -> Result<Vec<Row>> {
    ensure!(
        text.lines().count() <= 100_000,
        "friend list exceeds 100000-line limit"
    );
    let re = Regex::new(r"^(\d+)x\s+(.+?)(?:\s+\(([^):]+)(?::([^)]+))?\))?(?:\s+(f))?$")?;
    text.lines()
        .enumerate()
        .filter_map(|(line, text)| {
            let text = text.trim();
            if text.is_empty() || text.starts_with('#') {
                return None;
            }
            Some((|| {
                let m = re
                    .captures(text)
                    .with_context(|| format!("not a decklist line at {}", line + 1))?;
                let row = Row {
                    count: m[1].parse()?,
                    name: m[2].trim().replace(" / ", " // "),
                    set: m
                        .get(3)
                        .filter(|s| s.as_str() != "000")
                        .map(|s| s.as_str().to_uppercase()),
                    number: m.get(4).map(|s| s.as_str().to_owned()),
                    foil: m.get(5).is_some(),
                    flags: None,
                };
                validate(&row)?;
                Ok(row)
            })())
        })
        .collect()
}

pub(super) fn deck_row(request: &str, codes: &BTreeMap<String, String>) -> Result<String> {
    let row = forge_row(request)?;
    let code = row.set.as_deref().unwrap_or_default();
    let code = codes.get(code).map_or(code, String::as_str).to_uppercase();
    let mut text = format!("{} {}", row.count, row.name);
    if !code.is_empty() {
        write!(text, " ({code})")?;
        if let Some(number) = row.number.as_deref().filter(|n| !n.is_empty()) {
            write!(text, " {number}")?;
        }
    }
    if row.foil {
        text.push_str(" *F*");
    }
    Ok(text)
}

pub(super) fn slug(name: &str) -> Result<String> {
    let slug = name.to_lowercase().replace(' ', "-");
    ensure!(
        !slug.is_empty() && slug != "." && slug != "..",
        "empty or unsafe deck filename"
    );
    ensure!(
        !slug.chars().any(|c| c.is_control() || "/\\:[]".contains(c)),
        "deck name cannot be used as a filename"
    );
    Ok(slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_trackers_without_inventing_a_printing() {
        let rows = friend_list("# comment\n1x Front / Back (000)\n2x Oak (abc:12) f\n").unwrap();
        assert_eq!(rows[0].name, "Front // Back");
        assert!(rows[0].set.is_none());
        assert_eq!(rows[1].set.as_deref(), Some("ABC"));
        assert!(rows[1].foil);
        assert!(friend_list("Oak").is_err());
        assert!(friend_list("0x Oak").is_err());
    }
    #[test]
    fn translates_codes_flags_and_foil() {
        let codes = BTreeMap::from([("OLD".to_owned(), "new".to_owned())]);
        assert_eq!(
            deck_row("2 Oak+|OLD|[12]|${art=2}", &codes).unwrap(),
            "2 Oak (NEW) 12 *F*"
        );
        assert_eq!(forge_row("2 Oak|ABC|[12]").unwrap().count, 2);
        assert!(forge_row("2 Oak|ABC|[12] garbage").is_err());
        assert!(slug("../../outside").is_err());
        assert!(slug("Oak\n[sideboard]").is_err());
        assert_eq!(slug("My Deck").unwrap(), "my-deck");
    }
}
