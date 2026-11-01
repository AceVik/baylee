//! Pin reclaim input to the actual card macro and its cached printing.

use super::Mode;
use crate::{Path, fs, ledger, scryfall, stubgen};
use anyhow::Context;
use std::collections::BTreeMap;
use syn::parse::Parser as _;

struct Field {
    name: syn::Ident,
    value: syn::Expr,
}

impl syn::parse::Parse for Field {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        Ok(Self {
            name,
            value: input.parse()?,
        })
    }
}

/// Only actual, unique card fields may select the printing or ledger assignment.
pub(super) fn pinned_printing(
    cache: &Path,
    row: &ledger::LedgerEntry,
    source: &str,
    mode: Mode,
) -> anyhow::Result<scryfall::ScryfallCard> {
    let file = syn::parse_file(source)?;
    let cards: Vec<_> = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Macro(item) if item.mac.path.is_ident("card") => Some(&item.mac),
            _ => None,
        })
        .collect();
    anyhow::ensure!(
        cards.len() == 1,
        "{} must contain exactly one card macro",
        row.name
    );
    let parsed = syn::punctuated::Punctuated::<Field, syn::Token![,]>::parse_terminated
        .parse2(cards[0].tokens.clone())?;
    let mut fields = BTreeMap::new();
    for field in parsed {
        anyhow::ensure!(
            fields.insert(field.name.to_string(), field.value).is_none(),
            "{} has duplicate card fields",
            row.name
        );
    }
    anyhow::ensure!(
        path_is(fields.get("index"), &["index", &row.constant]),
        "{} source index disagrees with ledger",
        row.name
    );
    require_ownership(&fields, source, mode, &row.name)?;
    anyhow::ensure!(
        string(fields.get("oracle_id"))? == row.oracle_id,
        "{} source oracle disagrees with ledger",
        row.name
    );
    let id = string(fields.get("scryfall_id"))?;
    anyhow::ensure!(
        valid_id(&id),
        "{} has an invalid pinned printing id",
        row.name
    );
    anyhow::ensure!(
        crate::printings::header_scryfall_id(source) == Some(id.as_str()),
        "{} printing header disagrees with the card macro",
        row.name
    );
    let pinned = scryfall::printing_cache_path(cache, &id);
    let payload = if pinned.is_file() {
        fs::read_to_string(&pinned)?
    } else {
        fs::read_to_string(cache.join(format!("{}.json", stubgen::slug(&row.name))))
            .with_context(|| format!("missing cached pinned printing {id} for {}", row.name))?
    };
    let printing: scryfall::ScryfallCard = serde_json::from_str(&payload)?;
    anyhow::ensure!(
        printing.id == id,
        "{} cached payload is another printing",
        row.name
    );
    anyhow::ensure!(
        printing.oracle_id.as_deref() == Some(&row.oracle_id),
        "{} cached oracle disagrees with ledger",
        row.name
    );
    anyhow::ensure!(
        printing.name == row.name,
        "{} cached name disagrees with ledger",
        row.name
    );
    Ok(printing)
}

fn require_ownership(
    fields: &BTreeMap<String, syn::Expr>,
    source: &str,
    mode: Mode,
    name: &str,
) -> anyhow::Result<()> {
    match mode {
        Mode::Reclaim => anyhow::ensure!(
            path_is(fields.get("coverage"), &["Coverage", "Implemented"])
                && !source.contains(stubgen::STUB_MARKER),
            "{name} must already be Implemented"
        ),
        Mode::CompleteStub => anyhow::ensure!(
            stubgen::adopt_stub(source).is_some()
                && (!fields.contains_key("coverage")
                    || path_is(fields.get("coverage"), &["Coverage", "Unimplemented"])),
            "{name} must be an unfinished machine-owned stub"
        ),
    }
    Ok(())
}

fn string(expr: Option<&syn::Expr>) -> anyhow::Result<String> {
    if let Some(syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(value),
        ..
    })) = expr
    {
        return Ok(value.value());
    }
    anyhow::bail!("a selected-card identity field is not a string literal")
}

fn path_is(expr: Option<&syn::Expr>, names: &[&str]) -> bool {
    matches!(expr, Some(syn::Expr::Path(path)) if path.qself.is_none()
        && path.path.leading_colon.is_none()
        && path.path.segments.len() == names.len()
        && path.path.segments.iter().zip(names).all(|(segment, name)| segment.ident == *name
            && matches!(segment.arguments, syn::PathArguments::None)))
}

fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(at, byte)| {
            if matches!(at, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}
