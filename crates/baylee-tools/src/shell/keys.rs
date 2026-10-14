//! Gallery keyboard acceptance, including the photographed focus ring.
use super::{
    common::{self, Report},
    devctl::{COMMAND_KEY, Devctl, sleep},
    pixels,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{fs, path::Path};
pub(super) fn order(source: &str) -> Result<Vec<String>> {
    let (_, source) = source
        .split_once("GALLERY_ORDER: TabOrder")
        .context("missing GALLERY_ORDER")?;
    let (_, source) = source
        .split_once("stops: &[")
        .context("missing gallery stops")?;
    let (block, _) = source
        .split_once("],")
        .context("unterminated gallery stops")?;
    let re = regex::Regex::new(r#""([a-z0-9-]+)""#)?;
    let names: Vec<_> = re.captures_iter(block).map(|c| c[1].to_owned()).collect();
    ensure!(
        names.len() >= 3 && names.iter().any(|s| s == "create"),
        "invalid gallery order"
    );
    Ok(names)
}
fn focus(d: &Devctl) -> Result<Value> {
    Ok(d.shell_state()?["focus"]["id"].clone())
}
fn press(d: &Devctl, name: &str, character: Option<&str>) -> Result<()> {
    d.key(name, character, 0.2)
}
fn walk_order(d: &Devctl, order: &[String], report: &mut Report, tag: &str) -> Result<()> {
    let mut walked = Vec::new();
    for _ in order {
        press(d, "Tab", None)?;
        walked.push(focus(d)?);
    }
    report.expect(
        walked == order.iter().map(|s| json!(s)).collect::<Vec<_>>(),
        &format!("{tag} Tab walks GALLERY_ORDER ({} stops)", order.len()),
    );
    for _ in 0..3 {
        press(d, "Tab", None)?;
    }
    d.chord("ShiftLeft", "Tab", None)?;
    report.expect(
        focus(d)? == order[1],
        &format!("{tag} Shift+Tab steps back"),
    );
    report.expect(
        d.shell_state()?["focus_visible"] == true,
        &format!("{tag} ring visible after key"),
    );
    Ok(())
}
fn ring(d: &Devctl, out: &Path, order: &[String], report: &mut Report, tag: &str) -> Result<()> {
    // Unlike the old unbounded loop, a broken tab order fails after one cycle.
    for _ in 0..=order.len() {
        if focus(d)? == "create" {
            break;
        }
        press(d, "Tab", None)?;
    }
    ensure!(focus(d)? == "create", "Tab did not reach create");
    let tree = d.nodes(Some("gallery"))?;
    let create = tree
        .0
        .iter()
        .find(|n| n.kind() == "hit" && n.y > 60.0)
        .context("missing create hit wrapper")?;
    let path = out.join(format!("ring-key-{tag}.png"));
    d.screenshot(&path)?;
    let by_key = pixels::ring(&pixels::load(&path)?, create);
    let x = create.x + create.w / 2.0;
    let y = create.y + create.h / 2.0;
    d.pointer(x, y, false)?;
    d.pointer(x, y, true)?;
    sleep(0.6);
    let path = out.join(format!("ring-click-{tag}.png"));
    d.screenshot(&path)?;
    let by_click = pixels::ring(&pixels::load(&path)?, create);
    report.expect(
        by_key > 20 && by_click < 5 && d.shell_state()?["focus_visible"] == false,
        &format!("{tag} ring after Tab ({by_key} accent pixels), none after click ({by_click})"),
    );
    Ok(())
}
// Read the small TextSize contract without linking the Bevy client. Refuse
// a source shape we cannot understand instead of silently keeping old steps.
fn text_size_arm<'a>(
    implementation: &'a syn::ItemImpl,
    method: &str,
    variant: &syn::Ident,
) -> Result<&'a syn::Expr> {
    let function = implementation
        .items
        .iter()
        .find_map(|item| match item {
            syn::ImplItem::Fn(function) if function.sig.ident == method => Some(function),
            _ => None,
        })
        .with_context(|| format!("missing TextSize::{method}"))?;
    let [syn::Stmt::Expr(syn::Expr::Match(choice), None)] = function.block.stmts.as_slice() else {
        bail!("expected a match in TextSize::{method}");
    };
    choice
        .arms
        .iter()
        .find_map(|arm| match &arm.pat {
            syn::Pat::Path(path)
                if arm.guard.is_none()
                    && path
                        .path
                        .segments
                        .last()
                        .is_some_and(|part| &part.ident == variant) =>
            {
                Some(arm.body.as_ref())
            }
            _ => None,
        })
        .with_context(|| format!("missing TextSize::{method} arm for {variant}"))
}

fn text_size_steps(source: &str) -> Result<[u8; 3]> {
    let source = syn::parse_file(source)?;
    let size = source
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item) if item.ident == "TextSize" => Some(item),
            _ => None,
        })
        .context("missing TextSize enum")?;
    let initial = size
        .variants
        .iter()
        .find(|variant| variant.ident == "M")
        .context("missing TextSize::M")?;
    let reset = size
        .variants
        .iter()
        .find(|variant| {
            variant
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("default"))
        })
        .context("missing TextSize default")?;
    let implementation = source.items.iter().find_map(|item| match item {
        syn::Item::Impl(item) if item.trait_.is_none()
            && matches!(item.self_ty.as_ref(), syn::Type::Path(path) if path.path.is_ident("TextSize")) => Some(item),
        _ => None,
    }).context("missing TextSize implementation")?;
    let syn::Expr::Path(next) = text_size_arm(implementation, "larger", &initial.ident)? else {
        bail!("expected a TextSize variant from larger()");
    };
    let next = &next
        .path
        .segments
        .last()
        .context("missing larger() variant")?
        .ident;
    let step = |variant| -> Result<u8> {
        let syn::Expr::Lit(value) = text_size_arm(implementation, "step", variant)? else {
            bail!("expected a numeric TextSize::step");
        };
        let syn::Lit::Int(value) = &value.lit else {
            bail!("expected an integer TextSize::step");
        };
        Ok(value.base10_parse()?)
    };
    Ok([step(&initial.ident)?, step(next)?, step(&reset.ident)?])
}

fn overlay(
    d: &Devctl,
    out: &Path,
    report: &mut Report,
    tag: &str,
    expected_steps: [u8; 3],
) -> Result<()> {
    d.chord(COMMAND_KEY, "Slash", Some("/"))?;
    let shell = d.shell_state()?;
    report.expect(
        shell["overlay"] == true && shell["table"] == "overlay" && shell["focus"]["id"] == "search",
        &format!("{tag} Ctrl/Cmd+/ opens overlay focused on search"),
    );
    d.screenshot(&out.join(format!("overlay-{tag}.png")))?;
    let mut cycle = Vec::new();
    for _ in 0..4 {
        press(d, "Tab", None)?;
        cycle.push(focus(d)?);
    }
    report.expect(
        cycle
            == vec![
                json!("list"),
                json!("edit-keys"),
                json!("close"),
                json!("search"),
            ],
        &format!("{tag} Tab cycles inside overlay"),
    );
    let before = d.shell_state()?["fired"].clone();
    for (c, key) in [
        ("c", "KeyC"),
        ("1", "Digit1"),
        ("/", "Slash"),
        ("n", "KeyN"),
    ] {
        press(d, key, Some(c))?;
    }
    report.expect(
        d.shell_state()?["fired"] == before,
        &format!("{tag} c 1 / n in overlay search fire nothing"),
    );
    press(d, "Escape", None)?;
    press(d, "Escape", None)?;
    report.expect(
        d.shell_state()?["overlay"] == false,
        &format!("{tag} Esc clears then closes"),
    );
    let step0 = d.shell_state()?["text_size"].clone();
    d.chord(COMMAND_KEY, "Equal", Some("="))?;
    let step1 = d.shell_state()?["text_size"].clone();
    d.chord(COMMAND_KEY, "Digit0", Some("0"))?;
    let step2 = d.shell_state()?["text_size"].clone();
    report.expect(
        [step0, step1, step2] == expected_steps.map(Value::from),
        &format!(
            "{tag} Ctrl/Cmd+= then 0: steps {} -> {} -> {}",
            expected_steps[0], expected_steps[1], expected_steps[2],
        ),
    );
    Ok(())
}
pub(super) fn run(root: &Path, d: &Devctl, out: &Path) -> Result<()> {
    common::prepare(out)?;
    let order = order(&fs::read_to_string(
        root.join("crates/baylee-client/src/shellkit/gallery.rs"),
    )?)?;
    let expected_steps = text_size_steps(&fs::read_to_string(
        root.join("crates/baylee-client/src/shellkit/size.rs"),
    )?)?;
    let result = (|| {
        let mut report = Report::default();
        for (w, h) in [(1920, 1080), (844, 390)] {
            d.resize(w, h)?;
            d.unit_scale()?;
            d.shell(&json!({"gallery":true,"text_size":"m","lang":"en","input":"pointer"}))?;
            sleep(1.0);
            let tag = format!("{w}x{h}");
            walk_order(d, &order, &mut report, &tag)?;
            ring(d, out, &order, &mut report, &tag)?;
            overlay(d, out, &mut report, &tag, expected_steps)?;
        }
        report.finish(Some(&out.join("keys-check.log")), false)
    })();
    common::cleanup(
        result,
        d.shell(&json!({"gallery":false,"text_size":"m","lang":"en","input":"auto"})),
    )
}

#[cfg(test)]
mod tests {
    use super::text_size_steps;

    #[test]
    fn text_steps_follow_the_client_type_including_larger_and_reset() {
        let source = include_str!("../../../baylee-client/src/shellkit/size.rs");
        assert_eq!(text_size_steps(source).unwrap(), [3, 4, 3]);

        // Change the real source in memory: the helper must derive all three
        // values anew rather than assuming adjacent steps or reset == M.
        let changed = source
            .replace("Self::M => 3,", "Self::M => 8,")
            .replace("Self::M => Self::L,", "Self::M => Self::Xl,")
            .replace("#[default]\n    M,", "M,")
            .replace("    S,", "    #[default]\n    S,");
        assert_eq!(text_size_steps(&changed).unwrap(), [8, 5, 2]);
        assert!(text_size_steps(&source.replace("fn step(", "fn removed_step(")).is_err());
    }
}
