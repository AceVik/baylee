//! Native rank reward acquisition, equipment and a subsequent online duel.
use super::{CAL, EQUIP, Visit, nodes, visible};
use anyhow::{Context, Result};
use serde_json::json;

impl Visit<'_> {
    fn named_row(&mut self, table: &str, id: &str, title: &str) -> Result<()> {
        let view = self.wait(title, |v| {
            nodes(v).any(|n| visible(n) && n["t"].as_str() == Some(title))
        })?;
        let word = nodes(&view)
            .find(|n| visible(n) && n["t"].as_str() == Some(title))
            .context("missing visible row title")?;
        let midpoint = |axis: &str, size: &str| {
            word[axis]
                .as_f64()
                .zip(word[size].as_f64())
                .map(|(x, w)| x + w / 2.)
        };
        let x = midpoint("x", "w").context("row text x")?;
        let y = midpoint("y", "h").context("row text y")?;
        let row = nodes(&view)
            .find(|row| {
                visible(row)
                    && row["stop"]["table"] == table
                    && row["stop"]["id"] == id
                    && ["x", "y", "w", "h"].iter().all(|k| row[*k].is_number())
                    && (row["x"].as_f64().unwrap()
                        ..row["x"].as_f64().unwrap() + row["w"].as_f64().unwrap())
                        .contains(&x)
                    && (row["y"].as_f64().unwrap()
                        ..row["y"].as_f64().unwrap() + row["h"].as_f64().unwrap())
                        .contains(&y)
            })
            .context("title must belong to a visible focusable row")?;
        self.press(
            table,
            id,
            u8::try_from(row["stop"]["item"].as_u64().context("row index")?)?,
        )
    }

    pub(super) fn ranked_items(&mut self) -> Result<()> {
        self.words("Card sleeve · Rank reward · Gold II")?;
        self.words("Playmat · Rank reward · Gold II")?;
        self.capture("12-ranked-collection")?;
        for (title, frame) in [
            ("QA Gilded Hall", "13-ranked-playmat"),
            ("QA Sunburst", "15-ranked-sleeve"),
        ] {
            self.named_row(CAL, "item", title)?;
            self.words("Rank reward · Gold II")?;
            self.wait("ranked private collection asset", |v| {
                nodes(v).any(|n| visible(n) && n["cosmetic_preview"] == true)
            })?;
            self.capture(frame)?;
            self.press(CAL, "deck", 0)?;
            self.named_row(EQUIP, "item", "Private forest 61")?;
            self.words("Rank reward · Gold II")?;
            self.press(EQUIP, "apply", 0)?;
            self.words("Saved for this deck")?;
            if title == "QA Gilded Hall" {
                self.capture("14-ranked-deck-picker")?;
            }
            self.press(EQUIP, "close", 0)?;
            self.press(CAL, "gallery", 0)?;
        }
        Ok(())
    }

    fn lobby_press(&mut self, prefix: &str) -> Result<()> {
        let view = self.wait(prefix, |v| {
            v["lobby_controls"].as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|row| row["press"].as_str().is_some_and(|s| s.starts_with(prefix)))
            })
        })?;
        let press = view["lobby_controls"]
            .as_array()
            .context("lobby controls")?
            .iter()
            .filter_map(|r| r["press"].as_str())
            .find(|s| s.starts_with(prefix))
            .context("lobby action")?;
        super::super::click(self.dev, press)
    }

    fn game_button(&mut self, label: &str) -> Result<()> {
        let view = self.wait(label, |v| {
            v["buttons"].as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|r| r["kind"] == "menu" && r["label"] == label)
            })
        })?;
        let button = view["buttons"]
            .as_array()
            .context("game buttons")?
            .iter()
            .find(|r| r["kind"] == "menu" && r["label"] == label)
            .context("game button")?;
        self.dev.post(
            "pointer",
            &json!({"x":button["at_x"],"y":button["at_y"],
            "button":"left","press":true}),
        )?;
        Ok(())
    }

    pub(super) fn ranked_table(&mut self) -> Result<()> {
        if std::env::var("BAYLEE_CENTRAL_ADVENT_UI_DE").as_deref() == Ok("1") {
            self.press(CAL, "close", 0)?;
        } else {
            self.press("ranked", "back", 0)?;
        }
        self.press("play", "create", 0)?;
        self.lobby_press("Play(Players(2))")?;
        self.press("create-table", "open", 0)?;
        self.lobby_press("Room(RoomDeckPicker(0))")?;
        self.named_row("room", "seats", "Private forest 61")?;
        self.lobby_press("Shared(OpenMenu(SeatAi(1)))")?;
        self.lobby_press("Room(SeatAiOpen(")?;
        self.lobby_press("Room(RoomDeckPicker(1))")?;
        self.named_row("room", "seats", "01 Advent QA B")?;
        self.press("room", "start", 0)?;
        let view = self.wait(
            "the authorized Gold II playmat on the owner's live 1v1 surface",
            |v| {
                v["phase"] == "Playing"
                    && v["loading"]["destination"] == "hidden"
                    && v["buttons"]
                        .as_array()
                        .is_some_and(|buttons| buttons.iter().any(|b| b["label"] == "Keep"))
                    && v["arrangement"]["mats"].as_array().is_some_and(|mats| {
                        mats.len() == 2
                            && mats
                                .iter()
                                .any(|m| m["seat"] == 0 && m["print_bound"] == true)
                            && mats
                                .iter()
                                .any(|m| m["seat"] == 1 && m["print_bound"] != true)
                    })
            },
        )?;
        std::fs::write(
            self.dir.join("ranked-mat-binding.json"),
            serde_json::to_vec_pretty(&view["arrangement"]["mats"])?,
        )?;
        self.capture("16-ranked-mat-in-play")?;
        self.game_button("ToggleGameMenu")?;
        self.game_button("Concede")?;
        std::thread::sleep(std::time::Duration::from_millis(200));
        self.game_button("Concede")?;
        self.wait("completed synthetic 1v1", |v| v["phase"] == "Finished")?;
        Ok(())
    }
}
