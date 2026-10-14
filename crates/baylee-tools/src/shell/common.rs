//! Shared scenario reporting and navigation; assertions accumulate, I/O errors stop.
use super::{
    check,
    devctl::{Devctl, sleep},
    pixels,
    tree::Tree,
};
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Default)]
pub(super) struct Report {
    pub lines: Vec<String>,
    pub failures: usize,
}
impl Report {
    pub fn note(&mut self, line: String) {
        println!("{line}");
        self.lines.push(line);
    }
    pub fn expect(&mut self, condition: bool, what: &str) {
        self.failures += usize::from(!condition);
        self.note(format!(
            "{} {what}",
            if condition { "PASS" } else { "FAIL" }
        ));
    }
    pub fn case(&mut self, tag: &str, nodes: usize, faults: &[String], extra: &str) {
        self.failures += faults.len();
        self.note(format!(
            "{tag}: {nodes} nodes, {} faults {extra}",
            faults.len()
        ));
        // Keep every fault in the artifact; bound terminal noise only.
        for (i, f) in faults.iter().enumerate() {
            let line = format!("    {f}");
            if i < 12 {
                println!("{line}");
            }
            self.lines.push(line);
        }
    }
    pub fn finish(&self, path: Option<&Path>, append: bool) -> Result<()> {
        let summary = format!("{}\nfailures: {}\n", self.lines.join("\n"), self.failures);
        if let Some(path) = path {
            if append {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?
                    .write_all(summary.as_bytes())?;
            } else {
                fs::write(path, summary)?;
            }
        }
        println!("failures: {}", self.failures);
        ensure!(
            self.failures == 0,
            "{} shell assertions failed",
            self.failures
        );
        Ok(())
    }
}
pub(super) fn prepare(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    Ok(())
}
pub(super) fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
pub(super) fn reset(d: &Devctl) -> Result<()> {
    d.shell(&json!({"text_size":"m","lang":"en","input":"auto"}))
}
pub(super) fn cleanup<T>(result: Result<T>, clean: Result<()>) -> Result<T> {
    match (result, clean) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) | (Ok(_), Err(e)) => Err(e),
        (Err(e), Err(c)) => Err(e.context(format!("cleanup also failed: {c}"))),
    }
}
pub(super) fn go(d: &Devctl, screen: &str, wait: f64) -> Result<()> {
    let steps: &[&str] = match screen {
        "play" => &["Header(Nav(0))"],
        "decks" => &["Header(Nav(1))", "Decks(Tab(Mine))"],
        "house" => &["Header(Nav(1))", "Decks(Tab(House))"],
        "create" => &["Header(Nav(0))", "Play(CreateTable)"],
        "room" => &[],
        _ => bail!("unknown screen {screen}"),
    };
    for step in steps {
        d.tap(step, wait)?;
    }
    d.settle(8.0)
}
pub(super) fn leave(d: &Devctl, screen: &str) -> Result<()> {
    if screen == "create" {
        d.key("Escape", None, 0.5)?;
    }
    Ok(())
}
pub(super) fn sizes(input: Option<&str>) -> Result<Vec<(u32, u32)>> {
    let Some(input) = input else {
        return Ok(check::SIZES.to_vec());
    };
    let out: Result<Vec<_>> = input
        .split(',')
        .map(|s| {
            let (w, h) = s
                .split_once('x')
                .ok_or_else(|| anyhow::anyhow!("size must be WIDTHxHEIGHT"))?;
            let w: u32 = w.parse()?;
            let h: u32 = h.parse()?;
            ensure!(w > 0 && h > 0 && w <= 16384 && h <= 16384, "invalid size");
            Ok((w, h))
        })
        .collect();
    let out = out?;
    ensure!(!out.is_empty(), "empty size matrix");
    Ok(out)
}
type Matrix = (Vec<(u32, u32)>, Vec<String>);
pub(super) fn matrix() -> Result<Matrix> {
    let size_env = std::env::var("SHELL_SIZES").ok();
    let sizes = sizes(size_env.as_deref())?;
    let steps: Vec<_> = std::env::var("SHELL_STEPS")
        .unwrap_or_else(|_| "xs,m,xl".into())
        .split(',')
        .map(str::to_owned)
        .collect();
    ensure!(
        steps
            .iter()
            .all(|s| ["xs", "s", "m", "l", "xl"].contains(&s.as_str())),
        "invalid SHELL_STEPS"
    );
    Ok((sizes, steps))
}
pub(super) fn language_shot(lang: &str, size: (u32, u32)) -> bool {
    lang == "en" || [(960, 700), (844, 390)].contains(&size)
}
pub(super) fn medium_shot(step: &str, lang: &str, size: (u32, u32)) -> bool {
    step == "m" && language_shot(lang, size)
}
pub(super) fn photograph(
    d: &Devctl,
    tree: &Tree,
    path: &Path,
    skip_kind: Option<&str>,
) -> Result<(Vec<String>, String)> {
    d.screenshot(path)?;
    let image = pixels::load(path)?;
    let (w, h) = d.size()?;
    let (mut faults, mut report) = pixels::contrast(tree, &image, w, h)?;
    if let Some(kind) = skip_kind {
        faults.retain(|f| !f.contains(&format!(" on {kind} ")));
        report.retain(|c| c.kind != kind);
    }
    let ink = report.iter().map(|c| c.ink).reduce(f64::min);
    let muted = report
        .iter()
        .filter(|c| c.kind != "mist")
        .map(|c| c.muted)
        .reduce(f64::min);
    Ok((faults, format!("ink_min={ink:?} muted_min={muted:?}")))
}
pub(super) fn words(tree: &Tree) -> String {
    tree.0
        .iter()
        .map(super::tree::Node::text)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
pub(super) fn gallery(d: &Devctl, out: &Path) -> Result<()> {
    prepare(out)?;
    d.shell(&json!({"gallery":true}))?;
    let result = (|| {
        let mut report = Report::default();
        for &(w, h) in check::SIZES {
            d.resize(w, h)?;
            for step in ["xs", "m", "xl"] {
                for lang in ["en", "de"] {
                    for touch in [false, true] {
                        d.knobs(step, lang, touch)?;
                        if d.health()?["paused"] == true {
                            d.post("/step", &json!({"frames":4}))?;
                        }
                        Devctl::until(20.0, || {
                            Ok(d.state()?["shell_nodes"]
                                .as_array()
                                .is_some_and(|a| a.iter().any(|n| n["r"] == "gallery")))
                        })?;
                        sleep(0.6);
                        let state = d.state()?;
                        let tree = Tree::read(&state, Some("gallery"))?;
                        let mut faults =
                            check::layout(&tree, f64::from(w), lang == "de", touch, true);
                        let tag = format!(
                            "{w}x{h}-{step}-{lang}-{}",
                            if touch { "touch" } else { "pointer" }
                        );
                        let mut extra = String::new();
                        if step == "m" && !touch {
                            let (more, contrast) = photograph(
                                d,
                                &tree,
                                &out.join(format!("gallery-{tag}.png")),
                                None,
                            )?;
                            faults.extend(more);
                            extra = contrast;
                        }
                        write_json(
                            &out.join(format!("gallery-{tag}.json")),
                            &state["shell_nodes"],
                        )?;
                        report.case(&tag, tree.0.len(), &faults, &extra);
                    }
                }
            }
        }
        report.finish(Some(&out.join("gallery-check.log")), false)
    })();
    cleanup(
        result,
        d.shell(&json!({"text_size":"m","lang":"en","input":"auto","gallery":false})),
    )
}
pub(super) fn value_array(value: &Value) -> Result<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| anyhow::anyhow!("expected state array"))
}
