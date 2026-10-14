//! Front door faces and the always-present legal colophon.
use super::{
    check,
    common::{self, Report},
    devctl::{Devctl, sleep},
    tree::Tree,
};
use anyhow::{Result, bail};
use std::path::Path;
const FACES: &[&str] = &["gateway", "signin", "create", "guest", "about"];
fn choose(d: &Devctl, gateway: &str) -> Result<()> {
    if d.has("Front(LeaveGateway)")? {
        d.press_until("Front(LeaveGateway)", || d.has("Front(AddGateway)"), 3)?;
        sleep(1.0);
    }
    let state = d.state()?;
    let tree = Tree::read(&state, None)?;
    let controls = d.controls()?;
    for n in tree.0.iter().filter(|n| n.text() == gateway) {
        if let Some(row) = controls.iter().find(|c| {
            c.press.starts_with("Front(SelectGateway") && (c.at_y - (n.y + n.h / 2.0)).abs() < 30.0
        }) {
            d.press_until(&row.press, || d.has("Front(LeaveGateway)"), 3)?;
            sleep(1.2);
            return Ok(());
        }
    }
    bail!("named gateway is not present")
}
fn face(d: &Devctl, face: &str, gateway: &str) -> Result<()> {
    for name in ["Settings(CloseSettings)", "Front(About(false))"] {
        if d.has(name)? {
            d.press_until(name, || Ok(!d.has(name)?), 3)?;
        }
    }
    if face == "gateway" {
        if d.has("Front(BackToSignIn)")? {
            d.press_until("Front(BackToSignIn)", || d.has("Front(LeaveGateway)"), 3)?;
        }
        if d.has("Front(LeaveGateway)")? {
            d.press_until("Front(LeaveGateway)", || d.has("Front(AddGateway)"), 3)?;
            sleep(1.0);
        }
        return Ok(());
    }
    if face == "about" {
        return d.press_until("Front(About(true))", || d.has("Front(About(false))"), 3);
    }
    if d.has("Front(AddGateway)")? {
        choose(d, gateway)?;
    }
    if d.has("Front(BackToSignIn)")? {
        d.press_until("Front(BackToSignIn)", || d.has("Front(GuestFace)"), 3)?;
        sleep(0.6);
    }
    match face {
        "create" => d.press_until(
            "Front(ToggleRegistering)",
            || d.has("Shared(Focus(DisplayName))"),
            3,
        )?,
        "guest" => d.press_until("Front(GuestFace)", || d.has("Shared(Focus(GuestName))"), 3)?,
        _ => {}
    }
    sleep(0.8);
    Ok(())
}
pub(super) fn run(d: &Devctl, out: &Path, gateway: &str) -> Result<()> {
    common::prepare(out)?;
    let (sizes, steps) = common::matrix()?;
    let result = (|| {
        let mut report = Report::default();
        for (w, h) in sizes {
            d.resize(w, h)?;
            sleep(1.0);
            let touch = check::TOUCH.contains(&(w, h));
            for step in &steps {
                for lang in ["en", "de"] {
                    d.knobs(step, lang, touch)?;
                    sleep(0.6);
                    for &which in FACES {
                        face(d, which, gateway)?;
                        d.settle(4.0)?;
                        let mut tree = d.nodes(None)?;
                        let mut faults =
                            check::layout(&tree, f64::from(w), lang == "de", touch, true);
                        if which != "about" {
                            faults.extend(check::colophon(&common::words(&tree)));
                        }
                        let tag = format!("{which}-{w}x{h}-{step}-{lang}");
                        let mut extra = String::new();
                        if common::medium_shot(step, lang, (w, h)) {
                            let png = out.join(format!("front-{tag}.png"));
                            if lang == "en" {
                                if which == "about" {
                                    for n in &mut tree.0 {
                                        if matches!(n.kind(), "panel" | "mist") {
                                            n.k = None;
                                        }
                                    }
                                }
                                let (more, contrast) = common::photograph(d, &tree, &png, None)?;
                                faults.extend(more);
                                extra = contrast;
                            } else {
                                d.screenshot(&png)?;
                            }
                        }
                        report.case(&tag, tree.0.len(), &faults, &extra);
                    }
                }
            }
        }
        face(d, "signin", gateway)?;
        report.finish(Some(&out.join("front-check.log")), false)
    })();
    common::cleanup(result, common::reset(d))
}
