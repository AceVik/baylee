//! Drive rendered keyboard stops with real pointer coordinates. All game mutations
//! go through the normal client, its authenticated account and the fixture Central.
use super::{Devctl, Owned, Path, Value, fs, json, state};
use anyhow::{Context, Result, bail, ensure};
use std::time::{Duration, Instant};

const CAL: &str = "advent";
const EQUIP: &str = "deck-appearance";

fn nodes(view: &Value) -> impl Iterator<Item = &Value> {
    ["shell_nodes", "ui_nodes"]
        .into_iter()
        .flat_map(|key| view[key].as_array().into_iter().flatten())
}

fn visible(row: &Value) -> bool {
    ["hid", "center_clipped", "no_camera"]
        .iter()
        .all(|key| row[*key] != true)
        && ["w", "h"]
            .iter()
            .all(|key| row[*key].as_f64().is_some_and(|v| v > 0.5))
}

fn control<'a>(view: &'a Value, table: &str, id: &str, item: u8) -> Option<&'a Value> {
    nodes(view).find(|n| {
        visible(n)
            && n["stop"]["table"] == table
            && n["stop"]["id"] == id
            && n["stop"]["item"] == item
    })
}

struct Visit<'a> {
    dev: &'a Devctl,
    client: &'a mut Owned,
    dir: &'a Path,
}

impl Visit<'_> {
    fn wait(&mut self, name: &str, ready: impl Fn(&Value) -> bool) -> Result<Value> {
        let timeout = Instant::now() + Duration::from_secs(30);
        loop {
            let view = state(self.dev)?;
            if ready(&view) {
                return Ok(view);
            }
            if Instant::now() >= timeout || self.client.exited() {
                fs::write(self.dir.join("advent-failure-state.json"), view.to_string())?;
                bail!("native Advent check timed out waiting for {name}");
            }
            std::thread::sleep(Duration::from_millis(125));
        }
    }

    fn button(&mut self, table: &str, id: &str, item: u8) -> Result<Value> {
        let label = format!("{table}/{id}/{item}");
        let view = self.wait(&label, |v| control(v, table, id, item).is_some())?;
        Ok(control(&view, table, id, item)
            .context("control disappeared")?
            .clone())
    }

    fn press(&mut self, table: &str, id: &str, item: u8) -> Result<()> {
        let row = self.button(table, id, item)?;
        let center = |start: &str, size: &str| -> Result<f64> {
            Ok(row[start].as_f64().context("control offset")?
                + row[size].as_f64().context("control dimension")? / 2.)
        };
        let (x, y) = (center("x", "w")?, center("y", "h")?);
        let health = self.dev.get("health", Duration::from_secs(5))?;
        ensure!(
            x >= 0.
                && y >= 0.
                && x < health["width"].as_f64().unwrap_or_default()
                && y < health["height"].as_f64().unwrap_or_default(),
            "control outside the client window: {table}/{id}"
        );
        self.dev.post(
            "pointer",
            &json!({"x":x,"y":y,"button":"left","press":true}),
        )?;
        Ok(())
    }

    fn words(&mut self, expected: &str) -> Result<()> {
        self.wait(expected, |view| {
            nodes(view).any(|row| visible(row) && row["t"].as_str() == Some(expected))
        })?;
        Ok(())
    }

    fn capture(&mut self, name: &str) -> Result<()> {
        let view = state(self.dev)?;
        ensure!(
            nodes(&view)
                .filter(|n| visible(n))
                .filter_map(|n| n["t"].as_str())
                .all(|s| !s.contains("{$") && !s.contains("{ $")),
            "untranslated variable in visible text on {name}"
        );
        let path = self.dir.join(format!("advent-{name}.png"));
        for _ in 0..8 {
            let result = self.dev.post_within(
                "screenshot",
                &json!({"path":path}),
                Duration::from_secs(20),
            )?;
            if result["contrast"] == true {
                return Ok(());
            }
            ensure!(result["ok"] == true, "could not write {name}");
            // Metal may yield the previous empty drawable on the first frame
            // after raising a passive client. Keep the scene and request a fresh one.
            std::thread::sleep(Duration::from_millis(260));
        }
        bail!("no rendered pixels in eight frames of {name}");
    }

    fn keyboard_open(&mut self) -> Result<()> {
        for _ in 0..40 {
            let view = state(self.dev)?;
            let focus = &view["shell"]["focus"];
            if focus["table"] == "play" && focus["id"] == "advent" {
                self.dev.post("key", &json!({"name":"Enter"}))?;
                self.button(CAL, "door", 1)?;
                return Ok(());
            }
            self.dev.post("key", &json!({"name":"Tab"}))?;
        }
        bail!("calendar CTA unreachable through the native tab order");
    }
    fn entry(&mut self, moving: bool) -> Result<Value> {
        let view = self.wait("rendered calendar CTA material", |v| {
            control(v, "play", "advent", 0)
                .is_some_and(|row| row["advent_light"][1].as_f64() == Some(f64::from(moving)))
        })?;
        Ok(control(&view, "play", "advent", 0)
            .context("missing CTA")?
            .clone())
    }

    fn same_rect(a: &Value, b: &Value) -> Result<()> {
        for key in ["x", "y", "w", "h"] {
            ensure!(
                (a[key].as_f64().context("first CTA rect")?
                    - b[key].as_f64().context("second CTA rect")?)
                .abs()
                    < 1.,
                "seasonal action moved at {key}"
            );
        }
        Ok(())
    }
}

pub(super) fn visit(dev: &Devctl, client: &mut Owned, dir: &Path) -> Result<()> {
    let mut qa = Visit { dev, client, dir };
    let first = qa.entry(true)?;
    qa.capture("01-entry")?;
    std::thread::sleep(Duration::from_millis(260));
    let next = qa.entry(true)?;
    Visit::same_rect(&first, &next)?;
    ensure!(
        first["advent_light"][0] != next["advent_light"][0],
        "attention glow did not progress"
    );
    qa.press("play", "advent", 0)?;
    qa.button(CAL, "door", 1)?;
    let calendar = qa.wait("calendar image and 24 door numbers", |v| {
        nodes(v).any(|n| visible(n) && n["calendar_image"] == true)
            && (1..=24)
                .all(|n| nodes(v).any(|r| visible(r) && r["t"].as_str() == Some(&n.to_string())))
    })?;
    ensure!(
        (2..=24).all(|n| control(&calendar, CAL, "door", n).is_none()),
        "a locked door accepted input"
    );
    qa.capture("02-calendar")?;
    qa.press(CAL, "door", 1)?;
    qa.button(CAL, "move", 1)?;
    qa.words("Door 1")?;
    qa.words("Free hint 1")?;
    qa.capture("03-puzzle")?;
    qa.press(CAL, "hint", 1)?;
    qa.words("Check the opposing life total.")?;
    qa.press(CAL, "move", 0)?;
    qa.press(CAL, "submit", 0)?;
    qa.words("This line missed the goal. You can try again.")?;
    qa.press(CAL, "reset", 0)?;
    qa.press(CAL, "move", 1)?;
    qa.press(CAL, "submit", 0)?;
    qa.button(CAL, "claim", 0)?;
    qa.capture("04-solved")?;
    qa.press(CAL, "claim", 0)?;
    qa.button(CAL, "deck", 0)?;
    qa.words("Fixture aurora 01")?;
    qa.wait("loaded private reward image", |v| {
        nodes(v).any(|n| visible(n) && n["cosmetic_preview"] == true)
    })?;
    qa.capture("05-reward")?;
    qa.press(CAL, "deck", 0)?;
    qa.words("00 Advent QA A")?;
    qa.press(EQUIP, "item", 0)?;
    qa.words("00 Advent QA A")?;
    qa.press(EQUIP, "apply", 0)?;
    qa.words("Saved for this deck")?;
    qa.capture("06-equipped")?;
    qa.press(EQUIP, "close", 0)?;
    qa.press(CAL, "deck", 0)?;
    qa.press(EQUIP, "item", 1)?;
    qa.words("01 Advent QA B")?;
    qa.button(EQUIP, "apply", 0)?;
    qa.press(EQUIP, "close", 0)?;
    qa.press(CAL, "deck", 0)?;
    qa.press(EQUIP, "item", 0)?;
    qa.words("Equipped on this deck")?;
    qa.dev.post("key", &json!({"name":"Escape"}))?;
    qa.wait("keyboard close of deck picker", |v| {
        control(v, EQUIP, "close", 0).is_none()
    })?;
    qa.press(CAL, "gallery", 0)?;
    qa.button(CAL, "item", 0)?;
    qa.capture("07-gallery")?;
    qa.dev.post("key", &json!({"name":"Escape"}))?;
    qa.wait("keyboard close of calendar", |v| {
        control(v, CAL, "close", 0).is_none()
    })?;
    let after = qa.entry(false)?;
    Visit::same_rect(&first, &after)?;
    ensure!(
        after["advent_light"][0] == 0.0 && after["advent_light"][3] == 0.0,
        "resolved calendar still draws attention"
    );
    qa.capture("08-entry-resolved")?;
    qa.keyboard_open()?;
    qa.press(CAL, "close", 0)?;
    qa.press("play", "ranked", 0)?;
    qa.press("play", "ranked-board", 0)?;
    qa.button("ranked", "back", 0)?;
    qa.capture("09-ranked-reference")?;
    fs::write(
        dir.join("advent-motion.json"),
        serde_json::to_vec_pretty(&json!({"before":first,"next":next,"resolved":after}))?,
    )?;
    println!(
        "Advent passed through a signed account, including retry, claim and equipment for two independent decks."
    );
    Ok(())
}
