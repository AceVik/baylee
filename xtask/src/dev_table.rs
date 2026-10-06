//! `dev-table`: a whole table through a real gateway, with only sign-in
//! and the deck pick skipped.

use crate::{Path, acceptance, fs};

/// The dev account's username. Fixed, so a repeated run reuses one account
/// and one deck rather than filling the store with strangers. An older dev
/// database's `dev@baylee.local` was given this name by the migration that
/// brought usernames (#269).
pub(crate) const DEV_USERNAME: &str = "dev";

/// The dev account's password. This account exists only on a developer's own
/// gateway and owns nothing worth taking.
pub(crate) const DEV_PASSWORD: &str = "dev-password-dev-password";

/// The dev account's display name.
pub(crate) const DEV_NAME: &str = "dev";

/// POSTs JSON and returns `(status, body)`. A refusal is a body, not an
/// error: several steps here expect one (an account that already exists).
pub(crate) fn post(
    agent: &ureq::Agent,
    url: &str,
    token: Option<&str>,
    body: &serde_json::Value,
) -> anyhow::Result<(u16, String)> {
    let mut req = agent.post(url).header("content-type", "application/json");
    if let Some(token) = token {
        req = req.header("authorization", &format!("Bearer {token}"));
    }
    match req.send_json(body) {
        Ok(mut resp) => Ok((resp.status().as_u16(), resp.body_mut().read_to_string()?)),
        Err(ureq::Error::StatusCode(code)) => Ok((code, String::new())),
        Err(e) => Err(anyhow::anyhow!("{url}: {e}")),
    }
}

/// PUTs JSON and returns the status. Same shape as [`post`], and the same
/// reason for returning a refusal rather than raising it.
pub(crate) fn put(
    agent: &ureq::Agent,
    url: &str,
    token: &str,
    body: &serde_json::Value,
) -> anyhow::Result<u16> {
    let req = agent
        .put(url)
        .header("content-type", "application/json")
        .header("authorization", &format!("Bearer {token}"));
    match req.send_json(body) {
        Ok(resp) => Ok(resp.status().as_u16()),
        Err(ureq::Error::StatusCode(code)) => Ok(code),
        Err(e) => Err(anyhow::anyhow!("{url}: {e}")),
    }
}

/// GETs JSON and returns the body.
pub(crate) fn get(agent: &ureq::Agent, url: &str, token: &str) -> anyhow::Result<String> {
    let mut resp = agent
        .get(url)
        .header("authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| anyhow::anyhow!("{url}: {e}"))?;
    Ok(resp.body_mut().read_to_string()?)
}

/// Pulls a string field out of a JSON object body.
pub(crate) fn field(body: &str, name: &str) -> anyhow::Result<String> {
    let value: serde_json::Value = serde_json::from_str(body)?;
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("no `{name}` in {body}"))
}

/// The owner's play decks, which only a dev table reads: Moxfield rows with
/// their printings, and cards the pool may not have. Kept out of
/// `data/acceptance-decks.txt`, the suite `validate` holds the pool to.
pub(crate) const DEV_DECKS: &str = "data/dev-decks.txt";

/// A deck a dev table can play, from the acceptance file or the dev decks.
pub(crate) struct TableDeck {
    /// The `POST /decks` body.
    pub(crate) body: serde_json::Value,
    /// Whether it came from the acceptance file, which `baylee-seat
    /// --acceptance` also reads.
    pub(crate) acceptance: bool,
    /// Cards the pool does not have, left out of the deck.
    pub(crate) missing: Vec<String>,
    /// Cards in the deck the pool has but does not play in full.
    pub(crate) unfinished: Vec<String>,
}

impl TableDeck {
    /// Says, without failing, what of the deck the table will not play as
    /// printed.
    pub(crate) fn report(&self, name: &str) {
        if !self.missing.is_empty() {
            println!(
                "deck {name}: {} card(s) not in the pool, left out: {}",
                self.missing.len(),
                self.missing.join(", ")
            );
        }
        if !self.unfinished.is_empty() {
            println!(
                "deck {name}: {} card(s) not fully implemented: {}",
                self.unfinished.len(),
                self.unfinished.join(", ")
            );
        }
    }

    /// The deck as Baylee text (`docs/deck-format.md`), for a bridge's
    /// `--deck`.
    pub(crate) fn text(&self) -> String {
        let rows = |key: &str| -> Vec<String> {
            self.body[key]
                .as_array()
                .map(|rows| {
                    rows.iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        // The bridge reads the rows by name alone: a printing is the
        // gateway's business, so the file names each card plainly.
        let plain = |row: &str| match baylee_core::deckrow::parse(row) {
            Ok(parsed) => format!("{} {}", parsed.count, parsed.name),
            Err(_) => row.to_string(),
        };
        let mut out = format!(
            "# baylee deck export v1\n# name: {}\n",
            self.body["name"].as_str().unwrap_or("Deck")
        );
        if let Some(commander) = self.body["commander"].as_str() {
            out += &["CMD: 1 ", commander, "\n"].concat();
        }
        for row in rows("cards") {
            out += &plain(&row);
            out.push('\n');
        }
        for row in rows("sideboard") {
            out += &["SB: ", &plain(&row), "\n"].concat();
        }
        out
    }
}

/// Every deck name a dev table can play: the acceptance decks, then the
/// dev decks.
pub(crate) fn table_deck_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for file in ["data/acceptance-decks.txt", DEV_DECKS] {
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            continue;
        };
        for name in baylee_cards::decks::acceptance_names(&text) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// The deck `name`, as the `POST /decks` body wants it, from the acceptance
/// file or else the dev decks.
///
/// A row's printing is kept (the gateway reads it); a card the pool does not
/// have is left out and named in [`TableDeck::missing`], so a dev deck still
/// sits down with what it can play.
pub(crate) fn table_deck(root: &Path, name: &str) -> anyhow::Result<TableDeck> {
    let mut found = None;
    for (file, acceptance) in [("data/acceptance-decks.txt", true), (DEV_DECKS, false)] {
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            continue;
        };
        let rows = acceptance::parse_decks(&text).map_err(|e| anyhow::anyhow!("{file}: {e}"))?;
        let rows: Vec<_> = rows.into_iter().filter(|r| r.deck == name).collect();
        if !rows.is_empty() {
            found = Some((rows, acceptance));
            break;
        }
    }
    let Some((rows, acceptance)) = found else {
        let known = table_deck_names(root).join(", ");
        anyhow::bail!("no deck called `{name}` (there are: {known})");
    };
    let mut main = Vec::new();
    let mut side = Vec::new();
    // `POST /decks` takes one commander by name, and takes it *beside* the
    // rows — `LoadedDeck` seats a leader that has no row of its own.
    let mut commander = None;
    let mut missing = Vec::new();
    let mut unfinished = Vec::new();
    for row in &rows {
        let line = format!("{} {}", row.count, row.name);
        let parsed = baylee_core::deckrow::parse(&line)
            .map_err(|e| anyhow::anyhow!("deck {name}: `{line}`: {e:?}"))?;
        let Some(index) = baylee_cards::decks::by_name(&parsed.name) else {
            missing.push(parsed.name);
            continue;
        };
        if baylee_cards::by_index(index)
            .is_some_and(|def| !matches!(def.coverage, baylee_cards::dsl::Coverage::Implemented))
        {
            unfinished.push(parsed.name.clone());
        }
        match row.zone {
            acceptance::Zone::Main => main.push(line),
            acceptance::Zone::Sideboard => side.push(line),
            acceptance::Zone::Commander => commander = Some(parsed.name),
        }
    }
    anyhow::ensure!(!main.is_empty(), "deck {name} has no card the pool plays");
    Ok(TableDeck {
        body: serde_json::json!({
            "name": name,
            "cards": main,
            "sideboard": side,
            "commander": commander,
        }),
        acceptance,
        missing,
        unfinished,
    })
}

/// Arranges a room's chairs and starts it.
///
/// Split out of [`dev_table`] because it is the half that talks to the lobby
/// as a *host*: the AI chairs, the sides and the two statements a start takes.
///
/// A bridge's chair (`bridge_chair`) is left open for the bridge to take,
/// and `before_start` runs between arranging the table and saying ready: a
/// room whose chairs are not all ready cannot start.
#[expect(
    clippy::too_many_arguments,
    reason = "the room's arrangement, spelt out at its one call site and its test"
)]
pub(crate) fn arrange_room(
    agent: &ureq::Agent,
    gateway: &str,
    token: &str,
    game_id: &str,
    seats: usize,
    ai: &str,
    teams: &[u8],
    bridge_chairs: &[usize],
    before_start: impl FnOnce() -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // Use the room path for duels too: the one-tap "ai" mode starts
    // immediately with the default profile, before --ai can be applied.
    for seat in (1..seats).filter(|seat| !bridge_chairs.contains(seat)) {
        let url = format!("{gateway}/lobby/games/{game_id}/seats/{seat}");
        let (status, body) = post(
            agent,
            &url,
            Some(token),
            &serde_json::json!({ "kind": "ai", "ai": ai }),
        )?;
        anyhow::ensure!(
            status == 200,
            "seat the AI in chair {seat}: {status} {body}"
        );
    }

    // Sides, once every chair is arranged: the host says who plays with whom,
    // a side being the format rather than a preference.
    for (seat, team) in teams.iter().enumerate() {
        let url = format!("{gateway}/lobby/games/{game_id}/seats/{seat}");
        let (status, body) = post(
            agent,
            &url,
            Some(token),
            &serde_json::json!({ "team": team }),
        )?;
        anyhow::ensure!(
            status == 200,
            "put chair {seat} on team {team}: {status} {body}"
        );
    }

    before_start()?;

    // A room does not start itself — that takes two statements by two people
    // (`ready` is the player's, `start` is the host's), and here the dev
    // account is both. An AI chair is ready as soon as it is configured.
    let (status, body) = post(
        agent,
        &format!("{gateway}/lobby/games/{game_id}/ready"),
        Some(token),
        &serde_json::json!({ "ready": true }),
    )?;
    anyhow::ensure!(status == 200, "say ready: {status} {body}");
    let (status, body) = post(
        agent,
        &format!("{gateway}/lobby/games/{game_id}/start"),
        Some(token),
        &serde_json::json!({}),
    )?;
    anyhow::ensure!(status == 200, "start the table: {status} {body}");
    Ok(())
}

/// Seats the dev account at a table and prints or plays its ticket.
///
/// Every step is a real request to a real gateway: the only thing skipped is
/// having to type them into the lobby.
/// What a dev table is to be.
pub(crate) struct TableSpec<'a> {
    /// How many chairs.
    pub(crate) seats: usize,
    /// The AI chairs' level.
    pub(crate) ai: &'a str,
    /// The dev account's acceptance deck.
    pub(crate) deck: &'a str,
    /// Sides, in seat order; empty for none.
    pub(crate) teams: &'a [u8],
    /// The minds the bridges play.
    pub(crate) bridges: &'a [String],
}

impl TableSpec<'_> {
    /// Whether the table can be set at all.
    pub(crate) fn check(&self) -> anyhow::Result<()> {
        let (seats, teams) = (self.seats, self.teams);
        anyhow::ensure!(
            (2..=8).contains(&seats),
            "a table seats between two and eight"
        );
        anyhow::ensure!(
            teams.is_empty() || seats > 2,
            "--teams needs three chairs or more; a duel is already two sides"
        );
        anyhow::ensure!(
            teams.is_empty() || teams.len() == seats,
            "--teams needs one side per chair ({seats} of them)"
        );
        anyhow::ensure!(
            teams.is_empty() || teams.iter().any(|t| *t != teams[0]),
            "a table needs two sides; every chair is on team {}",
            teams.first().copied().unwrap_or(0)
        );
        Ok(())
    }
}

pub(crate) fn dev_table(
    root: &Path,
    gateway: &str,
    spec: &TableSpec<'_>,
    play: bool,
) -> anyhow::Result<()> {
    spec.check()?;
    let TableSpec {
        seats,
        ai,
        deck: deck_name,
        teams,
        bridges,
    } = *spec;
    let agent = ureq::Agent::new_with_defaults();

    // An account. A second run finds it already there, which is not an error.
    let _ = post(
        &agent,
        &format!("{gateway}/auth/register"),
        None,
        &serde_json::json!({
            "username": DEV_USERNAME, "display_name": DEV_NAME, "password": DEV_PASSWORD,
        }),
    )?;
    let (status, body) = post(
        &agent,
        &format!("{gateway}/auth/login"),
        None,
        &serde_json::json!({ "username": DEV_USERNAME, "password": DEV_PASSWORD }),
    )?;
    anyhow::ensure!(status == 200, "sign in as {DEV_NAME}: {status} {body}");
    let token = field(&body, "token")?;

    // A deck. The account survives between runs, so one is usually already
    // stored — and it is *rewritten* rather than reused, because
    // the deck file (`data/acceptance-decks.txt` or `data/dev-decks.txt`) is
    // what a dev table is supposed to be
    // playing. A deck saved by an older build simply stayed as it was, which
    // is how seat 0 kept sitting down at a commander table with no commander
    // for a while after the file had one.
    let deck = table_deck(root, deck_name)?;
    deck.report(deck_name);
    let body = deck.body;
    let decks: serde_json::Value =
        serde_json::from_str(&get(&agent, &format!("{gateway}/decks"), &token)?)?;
    let existing = decks.as_array().and_then(|list| {
        list.iter()
            .find(|d| d.get("name").and_then(serde_json::Value::as_str) == Some(deck_name))
            .and_then(|d| d.get("id").and_then(serde_json::Value::as_str))
            .map(ToString::to_string)
    });
    let deck_id = if let Some(id) = existing {
        let status = put(&agent, &format!("{gateway}/decks/{id}"), &token, &body)?;
        anyhow::ensure!(
            status == 204,
            "refresh the {deck_name} deck from the file: {status}"
        );
        id
    } else {
        let (status, body) = post(&agent, &format!("{gateway}/decks"), Some(&token), &body)?;
        anyhow::ensure!(status == 200, "save the {deck_name} deck: {status} {body}");
        field(&body, "deck_id")?
    };

    // Configure every table before starting, including a two-seat one.
    let create = serde_json::json!({ "deck_id": deck_id, "seats": seats, "name": "dev table" });
    let (status, body) = post(
        &agent,
        &format!("{gateway}/lobby/games"),
        Some(&token),
        &create,
    )?;
    anyhow::ensure!(status == 200, "open a table: {status} {body}");
    let game_id = field(&body, "game_id")?;
    let seat_token = field(&body, "seat_token")?;

    let mut bridge_processes = Vec::new();
    let bridge_chairs: Vec<usize> = (1..=bridges.len()).collect();
    arrange_room(
        &agent,
        gateway,
        &token,
        &game_id,
        seats,
        ai,
        teams,
        &bridge_chairs,
        || {
            for (i, mind) in bridges.iter().enumerate() {
                let child = seat_bridge(root, gateway, &game_id, mind, deck_name, i)?;
                bridge_processes.push(child);
            }
            if bridge_processes.is_empty() {
                return Ok(());
            }
            wait_for_bridges(
                &agent,
                gateway,
                &token,
                &game_id,
                &bridge_chairs,
                &mut bridge_processes,
            )
        },
    )?;

    let opponents = seats - 1 - bridges.len();
    println!("table ready: {seats} chairs, {opponents} × {ai} AI, playing {deck_name}");
    for (i, mind) in bridges.iter().enumerate() {
        println!(
            "chair {}: a bridge playing {mind}; transcripts in {BRIDGE_TRANSCRIPTS}/",
            i + 1
        );
    }
    if !teams.is_empty() {
        let sides: Vec<String> = teams.iter().map(ToString::to_string).collect();
        println!("sides, in seat order: {}", sides.join(", "));
    }
    seat_the_player(root, gateway, &game_id, &seat_token, play, bridge_processes)
}

/// Prints the dev seat's ticket, or launches the client on it; a bridge at
/// the table plays on in this terminal meanwhile.
pub(crate) fn seat_the_player(
    root: &Path,
    gateway: &str,
    game_id: &str,
    seat_token: &str,
    play: bool,
    mut bridge_processes: Vec<std::process::Child>,
) -> anyhow::Result<()> {
    if !play {
        println!(
            "\nBAYLEE_GATEWAY={gateway} \\\n  BAYLEE_GAME={game_id} \\\n  BAYLEE_SEAT_TOKEN={seat_token} \\\n  cargo run -p baylee-client"
        );
        // The bridge plays on in this terminal until the game is over.
        for child in &mut bridge_processes {
            let status = child.wait()?;
            anyhow::ensure!(status.success(), "a bridge exited with {status}");
        }
        return Ok(());
    }
    let status = std::process::Command::new("cargo")
        .args(["run", "-p", "baylee-client"])
        .current_dir(root)
        .env("BAYLEE_GATEWAY", gateway)
        .env("BAYLEE_GAME", game_id)
        .env("BAYLEE_SEAT_TOKEN", seat_token)
        .status();
    // A client that closed leaves nobody for the bridge to play but the
    // house standing in: it goes too.
    for child in &mut bridge_processes {
        let _ = child.kill();
        let _ = child.wait();
    }
    let status = status?;
    anyhow::ensure!(status.success(), "the client exited with {status}");
    Ok(())
}

/// Starts `baylee-seat join` on the room, playing `mind` with a deck the dev
/// account did not bring: an acceptance deck by name, or a dev deck written
/// out as a file the bridge reads (`--deck`).
pub(crate) fn seat_bridge(
    root: &Path,
    gateway: &str,
    game_id: &str,
    mind: &str,
    dev_deck: &str,
    index: usize,
) -> anyhow::Result<std::process::Child> {
    let decks = ["Schwarzrand", "Euro-Highlander", "Allytifact", "Weltenbaum"];
    let available: Vec<&str> = decks.into_iter().filter(|&d| d != dev_deck).collect();
    // The bridges take the decks the dev account did not bring, one after
    // another.
    let theirs = if available.is_empty() {
        decks[index % decks.len()]
    } else {
        available[index % available.len()]
    };
    let deck = table_deck(root, theirs)?;
    deck.report(theirs);
    let mut command = std::process::Command::new("cargo");
    command
        .args(["run", "-q", "-p", "baylee-seat", "--", "join", game_id])
        .args(bridge_mind(mind))
        .args(["--gateway", gateway]);
    if deck.acceptance {
        command.args(["--acceptance", theirs]);
    } else {
        let dir = root.join("target/dev-decks");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{theirs}.txt"));
        fs::write(&path, deck.text())?;
        command.arg("--deck").arg(path);
    }
    Ok(command
        .args(["--think-secs", "300"])
        .arg("--transcripts")
        .arg(root.join(BRIDGE_TRANSCRIPTS))
        .current_dir(root)
        .spawn()?)
}

/// What tells a bridge its mind: `--profile <name>` for `profile:<name>`,
/// `--mind <mind>` for anything else (the bridge refuses what it does not
/// know, in its own words).
pub(crate) fn bridge_mind(mind: &str) -> [&str; 2] {
    match mind.strip_prefix("profile:") {
        Some(profile) => ["--profile", profile],
        None => ["--mind", mind],
    }
}

/// Where a dev table's bridge writes down what it was asked and answered
/// (and a language model's mind, what it was told and said), under the
/// repository.
pub(crate) const BRIDGE_TRANSCRIPTS: &str = "target/seat-transcripts";

/// How long a bridge may take to sit down: long enough for `cargo run` to
/// build it on a cold target.
pub(crate) const BRIDGE_PATIENCE: std::time::Duration = std::time::Duration::from_mins(15);

/// Waits until every bridge has taken a chair and said ready, as the room's
/// listing shows it to the host. The bridges race for the open chairs, so
/// which one sits where is theirs to settle; what is waited for is that all
/// of `chairs` are taken and ready.
pub(crate) fn wait_for_bridges(
    agent: &ureq::Agent,
    gateway: &str,
    token: &str,
    game_id: &str,
    chairs: &[usize],
    children: &mut [std::process::Child],
) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + BRIDGE_PATIENCE;
    loop {
        for child in children.iter_mut() {
            if let Some(status) = child.try_wait()? {
                anyhow::bail!("a bridge exited with {status} before it sat down");
            }
        }
        let body = get(agent, &format!("{gateway}/lobby/games?q={game_id}"), token)?;
        let listing: serde_json::Value = serde_json::from_str(&body)?;
        let seated = listing["games"].as_array().is_some_and(|games| {
            games.iter().filter(|g| g["id"] == game_id).any(|g| {
                g["seats"].as_array().is_some_and(|seats| {
                    chairs.iter().all(|chair| {
                        seats.iter().any(|s| {
                            s["seat"] == *chair && s["taken"] == true && s["ready"] == true
                        })
                    })
                })
            })
        });
        if seated {
            return Ok(());
        }
        anyhow::ensure!(
            std::time::Instant::now() < deadline,
            "the bridges did not sit down within {BRIDGE_PATIENCE:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
