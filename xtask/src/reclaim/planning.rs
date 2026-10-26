//! Offline identity preflight and rendering of explicit card selections.

use super::{CardPlan, Mode, Plan, identity};
use crate::{BTreeSet, Path, fs, ledger, scriptgen, scripts, stubgen};
use anyhow::Context;

struct RenderContext<'a> {
    root: &'a Path,
    cache: &'a Path,
    cards_dir: &'a Path,
    ledger: &'a ledger::IndexLedger,
    cats: &'a crate::catalog::SubtypeCatalogs,
    cycles: &'a crate::layout::LandCycles,
    holds: &'a crate::coverage_holds::CoverageHolds,
    lookup: &'a scriptgen::ScriptLookup,
    tokens: Option<&'a crate::tokengen::TokenLookup>,
    mode: Mode,
}

/// Preserve the tested-card-only contract of ordinary reclamation.
pub(super) fn plan(
    root: &Path,
    cache: &Path,
    scripts: &Path,
    names: &[String],
) -> anyhow::Result<Plan> {
    plan_with_mode(root, cache, scripts, names, Mode::Reclaim)
}

/// Resolve and render the whole selection before any card can be written.
pub(super) fn plan_with_mode(
    root: &Path,
    cache: &Path,
    scripts: &Path,
    names: &[String],
    mode: Mode,
) -> anyhow::Result<Plan> {
    anyhow::ensure!(
        !names.is_empty(),
        "select at least one card to {}",
        mode.label()
    );
    let names: BTreeSet<_> = names.iter().collect();
    let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS)?;
    let completing = if mode == Mode::CompleteStub {
        names
            .iter()
            .map(|name| Ok(ledger.entry_named(name)?.oracle_id.clone()))
            .collect::<anyhow::Result<BTreeSet<_>>>()?
    } else {
        BTreeSet::new()
    };
    let holds = crate::coverage_holds::load_completing_stubs(root, &completing)?;
    let cards_dir = root.join("crates/baylee-cards/src/cards");
    let files = crate::card_files(&cards_dir)?;
    let cats = crate::hold_back::compiled_catalogs();
    let cycles =
        crate::layout::LandCycles::parse(&fs::read_to_string(root.join("data/land-cycles.tsv"))?);
    let scripts_dir = crate::scripts_root(root, scripts);
    anyhow::ensure!(
        scripts_dir.is_dir(),
        "{} requires a local card-script corpus",
        mode.label()
    );
    let lookup =
        scriptgen::ScriptLookup::new(scripts_dir.clone(), scripts::build_index(&scripts_dir)?);
    let tokens = crate::tokengen::TokenLookup::beside(&scripts_dir)?;
    let cache = root.join(cache);
    let context = RenderContext {
        root,
        cache: &cache,
        cards_dir: &cards_dir,
        ledger: &ledger,
        cats: &cats,
        cycles: &cycles,
        holds: &holds,
        lookup: &lookup,
        tokens: tokens.as_ref(),
        mode,
    };
    let mut cards = Vec::new();
    let mut selected = BTreeSet::new();
    for name in names {
        let row = ledger.entry_named(name)?;
        if selected.insert(row.index) {
            let path = files
                .get(&crate::front_face_slug(&row.name))
                .with_context(|| format!("no card file for {name}"))?;
            cards.push(context.card(row, path)?);
        }
    }
    let sources: Vec<_> = cards.iter().map(|card| card.generated.clone()).collect();
    for (card, formatted) in cards.iter_mut().zip(crate::files::format_rust_many(
        &root.join("target"),
        &sources,
    )?) {
        syn::parse_file(&formatted).context("generated source must be valid Rust syntax")?;
        card.generated = formatted;
    }
    Ok(Plan { cards, mode })
}

impl RenderContext<'_> {
    fn card(&self, row: &ledger::LedgerEntry, path: &Path) -> anyhow::Result<CardPlan> {
        let original = fs::read_to_string(path)?;
        let printing = identity::pinned_printing(self.cache, row, &original, self.mode)?;
        let slug = crate::front_face_slug(&row.name);
        let canonical = crate::layout::path_for(&printing, &slug, self.cycles);
        anyhow::ensure!(
            path == self.cards_dir.join(&canonical).as_path(),
            "{} is not at its canonical path {canonical}; {} never moves cards",
            row.name,
            self.mode.label()
        );
        let resolved_path = self.cards_dir.canonicalize()?.join(&canonical);
        anyhow::ensure!(
            path.canonicalize()? == resolved_path,
            "{} source path resolves outside its canonical card location",
            row.name
        );
        let tested = self
            .root
            .join("crates/baylee-engine/src/engine/card_tests")
            .join(&canonical)
            .is_file();
        let implemented = self.holds.accepts_tests(&row.oracle_id, tested);
        anyhow::ensure!(
            self.mode == Mode::CompleteStub || implemented,
            "{} needs accepted canonical card tests",
            row.name
        );
        let script = self
            .lookup
            .script(&row.name)
            .with_context(|| format!("{} has no reference script", row.name))?;
        anyhow::ensure!(
            scriptgen::transcode(&script, self.cats, self.tokens).is_some()
                || (self.mode == Mode::CompleteStub && scriptgen::is_vanilla(&script)),
            "{} is not fully read: {:?}",
            row.name,
            scriptgen::refusal_reason(&script, self.cats, self.tokens)
        );
        let (info, generated) = stubgen::render_stub(
            &printing,
            row,
            self.ledger,
            self.cats,
            Some(self.lookup),
            self.tokens,
            self.cycles,
            &|_| implemented,
        )?;
        anyhow::ensure!(
            info.path == canonical && info.index == row.index && info.oracle_id == row.oracle_id,
            "{} generated identity or path changed",
            row.name
        );
        let coverage = if implemented {
            "Coverage::Implemented"
        } else {
            "Coverage::Generated"
        };
        anyhow::ensure!(
            generated.contains(stubgen::OWNED_MARKER)
                && !generated.contains(stubgen::STUB_MARKER)
                && crate::knob(&generated, "coverage").is_some_and(|v| v.starts_with(coverage)),
            "{} would not retain the required fully read coverage",
            row.name
        );
        Ok(CardPlan {
            path: path.to_path_buf(),
            resolved_path,
            original,
            generated,
            implemented,
        })
    }
}
