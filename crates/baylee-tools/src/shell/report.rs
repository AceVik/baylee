//! W9b reports: references, view-only candidates, and phone keyboard clearance.
use super::{
    check,
    common::{self, Report, value_array},
    devctl::{COMMAND_KEY, Devctl, sleep},
    tree::Tree,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};
const KEYBOARD_BAND: f64 = 195.0;
fn state(d: &Devctl) -> Result<Value> {
    let r = d.state()?["report"].clone();
    ensure!(r.is_object(), "report state missing");
    Ok(r)
}
fn key(d: &Devctl, name: &str) -> Result<()> {
    d.key(name, None, 0.4)
}
fn typed(d: &Devctl, text: &str) -> Result<()> {
    d.typed(text, 0.6)
}
fn open(d: &Devctl) -> Result<()> {
    if state(d)?["open"] != true {
        d.key("F8", None, 0.2)?;
    }
    Devctl::until(20.0, || {
        Ok(d.state()?["shell_nodes"]
            .as_array()
            .is_some_and(|a| a.iter().any(|n| n["r"] == "report")))
    })?;
    sleep(0.6);
    Ok(())
}
fn close(d: &Devctl) -> Result<()> {
    for _ in 0..3 {
        if state(d)?["open"] == false {
            return Ok(());
        }
        key(d, "Escape")?;
    }
    ensure!(state(d)?["open"] == false, "report sheet failed to close");
    Ok(())
}
fn clear(d: &Devctl) -> Result<()> {
    d.chord(COMMAND_KEY, "A", None)?;
    key(d, "Backspace")
}
fn names(r: &Value) -> Result<Vec<String>> {
    value_array(&r["suggestions"])?
        .iter()
        .map(|s| {
            s["name"]
                .as_str()
                .map(str::to_owned)
                .context("suggestion without name")
        })
        .collect()
}
fn walk_references(d: &Devctl, r: &mut Report) -> Result<()> {
    let focus = d.shell_state()?["focus"].clone();
    r.expect(
        focus["table"] == "report" && focus["id"] == "text",
        "F8 puts keyboard on text",
    );
    typed(d, "#Lightning B")?;
    let names = names(&state(d)?)?;
    r.expect(
        names.first().is_some_and(|s| s == "Lightning Bolt"),
        "#Lightning B offers Lightning Bolt first",
    );
    key(d, "ArrowDown")?;
    r.expect(
        state(d)?["chosen"] == json!(names.len().saturating_sub(1).min(1)),
        "↓ moves in list",
    );
    key(d, "ArrowUp")?;
    r.expect(state(d)?["chosen"] == 0, "↑ moves back");
    key(d, "Enter")?;
    let s = state(d)?;
    r.expect(
        s["text"] == "[Lightning Bolt] ",
        "Enter writes bracketed name and trailing space",
    );
    r.expect(s["suggestions"] == json!([]), "suggestion list is gone");
    r.expect(
        s["refs"]["cards"][0]["text"] == "Lightning Bolt"
            && s["refs"]["cards"][0]["at"] == json!([0, 16]),
        "reference rides in refs with exact span [0,16]",
    );
    let tree = d.nodes(Some("report"))?;
    let field = tree
        .0
        .iter()
        .find(|n| n.kind() == "field")
        .context("missing report field")?;
    let mut previewed = false;
    for _ in 0..3 {
        d.pointer(field.x + 40.0, field.y + 20.0, false)?;
        for _ in 0..15 {
            if !state(d)?["preview"].is_null() {
                previewed = true;
                break;
            }
            sleep(0.2);
        }
        if previewed {
            break;
        }
    }
    r.expect(
        previewed
            && state(d)?["preview"]
                .as_str()
                .is_some_and(|s| s.ends_with("#0")),
        "hovering span previews its printing",
    );
    d.pointer(5.0, 5.0, false)?;
    Devctl::until(3.0, || Ok(state(d)?["preview"].is_null()))?;
    r.expect(true, "leaving removes preview");
    Ok(())
}
pub(super) fn walk(d: &Devctl) -> Result<()> {
    d.shell(&json!({"lang":"en","input":"pointer"}))?;
    open(d)?;
    clear(d)?;
    let result = (|| {
        let mut r = Report::default();
        walk_references(d, &mut r)?;
        typed(d, "at #Wrath")?;
        r.expect(
            !names(&state(d)?)?.is_empty(),
            "second reference opens list",
        );
        key(d, "Escape")?;
        let s = state(d)?;
        r.expect(
            s["open"] == true
                && s["suggestions"] == json!([])
                && s["text"].as_str().is_some_and(|s| s.ends_with("#Wrath")),
            "first Esc hides suggestions and keeps typing",
        );
        key(d, "Escape")?;
        r.expect(state(d)?["open"] == false, "second Esc closes sheet");
        open(d)?;
        r.expect(
            state(d)?["text"]
                .as_str()
                .is_some_and(|s| s.ends_with("at #Wrath")),
            "draft survives reopening",
        );
        r.finish(None, false)
    })();
    common::cleanup(result, close(d))
}
pub(super) fn shown_cards(view: &Value) -> Result<(BTreeSet<String>, usize)> {
    let mut shown = BTreeSet::new();
    let hand = value_array(&view["hand"])?;
    let mut count = hand.len();
    for c in hand {
        shown.insert(c["name"].as_str().context("hand card has no name")?.into());
    }
    for zone in ["battlefield", "stack", "looking_at", "library_tops"] {
        let objects = value_array(&view[zone])?;
        count += objects.len();
        for o in objects {
            if !o["card"].is_null() {
                shown.insert(
                    o["name"]
                        .as_str()
                        .context("public card has no name")?
                        .into(),
                );
            }
        }
    }
    for zone in ["graveyards", "exile", "command"] {
        for seat in value_array(&view[zone])? {
            let objects = value_array(seat)?;
            count += objects.len();
            for o in objects {
                if !o["card"].is_null() {
                    shown.insert(
                        o["name"]
                            .as_str()
                            .context("public card has no name")?
                            .into(),
                    );
                }
            }
        }
    }
    Ok((shown, count))
}
fn table_refs(d: &Devctl, r: &mut Report, offered: &BTreeSet<String>) -> Result<()> {
    if let Some(first) = offered.first() {
        typed(
            d,
            &format!("#{}", first.chars().take(3).collect::<String>()),
        )?;
        key(d, "Enter")?;
        let s = state(d)?;
        r.expect(
            s["text"].as_str().is_some_and(|s| s.starts_with('[')),
            "card taken at table",
        );
        r.expect(
            !s["refs"]["cards"][0]["zone"].is_null() && !s["refs"]["cards"][0]["object"].is_null(),
            "table reference names zone and object",
        );
        clear(d)?;
    }
    let s = state(d)?;
    let players = value_array(&s["players"])?;
    r.expect(!players.is_empty(), "@ offers other seats");
    if let Some(first) = players.first() {
        let name = first.as_str().context("player name missing")?;
        typed(d, &format!("@{}", name.chars().take(2).collect::<String>()))?;
        key(d, "Enter")?;
        r.expect(
            state(d)?["text"] == format!("[@{name}] "),
            "@ writes seat exactly as shown",
        );
        clear(d)?;
    }
    Ok(())
}
pub(super) fn table(d: &Devctl) -> Result<()> {
    d.shell(&json!({"lang":"en","input":"pointer"}))?;
    if d.state()?["view"].is_null() {
        if d.has("Front(PlayOffline)")? {
            d.press_until("Front(PlayOffline)", || d.has("Play(PlayHouse)"), 3)?;
        }
        Devctl::until(30.0, || d.has("Play(PlayHouse)"))?;
        d.press("Play(PlayHouse)")?;
        Devctl::until(90.0, || Ok(!d.state()?["view"].is_null()))?;
        sleep(3.0);
    }
    let view = d.state()?["view"].clone();
    ensure!(!view.is_null(), "no table view");
    let (shown, count) = shown_cards(&view)?;
    open(d)?;
    clear(d)?;
    let result = (|| {
        let mut r = Report::default();
        r.expect(
            state(d)?["at_table"] == true,
            "sheet knows it is over table",
        );
        let mut offered = BTreeSet::new();
        for letter in 'a'..='z' {
            typed(d, &format!("#{letter}"))?;
            offered.extend(names(&state(d)?)?);
            clear(d)?;
        }
        r.expect(
            !offered.is_empty() && offered.is_subset(&shown),
            "every offered name is face up in the seat's view",
        );
        r.expect(
            state(d)?["candidates"]
                .as_u64()
                .is_some_and(|n| n <= count as u64),
            "candidate count bounded by own/public view objects",
        );
        table_refs(d, &mut r, &offered)?;
        r.finish(None, false)
    })();
    common::cleanup(result, close(d))
}
pub(super) fn sheet_faults(tree: &Tree, touch: bool) -> Result<Vec<String>> {
    let surface = tree
        .0
        .iter()
        .find(|n| n.kind() == "opaque" && n.d == 1)
        .context("missing report surface")?;
    let mut faults: Vec<_> = tree
        .0
        .iter()
        .filter(|n| {
            matches!(n.kind(), "hit" | "button")
                && n.y < surface.bottom()
                && (n.x < surface.x - 1.0 || n.right() > surface.right() + 1.0)
        })
        .map(|n| format!("sheet: {} {} leaves sheet", n.kind(), n.rect()))
        .collect();
    if touch {
        let top = tree
            .0
            .iter()
            .find(|n| n.kind() == "field")
            .context("missing report field")?
            .y;
        let bottom = tree
            .0
            .iter()
            .filter(|n| n.kind() == "scroll" && n.y > top && n.h < 80.0)
            .map(super::tree::Node::bottom)
            .reduce(f64::max);
        if top >= KEYBOARD_BAND {
            faults.push(format!("phone: field top {top:.0} is not above y195"));
        }
        if bottom.is_none_or(|b| b >= KEYBOARD_BAND) {
            faults.push(format!(
                "phone: chip row bottom {bottom:?} is not above y195"
            ));
        }
    }
    Ok(faults)
}
pub(super) fn dump(d: &Devctl, out: &Path) -> Result<()> {
    common::prepare(out)?;
    let result = (|| {
        let mut r = Report::default();
        for (w, h, touch) in [(1708, 1032, false), (844, 390, true)] {
            d.resize(w, h)?;
            for lang in ["en", "de"] {
                d.shell(&json!({"lang":lang,"input":if touch{"touch"}else{"pointer"}}))?;
                open(d)?;
                clear(d)?;
                typed(d, "When I cast #Li")?;
                sleep(0.6);
                let state = d.state()?;
                let tree = Tree::read(&state, Some("report"))?;
                let mut faults = check::overflow(&tree);
                faults.extend(check::window(&tree, f64::from(w)));
                faults.extend(check::budget(&tree, lang == "de"));
                faults.extend(check::hit(&tree, touch));
                faults.extend(sheet_faults(&tree, touch)?);
                let tag = format!("{w}x{h}-{lang}-{}", if touch { "touch" } else { "pointer" });
                d.screenshot(&out.join(format!("report-{tag}.png")))?;
                common::write_json(
                    &out.join(format!("report-{tag}.json")),
                    &json!({"report":state["report"],"nodes":tree.0}),
                )?;
                r.case(&tag, tree.0.len(), &faults, "");
                clear(d)?;
                close(d)?;
            }
        }
        r.finish(Some(&out.join("report-check.log")), false)
    })();
    let clean = (|| {
        close(d)?;
        d.shell(&json!({"lang":"en","input":"auto"}))
    })();
    common::cleanup(result, clean)
}
