//! All ten settings sections, phone row counts, and the unsaved profile sheet.
use super::{
    check,
    common::{self, Report},
    devctl::{Devctl, sleep},
    tree::Tree,
};
use anyhow::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
const SECTIONS: &[(&str, &str, &str)] = &[
    ("Graphics", "Graphics", "Grafik"),
    ("Audio", "Audio", "Audio"),
    ("Display", "Display & Interface", "Anzeige & Oberfläche"),
    ("Controls", "Controls", "Steuerung"),
    ("Gameplay", "Gameplay", "Spielablauf"),
    ("Account", "Account", "Konto"),
    ("Network", "Network & Gateway", "Netzwerk & Gateway"),
    ("LanguageModels", "Language models", "Sprachmodelle"),
    ("Updates", "Updates", "Updates"),
    ("Privacy", "Privacy & Data", "Datenschutz & Daten"),
];
pub(super) fn shown(tree: &Tree, title: &str) -> bool {
    let Some(nav) = tree.0.iter().find(|n| n.kind() == "panel") else {
        return false;
    };
    let in_nav: BTreeSet<_> = tree
        .descendants(nav.index)
        .into_iter()
        .map(|n| n.index)
        .collect();
    tree.0
        .iter()
        .any(|n| n.text() == title && !in_nav.contains(&n.index))
}
pub(super) fn rows_in_view(tree: &Tree, height: f64) -> usize {
    tree.0
        .iter()
        .filter(|n| {
            n.kind() == "row"
                && n.y >= 0.0
                && n.bottom() <= height + 0.5
                && tree.owner(n, &["scroll"]).is_none_or(|p| n.inside(p, 1.0))
        })
        .count()
}
fn into_view(d: &Devctl, name: &str) -> Result<()> {
    super::front::into_view(d, name)
}
fn open(d: &Devctl) -> Result<()> {
    if !d.has("Settings(CloseSettings)")? {
        let entry = if d.has("Header(Nav(3))")? {
            "Header(Nav(3))"
        } else {
            "Settings(OpenSettings)"
        };
        d.press_until(entry, || d.has("Settings(CloseSettings)"), 3)?;
        sleep(0.6);
    }
    Ok(())
}
struct Case<'a> {
    w: u32,
    h: u32,
    step: &'a str,
    lang: &'a str,
    touch: bool,
}
fn section(
    d: &Devctl,
    out: &Path,
    c: &Case<'_>,
    section: &str,
    report: &mut Report,
    reached: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<()> {
    let state = d.state()?;
    let tree = Tree::read(&state, None)?;
    let drawn: BTreeSet<_> = d
        .names()?
        .into_iter()
        .filter(|s| s.starts_with("Settings(") && !s.contains("Section("))
        .collect();
    reached
        .entry(section.into())
        .or_default()
        .extend(drawn.iter().cloned());
    let mut faults = check::layout(&tree, f64::from(c.w), c.lang == "de", c.touch, true);
    let tag = format!("{section}-{}x{}-{}-{}", c.w, c.h, c.step, c.lang);
    if (c.w, c.h) == (844, 390) && c.step == "m" {
        let rows = rows_in_view(&tree, f64::from(c.h));
        let navs = tree.0.iter().filter(|n| n.kind() == "menu_item").count();
        if navs < 8 {
            faults.push(format!("phone: section list shows {navs} items"));
        }
        if ["Graphics", "Audio", "Display"].contains(&section) && rows < 3 {
            faults.push(format!("phone: {rows} rows in view, 3 wanted"));
        }
        report.note(format!("    phone: {navs} sections, {rows} rows in view"));
    }
    let mut extra = format!("presses={}", drawn.len());
    if common::medium_shot(c.step, c.lang, (c.w, c.h)) {
        let path = out.join(format!("settings-{tag}.png"));
        if c.lang == "en" {
            let (more, contrast) = common::photograph(d, &tree, &path, None)?;
            faults.extend(more);
            extra.push(' ');
            extra.push_str(&contrast);
        } else {
            d.screenshot(&path)?;
        }
    }
    report.case(&tag, tree.0.len(), &faults, &extra);
    Ok(())
}
fn profile_sheet(d: &Devctl, out: &Path, c: &Case<'_>, report: &mut Report) -> Result<()> {
    if c.step != "m" || !d.has("Settings(Section(LanguageModels))")? {
        return Ok(());
    }
    let title = if c.lang == "en" {
        "Language models"
    } else {
        "Sprachmodelle"
    };
    into_view(d, "Settings(Section(LanguageModels))")?;
    d.press_until(
        "Settings(Section(LanguageModels))",
        || Ok(shown(&d.nodes(None)?, title)),
        3,
    )?;
    d.settle(4.0)?;
    if !d.has("Settings(Seat(Add))")? {
        return Ok(());
    }
    into_view(d, "Settings(Seat(Add))")?;
    d.press_until("Settings(Seat(Add))", || d.has("Settings(CloseProfile)"), 3)?;
    d.settle(4.0)?;
    let result = (|| {
        let tree = d.nodes(None)?;
        let mut faults = check::window(&tree, f64::from(c.w));
        if !d.has("Settings(CloseProfile)")? {
            faults.push("profile sheet did not open".into());
        }
        let tag = format!("llm-sheet-{}x{}-{}", c.w, c.h, c.lang);
        d.screenshot(&out.join(format!("settings-{tag}.png")))?;
        d.key("Escape", None, 0.0)?;
        d.key("Escape", None, 0.5)?;
        if d.has("Settings(CloseProfile)")? {
            faults.push("Esc did not put the sheet away".into());
        }
        report.case(&tag, tree.0.len(), &faults, "");
        Ok(())
    })();
    // No model invocation, and no new profile survives the check.
    let clean = (|| {
        if d.has("Settings(CloseProfile)")? {
            into_view(d, "Settings(CloseProfile)")?;
            d.press_until(
                "Settings(CloseProfile)",
                || Ok(!d.has("Settings(CloseProfile)")?),
                3,
            )?;
        }
        if d.has("Settings(Seat(Revert))")? {
            into_view(d, "Settings(Seat(Revert))")?;
            d.press("Settings(Seat(Revert))")?;
        }
        Ok(())
    })();
    common::cleanup(result, clean)
}
pub(super) fn run(d: &Devctl, out: &Path) -> Result<()> {
    common::prepare(out)?;
    let (sizes, steps) = common::matrix()?;
    let result = (|| {
        let mut report = Report::default();
        let mut reached = BTreeMap::new();
        for (w, h) in sizes {
            for step in &steps {
                for lang in ["en", "de"] {
                    let (width, height) = d.size()?;
                    if (width - f64::from(w)).abs() > 0.01 || (height - f64::from(h)).abs() > 1.0 {
                        d.resize(w, h)?;
                        sleep(1.0);
                    }
                    let c = Case {
                        w,
                        h,
                        step,
                        lang,
                        touch: check::TOUCH.contains(&(w, h)),
                    };
                    d.knobs(step, lang, c.touch)?;
                    sleep(0.6);
                    open(d)?;
                    for &(name, en, de) in SECTIONS {
                        let press = format!("Settings(Section({name}))");
                        if !d.has(&press)? {
                            report.note(format!("{name}-{w}x{h}-{lang}: not offered"));
                            continue;
                        }
                        into_view(d, &press)?;
                        let title = if lang == "en" { en } else { de };
                        d.press_until(&press, || Ok(shown(&d.nodes(None)?, title)), 3)?;
                        d.settle(4.0)?;
                        section(d, out, &c, name, &mut report, &mut reached)?;
                    }
                    profile_sheet(d, out, &c, &mut report)?;
                }
            }
        }
        for (name, drawn) in reached {
            report.expect(!drawn.is_empty(), &format!("{name} drew controls"));
            report.note(format!(
                "reached {name}: {} presses: {}",
                drawn.len(),
                drawn.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        report.finish(Some(&out.join("settings-check.log")), false)
    })();
    let clean = (|| {
        common::reset(d)?;
        if d.has("Settings(CloseSettings)")? {
            into_view(d, "Settings(CloseSettings)")?;
            d.press("Settings(CloseSettings)")?;
        }
        Ok(())
    })();
    common::cleanup(result, clean)
}
