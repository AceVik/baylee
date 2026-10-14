//! W2/W3/W6/W7 use only /key, /text and read-only routes, never a pointer.
use super::{
    common::Report,
    devctl::{COMMAND_KEY, Devctl, sleep},
};
use anyhow::{Result, bail};
use std::collections::BTreeSet;
fn key(d: &Devctl, name: &str, wait: f64) -> Result<()> {
    d.key(name, (name == "Slash").then_some("/"), wait)
}
fn focus(d: &Devctl, table: &str, id: &str) -> Result<bool> {
    let s = d.shell_state()?;
    Ok(s["focus"]["table"] == table && s["focus"]["id"] == id)
}
fn uses(d: &Devctl) -> Result<BTreeSet<String>> {
    Ok(d.names()?
        .into_iter()
        .filter(|p| p.starts_with("Decks(Use("))
        .collect())
}
fn where_now(d: &Devctl, label: &str) -> Result<()> {
    let s = d.shell_state()?;
    println!(
        "    {label}: screen={} table={} focus={} typing={}",
        s["screen"], s["table"], s["focus"], s["typing"]
    );
    Ok(())
}
fn w6(d: &Devctl, r: &mut Report) -> Result<()> {
    println!("W6 · Use a deck for the next game");
    key(d, "Digit2", 0.8)?;
    where_now(d, "Decks")?;
    r.expect(focus(d, "decks", "tiles")?, "Decks opens on a tile");
    key(d, "ArrowRight", 0.35)?;
    where_now(d, "→")?;
    let before = uses(d)?;
    key(d, "Enter", 0.8)?;
    where_now(d, "Enter")?;
    let after = uses(d)?;
    r.expect(
        before != after && before.len() == after.len(),
        "the next game moved",
    );
    key(d, "Digit1", 0.8)?;
    where_now(d, "Play")?;
    r.expect(
        d.shell_state()?["table"] == "play",
        "End: Play, the hero shows it",
    );
    Ok(())
}
fn w7(d: &Devctl, r: &mut Report) -> Result<()> {
    println!("W7 · Delete a deck and undo");
    key(d, "Digit2", 0.8)?;
    let before = uses(d)?.len();
    key(d, "ArrowRight", 0.35)?;
    key(
        d,
        if cfg!(target_os = "macos") {
            "Backspace"
        } else {
            "Delete"
        },
        0.8,
    )?;
    where_now(d, "Delete")?;
    let after = uses(d)?.len();
    r.expect(before > 0 && after + 1 == before, "the tile is gone");
    r.expect(d.has("Decks(Undo)")?, "the Undo toast stands");
    d.chord(COMMAND_KEY, "KeyZ", None)?;
    where_now(d, "Undo")?;
    r.expect(uses(d)?.len() == before, "restored");
    Ok(())
}
fn w3(d: &Devctl, r: &mut Report) -> Result<()> {
    println!("W3 · Join a table from the list");
    key(d, "Digit2", 0.8)?;
    key(d, "Digit1", 0.8)?;
    where_now(d, "Play")?;
    r.expect(focus(d, "play", "tables")?, "Play opens on tables");
    key(d, "Slash", 0.35)?;
    where_now(d, "/")?;
    r.expect(d.shell_state()?["typing"] == true, "search has caret");
    d.typed("thurs", 0.4)?;
    sleep(1.5);
    key(d, "ArrowDown", 0.35)?;
    where_now(d, "↓")?;
    r.expect(focus(d, "play", "tables")?, "↓ to first match");
    key(d, "Enter", 2.5)?;
    where_now(d, "Enter")?;
    let s = d.shell_state()?;
    r.expect(s["screen"] == "Room" || s["table"] == "room", "in the room");
    Ok(())
}
fn w2(d: &Devctl, r: &mut Report) -> Result<()> {
    println!("W2 · Create table → set clocks → room (from step 2)");
    key(d, "Digit1", 0.8)?;
    key(d, "KeyC", 0.8)?;
    where_now(d, "c")?;
    let s = d.shell_state()?;
    r.expect(s["table"] == "create-table", "Create-table sheet");
    r.expect(s["typing"] == true, "name has caret");
    for _ in 0..20 {
        key(d, "Backspace", 0.03)?;
    }
    d.typed("Thursday walk", 0.4)?;
    key(d, "Tab", 0.35)?;
    where_now(d, "Tab: players")?;
    key(d, "Tab", 0.35)?;
    where_now(d, "Tab: templates")?;
    key(d, "ArrowRight", 0.35)?;
    where_now(d, "→ Duel")?;
    for _ in 0..6 {
        key(d, "Tab", 0.35)?;
        if d.shell_state()?["focus"]["id"] == "clock" {
            break;
        }
    }
    where_now(d, "Tab… clock")?;
    r.expect(d.shell_state()?["focus"]["id"] == "clock", "clock");
    key(d, "ArrowRight", 0.35)?;
    where_now(d, "→ standard")?;
    key(d, "Enter", 2.5)?;
    where_now(d, "Enter")?;
    r.expect(d.shell_state()?["table"] == "room", "the room");
    Ok(())
}
pub(super) fn run(d: &Devctl, which: &[String]) -> Result<()> {
    let defaults = ["w6".into(), "w7".into(), "w3".into()];
    let which = if which.is_empty() {
        &defaults[..]
    } else {
        which
    };
    let mut r = Report::default();
    for w in which {
        match w.as_str() {
            "w2" => w2(d, &mut r)?,
            "w3" => w3(d, &mut r)?,
            "w6" => w6(d, &mut r)?,
            "w7" => w7(d, &mut r)?,
            _ => bail!("unknown walk {w}"),
        }
    }
    r.finish(None, false)
}
