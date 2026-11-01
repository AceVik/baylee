//! The card catalog an exported model was trained on, bound to the one this
//! build compiles (`docs/trained-ai.md` §"Card catalog compatibility").
use crate::cardstructure::{Forest, compat, digest, refs::RuleRef};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// How the catalog a model was exported with must relate to the catalog
/// this build compiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogPolicy4 {
    /// The same catalog, digest for digest.
    #[default]
    Exact,
    /// This build may add cards and tokens. Every rule the model knows must
    /// be unchanged; references to added rules are left out, as a card's
    /// are when the encoder cannot see it.
    Superset,
    /// Indices stable, structures may drift: as [`Self::Superset`], and a
    /// card or token whose rules changed since the export is withdrawn. The
    /// export's rule table holds only the trees it was trained on, so a
    /// drifted card has no row for its current rules; all its references are
    /// left out as an added card's are, instead of reading the old tree. Its
    /// DSL summary is computed from its current rules at run time, as every
    /// card's is.
    Drift,
}

/// What a model's catalog was bound to, for run metadata.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CatalogBinding4 {
    /// The policy it was bound under.
    pub policy: CatalogPolicy4,
    /// Digest of the model's own `card_structure.json`.
    pub model_structure_sha256: String,
    /// Digest of this build's compiled catalog.
    pub catalog_sha256: String,
    /// Whether the two are the same catalog.
    pub identical: bool,
    /// Rule roots only this build has.
    pub new_roots: usize,
    /// Rule operators only this build uses.
    pub new_tags: usize,
    /// Cards this build has and the export did not: indices appended since.
    pub appended_cards: usize,
    /// Registry tokens appended since the export.
    pub appended_tokens: usize,
    /// Rule roots the export knows whose tree changed or is gone.
    pub changed_roots: usize,
    /// Cards and tokens whose references are left out because their rules
    /// changed since the export ([`CatalogPolicy4::Drift`]).
    pub withdrawn: Vec<DriftedRule4>,
}

/// A card or token whose rules drifted since a model's export.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DriftedRule4 {
    /// `card/N` (its `CardIndex`) or `token/N` (its registry slot).
    pub owner: String,
    /// Its name in this build.
    pub name: Option<String>,
}

impl DriftedRule4 {
    fn of(owner: String) -> Self {
        Self {
            name: compat::owner_name(&owner),
            owner,
        }
    }
}

/// The model's rule roots as this build reads them.
#[derive(Debug)]
pub(super) struct RuleRoots {
    /// Every key the model reads, with its node in the model's own DAG.
    roots: BTreeMap<String, u32>,
    /// Keys whose references are left out: rules the model never saw, or
    /// rules withdrawn because they changed.
    unknown: BTreeSet<String>,
    pub(super) binding: CatalogBinding4,
}

/// Reads a model's catalog, checks it against its metadata digest, and
/// binds it to this build's catalog under `policy`.
///
/// # Errors
/// When the manifest is not the model's, or the catalogs do not relate as
/// `policy` requires.
pub(super) fn load(
    path: &Path,
    expected: &str,
    policy: CatalogPolicy4,
) -> anyhow::Result<RuleRoots> {
    let forest: Forest = serde_json::from_slice(&std::fs::read(path)?)?;
    anyhow::ensure!(
        forest.version == 1 && digest(&forest)? == expected,
        "card structure manifest mismatch"
    );
    // Rule edits must not silently reuse embeddings learned for previous rules.
    let current = crate::cardstructure::catalog(
        &baylee_cards::all()
            .map(|c| c.index.get())
            .collect::<Vec<_>>(),
    )?;
    bind(forest, &current, expected, policy)
}

/// [`load`] past reading: `model` is the exported catalog, `current` this
/// build's.
fn bind(
    model: Forest,
    current: &Forest,
    expected: &str,
    policy: CatalogPolicy4,
) -> anyhow::Result<RuleRoots> {
    let catalog_sha256 = digest(current)?;
    let mut binding = CatalogBinding4 {
        policy,
        model_structure_sha256: expected.to_owned(),
        identical: catalog_sha256 == expected,
        catalog_sha256,
        new_roots: 0,
        new_tags: 0,
        appended_cards: 0,
        appended_tokens: 0,
        changed_roots: 0,
        withdrawn: Vec::new(),
    };
    if binding.identical {
        return Ok(RuleRoots {
            roots: model.roots,
            unknown: BTreeSet::new(),
            binding,
        });
    }
    anyhow::ensure!(
        policy != CatalogPolicy4::Exact,
        "compiled card rules differ from exported model; export card_structure4, refresh_structure4.py, then export_onnx4.py \
         (or accept a superset catalog explicitly: docs/trained-ai.md §\"Card catalog compatibility\")"
    );
    anyhow::ensure!(
        compat::same_contract(&model, current),
        "the card structure contract differs; the catalogs cannot be compared"
    );
    let report = compat::superset(&model, current);
    binding.new_roots = report.new_roots.len();
    binding.new_tags = report.new_tags.len();
    binding.changed_roots = report.changed.len() + report.missing.len();
    let appended = compat::appended(&report, &model);
    binding.appended_cards = appended.iter().filter(|o| o.starts_with("card/")).count();
    binding.appended_tokens = appended.iter().filter(|o| o.starts_with("token/")).count();
    if policy == CatalogPolicy4::Superset {
        anyhow::ensure!(
            report.holds(),
            "{} rules the model knows changed and {} are gone (first: {:?}); \
             bind with drift (eval4 --catalog-drift) or re-export the model",
            report.changed.len(),
            report.missing.len(),
            report.changed.iter().chain(&report.missing).next()
        );
    }
    let owners = compat::withdrawn(&report, &model);
    let withdrawn: BTreeSet<&str> = owners.iter().map(String::as_str).collect();
    let mut roots = model.roots;
    let mut unknown: BTreeSet<String> = report.new_roots.into_iter().collect();
    roots.retain(|key, _| {
        let keep = !withdrawn.contains(compat::owner(key).as_str());
        if !keep {
            unknown.insert(key.clone());
        }
        keep
    });
    binding.withdrawn = owners.into_iter().map(DriftedRule4::of).collect();
    Ok(RuleRoots {
        roots,
        unknown,
        binding,
    })
}

/// Packs rule references into `[count, width, (node, role)]`, leaving out
/// references to rules the model does not know. Returns the width, the
/// values and how many references were left out.
///
/// # Errors
/// When a row would be truncated, a reference names a key neither catalog
/// has, or a role is invalid.
pub(super) fn pack(
    rows: &[Vec<RuleRef>],
    count: usize,
    roots: &RuleRoots,
) -> anyhow::Result<(usize, Vec<i64>, u64)> {
    anyhow::ensure!(
        rows.len() <= count,
        "rule reference rows would be truncated"
    );
    let mut left_out = 0_u64;
    let mut known: Vec<Vec<(u32, u8)>> = Vec::with_capacity(rows.len());
    for refs in rows {
        let mut row = Vec::with_capacity(refs.len());
        for r in refs {
            anyhow::ensure!((1..=7).contains(&r.role), "invalid rule relation");
            match roots.roots.get(&r.key) {
                Some(&node) => row.push((node, r.role)),
                None if roots.unknown.contains(&r.key) => left_out += 1,
                None => anyhow::bail!("unrepresented rule: {}", r.key),
            }
        }
        known.push(row);
    }
    let width = known.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut values = vec![0; count * width * 2];
    for (i, refs) in known.iter().enumerate() {
        for (j, &(node, role)) in refs.iter().enumerate() {
            values[(i * width + j) * 2] = i64::from(node);
            values[(i * width + j) * 2 + 1] = i64::from(role);
        }
    }
    Ok((width, values, left_out))
}

#[cfg(test)]
mod tests;
