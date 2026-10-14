//! Lobby/room sweeps, header checks and quick looks.
use super::{
    check,
    common::{self, Report},
    devctl::{Devctl, sleep},
    pixels,
    tree::{Node, Tree},
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::path::Path;

// Native clients include Replays between Decks and Settings.
const HEADER_SCREENS: [(&str, &str); 3] = [
    ("play", "Header(Nav(0))"),
    ("decks", "Header(Nav(1))"),
    ("settings", "Header(Nav(3))"),
];
pub(super) fn tiles_per_row(tree: &Tree) -> usize {
    let top = tree
        .0
        .iter()
        .filter(|n| n.kind() == "tile")
        .map(|n| n.y)
        .reduce(f64::min);
    top.map_or(0, |top| {
        tree.0
            .iter()
            .filter(|n| n.kind() == "tile" && (n.y - top).abs() < 2.0)
            .count()
    })
}
fn visible(n: &Node, height: f64) -> bool {
    n.y >= 0.0 && n.bottom() <= height + 1.0
}
pub(super) fn phone_counts(screen: &str, tree: &Tree, height: f64) -> Value {
    match screen {
        "decks" => {
            json!({"tiles_whole":tree.0.iter().filter(|n|n.kind()=="tile").filter(|n|tree.descendants(n.index).into_iter().filter(|n|n.kind()=="hit").max_by(|a,b|a.bottom().total_cmp(&b.bottom())).is_some_and(|n|visible(n,height))).count()})
        }
        "play" => {
            json!({"rows_visible":tree.0.iter().filter(|n|n.kind()=="row" && n.w>200.0 && tree.descendants(n.index).iter().any(|d|!d.text().is_empty()) && visible(n,height)).count()})
        }
        _ => json!({}),
    }
}
struct Case<'a> {
    screen: &'a str,
    w: u32,
    h: u32,
    lang: &'a str,
    step: &'a str,
    touch: bool,
}
fn measure(d: &Devctl, out: &Path, c: &Case<'_>, report: &mut Report) -> Result<()> {
    let tree = d.nodes(None)?;
    let mut faults = check::layout(&tree, f64::from(c.w), c.lang == "de", c.touch, false);
    let input = if c.touch { "touch" } else { "pointer" };
    let tag = format!("{}-{}x{}-{}-{input}-{}", c.screen, c.w, c.h, c.lang, c.step);
    let mut extra = json!({});
    if matches!(c.screen, "decks" | "house") {
        extra["tiles_per_row"] = json!(tiles_per_row(&tree));
    }
    if [(844, 390), (640, 360)].contains(&(c.w, c.h)) {
        for (k, v) in phone_counts(c.screen, &tree, f64::from(c.h))
            .as_object()
            .into_iter()
            .flatten()
        {
            extra[k] = v.clone();
        }
    }
    if c.step == "xl" {
        d.unit_scale()?;
        let path = out.join(format!(
            "{}-{}x{}-{}-{input}.png",
            c.screen, c.w, c.h, c.lang
        ));
        d.screenshot(&path)?;
        let image = pixels::load(&path)?;
        let (mut more, mut contrast) =
            pixels::contrast(&tree, &image, f64::from(c.w), f64::from(c.h))?;
        if c.screen == "create" {
            more.retain(|f| !f.contains(" on header "));
            contrast.retain(|c| c.kind != "header");
        }
        faults.extend(more);
        if let Some(min) = contrast.iter().map(|c| c.ink).reduce(f64::min) {
            extra["ink_min"] = json!(min);
        }
        if let Some(min) = contrast
            .iter()
            .filter(|c| c.kind != "mist")
            .map(|c| c.muted)
            .reduce(f64::min)
        {
            extra["muted_min"] = json!(min);
        }
        if let Some(credit) = pixels::credit(&tree, &image) {
            extra["credit_min"] = json!((credit * 100.0).round() / 100.0);
            if credit < 4.5 {
                faults.push(format!("credit: {credit:.2} < 4.5"));
            }
        }
    }
    report.case(&tag, tree.0.len(), &faults, &extra.to_string());
    Ok(())
}
pub(super) fn sweep(d: &Devctl, out: &Path, which: &str) -> Result<()> {
    common::prepare(out)?;
    // Legacy "all" is the lobby matrix; room needs a separately seated lane.
    let screens: &[&str] = if which == "room" {
        &["room"]
    } else {
        &["play", "decks", "house", "create"]
    };
    let result = (|| {
        let mut report = Report::default();
        for &(w, h) in check::SIZES {
            d.resize(w, h)?;
            sleep(0.8);
            let inputs: &[bool] = if check::TOUCH.contains(&(w, h)) {
                &[false, true]
            } else {
                &[false]
            };
            for lang in ["en", "de"] {
                for &touch in inputs {
                    for step in ["xs", "xl"] {
                        d.knobs(step, lang, touch)?;
                        for &screen in screens {
                            common::go(d, screen, 0.7)?;
                            d.settle(8.0)?;
                            let result = measure(
                                d,
                                out,
                                &Case {
                                    screen,
                                    w,
                                    h,
                                    lang,
                                    step,
                                    touch,
                                },
                                &mut report,
                            );
                            common::cleanup(result, common::leave(d, screen))?;
                        }
                    }
                }
            }
        }
        report.finish(Some(&out.join("screens-summary.txt")), true)
    })();
    common::cleanup(result, common::reset(d))
}
pub(super) fn header(d: &Devctl, out: &Path) -> Result<()> {
    common::prepare(out)?;
    let result = (|| {
        let mut report = Report::default();
        for &(w, h) in check::SIZES {
            d.resize(w, h)?;
            let touch = check::TOUCH.contains(&(w, h));
            for lang in ["en", "de"] {
                d.knobs("l", lang, touch)?;
                for (screen, name) in HEADER_SCREENS {
                    d.tap(name, 1.0)?;
                    let tree = d.nodes(None)?;
                    let mut faults = Vec::new();
                    let heads: Vec<_> = tree.0.iter().filter(|n| n.kind() == "header").collect();
                    if heads.is_empty() {
                        faults.push("missing header; refusing vacuous acceptance".into());
                    }
                    for head in &heads {
                        let sub = tree.subtree(head.index)?;
                        faults.extend(check::overflow(&sub));
                        faults.extend(check::budget(&sub, lang == "de"));
                        faults.extend(check::hit(&sub, touch));
                        faults.extend(check::window(&sub, f64::from(w)));
                    }
                    let tag = format!(
                        "{screen}-{w}x{h}-{lang}-{}",
                        if touch { "touch" } else { "pointer" }
                    );
                    let mut extra = String::new();
                    if common::language_shot(lang, (w, h)) {
                        let path = out.join(format!("{tag}.png"));
                        d.screenshot(&path)?;
                        if lang == "en" {
                            let image = pixels::load(&path)?;
                            let (width, height) = d.size()?;
                            let (more, contrast) = pixels::contrast(&tree, &image, width, height)?;
                            faults.extend(more.into_iter().filter(|s| s.contains("header")));
                            extra = format!(
                                "headers={} contrast={}",
                                heads.len(),
                                serde_json::to_string(
                                    &contrast
                                        .into_iter()
                                        .filter(|c| c.kind == "header")
                                        .collect::<Vec<_>>()
                                )?
                            );
                        }
                    }
                    report.case(&tag, tree.0.len(), &faults, &extra);
                }
            }
        }
        report.finish(Some(&out.join("header-check.log")), false)
    })();
    let clean = (|| {
        d.shell(&json!({"lang":"en","input":"auto","text_size":"l"}))?;
        d.press("Header(Nav(0))")
    })();
    common::cleanup(result, clean)
}
pub(super) fn look(
    d: &Devctl,
    out: &Path,
    screens: &[String],
    sizes: &str,
    touch: bool,
    lang: &str,
) -> Result<()> {
    common::prepare(out)?;
    let sizes = common::sizes(Some(sizes))?;
    d.shell(&json!({"lang":lang,"input":if touch{"touch"}else{"pointer"}}))?;
    for (w, h) in sizes {
        d.resize(w, h)?;
        sleep(0.8);
        for screen in screens {
            common::go(d, screen, 0.8)?;
            sleep(0.6);
            let path = out.join(format!(
                "{screen}-{w}x{h}-{lang}{}.png",
                if touch { "-touch" } else { "" }
            ));
            common::cleanup(d.screenshot(&path), common::leave(d, screen))?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
pub(super) fn probe(
    d: &Devctl,
    screen: &str,
    size: &str,
    lang: &str,
    input: &str,
    step: &str,
    shot: Option<&Path>,
) -> Result<()> {
    let sizes = common::sizes(Some(size))?;
    ensure!(sizes.len() == 1, "probe takes one size");
    let (w, h) = sizes[0];
    d.resize(w, h)?;
    d.shell(&json!({"text_size":step,"lang":lang,"input":input}))?;
    sleep(0.5);
    common::go(d, screen, 0.7)?;
    let result = (|| {
        let tree = d.nodes(None)?;
        let faults = check::layout(&tree, f64::from(w), lang == "de", input == "touch", false);
        for n in tree
            .0
            .iter()
            .filter(|n| n.kind() == "hit" && (n.x < -1.0 || n.right() > f64::from(w) + 1.0))
        {
            let chain = tree
                .ancestors(n.index, None)
                .into_iter()
                .take(6)
                .map(|n| format!("{}{}", n.kind(), n.rect()))
                .collect::<Vec<_>>()
                .join(" ");
            println!("window: {:?} {} <- {chain}", tree.label(n.index), n.rect());
        }
        for f in &faults {
            if !f.starts_with("window") {
                println!("{f}");
            }
        }
        if let Some(path) = shot {
            d.screenshot(path)?;
        }
        ensure!(faults.is_empty(), "{} layout faults", faults.len());
        Ok(())
    })();
    common::cleanup(result, common::leave(d, screen))
}

#[cfg(test)]
mod tests {
    use super::HEADER_SCREENS;

    #[test]
    fn header_targets_match_the_clients_native_nav() {
        let source =
            syn::parse_file(include_str!("../../../baylee-client/src/lobby/header.rs")).unwrap();
        let nav = source
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Const(item) if item.ident == "NAV" => Some(item),
                _ => None,
            })
            .unwrap();
        let syn::Expr::If(nav) = nav.expr.as_ref() else {
            panic!("expected desktop/mobile NAV branches");
        };
        let [syn::Stmt::Expr(syn::Expr::Reference(nav), None)] = nav.then_branch.stmts.as_slice()
        else {
            panic!("expected the native NAV slice");
        };
        let syn::Expr::Array(tabs) = nav.expr.as_ref() else {
            panic!("expected native NAV entries");
        };
        for (screen, press) in HEADER_SCREENS {
            let index: usize = press
                .strip_prefix("Header(Nav(")
                .unwrap()
                .strip_suffix("))")
                .unwrap()
                .parse()
                .unwrap();
            let syn::Expr::Path(tab) = &tabs.elems[index] else {
                panic!("expected a NavTab");
            };
            assert_eq!(
                tab.path
                    .segments
                    .last()
                    .unwrap()
                    .ident
                    .to_string()
                    .to_ascii_lowercase(),
                screen,
                "{press} must open {screen}",
            );
        }
    }
}
