//! What a client's profile is made from: the scenes a person spends time in,
//! on this machine's window and GPU, through the harness (`dev-control`).
//!
//! The weight lies where it stutters: the deck builder on a list of five hundred
//! cards, long friend and chat lists, a huge stack, and whole games. Lobby,
//! settings, a full board and a replay go with them, so that the profile covers
//! the client and not a corner of it.
//!
//! Every scene is fixed: a seed for the game, generated data for the lists. A
//! step that cannot be taken fails the scene, and a failed scene stops the
//! build: a profile of half the client is not what the training promises. A
//! scene ends by the client closing itself, which is what writes its counts.

use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, bail};
use baylee_client_e2e::prelude::*;
use serde_json::{Value, json};

/// The seed of every game, so that a scene plays the same game again.
pub(super) const SEED: &str = "424242";
/// Seconds without a request after which a client closes itself. Longer than
/// any pause in a scene, or the scene loses its client.
const IDLE_EXIT_SECS: &str = "12";
/// How long a closing client may take.
const CLOSING: Duration = Duration::from_secs(60);
/// A whole game takes two to five minutes; an instrumented client is slower.
pub(super) const LONGEST_GAME: Duration = Duration::from_secs(600);
/// A game that shows nothing new for this long has a question the scene cannot
/// answer.
const STUCK: Duration = Duration::from_secs(45);

type Scene = fn(&Client, &Path) -> Result<()>;

const SCENES: [(&str, Scene); 5] = [
    ("the lobby and the settings", lobby_and_settings),
    ("the deck builder", deck_builder),
    ("long lists", long_lists),
    ("a whole game, a huge stack and a replay", whole_game),
    ("a full board", full_board),
];

/// Plays the scene at `only`, or every scene, each in a client of its own, into
/// `raw`.
pub fn scenes(
    root: &Path,
    client: &Path,
    raw: &Path,
    work: &Path,
    only: Option<usize>,
) -> Result<()> {
    for (at, (name, scene)) in SCENES.iter().enumerate() {
        if only.is_some_and(|only| only != at) {
            continue;
        }
        println!("training on {name}");
        let dir = work.join(at.to_string());
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("config/baylee"))?;
        if *name == "the deck builder" {
            fs::write(
                dir.join("config/baylee/offline-decks.json"),
                big_list(root)?,
            )?;
        }
        let mut launch = Launch::new()
            .tours(false)
            .env(
                "LLVM_PROFILE_FILE",
                &raw.join("%m_%p.profraw").to_string_lossy(),
            )
            .env("BAYLEE_DEV_IDLE_EXIT", IDLE_EXIT_SECS)
            .env("BAYLEE_DEV_SEED", SEED);
        if *name == "a full board" {
            launch = launch.env("BAYLEE_DEV_SEAT_BOARD", &board(root)?);
        }
        let running = Client::start(client, &launch, &dir)?;
        scene(&running, root).with_context(|| format!("{name}: {}", last_state(&running)))?;
        running.finish(CLOSING)?;
    }
    Ok(())
}

/// What the client stood at, for a scene's failure.
fn last_state(client: &Client) -> String {
    client.screen().map_or_else(
        |why| format!("no state: {why}"),
        |screen| {
            let names: Vec<&str> = screen
                .controls()
                .iter()
                .take(30)
                .map(|control| control.press.as_str())
                .collect();
            format!(
                "phase {}, question {}, controls {names:?}",
                screen.at("/phase"),
                question(&screen)
            )
        },
    )
}

fn linger(seconds: f32) {
    sleep(Duration::from_secs_f32(seconds));
}

/// Moves over the window's middle, which is what a person's hand does.
fn stir(client: &Client) -> Result<()> {
    let window = client.screen()?.window();
    let (x, y) = window.centre();
    for step in 0_u8..12 {
        client.hover(at(x + f32::from(step) * 9.0, y - f32::from(step) * 5.0))?;
        linger(0.1);
    }
    Ok(())
}

fn lobby_and_settings(client: &Client, _root: &Path) -> Result<()> {
    stir(client)?;
    client.play_offline()?;
    stir(client)?;
    client.click(press("Header(Nav(1))"))?;
    client.wait_until("the decks", |s| s.control("Decks(NewDeck)").is_some())?;
    for view in ["List", "Compact", "Gallery"] {
        client.click(press(format!("Decks(SetView({view}))")))?;
        linger(1.0);
    }
    client.click(press("Decks(Tab(House))"))?;
    linger(1.5);
    client.click(press("Decks(Tab(Mine))"))?;
    linger(1.0);
    client.click(press("Header(Nav(2))"))?;
    linger(2.0);
    client.click(press("Header(Nav(0))"))?;
    linger(1.5);
    client.click(press("Front(FrontMenu)"))?;
    linger(0.8);
    client.click(press("Settings(OpenSettings)"))?;
    client.wait_until("the settings", |s| {
        s.control("Settings(CloseSettings)").is_some()
    })?;
    let (x, y) = client.screen()?.window().centre();
    for _ in 0..12 {
        client.scroll(at(x, y), 3.0)?;
        linger(0.2);
    }
    client.click(press("Settings(CloseSettings)"))?;
    linger(1.0);
    Ok(())
}

/// A list of four hundred cards with a sideboard of a hundred, taken at even
/// steps through the pool.
fn big_list(root: &Path) -> Result<String> {
    let pool = fs::read_to_string(root.join("data/card-pool.txt")).context("the card pool")?;
    let names: Vec<&str> = pool
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();
    let spread = |count: usize, shift: usize| -> Vec<String> {
        (0..count)
            .map(|n| {
                format!(
                    "1 {}",
                    names[(n * names.len() / count + shift) % names.len()]
                )
            })
            .collect()
    };
    let deck = json!({"decks": [{
        "id": "01a1b26d-05a2-7725-981c-481ce41a0ffe",
        "account_id": "offline",
        "name": "Big list",
        "cards": spread(400, 0),
        "sideboard": spread(100, 7),
        "commanders": [],
        "format": "commander",
        "sleeve": null,
        "playmat": null,
        "updated_at": 1_793_995_191,
    }]});
    Ok(deck.to_string())
}

fn deck_builder(client: &Client, _root: &Path) -> Result<()> {
    client.play_offline()?;
    client.click(press("Header(Nav(1))"))?;
    client.wait_until("the decks", |s| s.control("Decks(NewDeck)").is_some())?;
    let list = client
        .screen()?
        .controls()
        .iter()
        .find(|control| control.press.starts_with("Decks(Edit("))
        .map(|control| control.press.clone())
        .context("no list to edit")?;
    client.click(press(list))?;
    client.wait_until("the builder", |s| {
        s.control("Build(CloseBuilder)").is_some()
    })?;
    linger(3.0);
    for round in 0..3 {
        client.click(press("Build(FocusBuild(Search))"))?;
        for word in ["sol", "drag", "t:elf"] {
            client.type_text(word)?;
            linger(0.5);
            for _ in 0..word.len() {
                client.press("Backspace")?;
                linger(0.1);
            }
        }
        client.press("Escape")?;
        for colour in ["W", "U", "B", "R", "G"] {
            let toggle = format!("Build(ToggleColor('{colour}'))");
            client.click(press(toggle.as_str()))?;
            linger(0.4);
            client.click(press(toggle.as_str()))?;
            linger(0.2);
        }
        for tab in ["Side", "Stats", "Main"] {
            client.click(press(format!("Build(SetTab({tab}))")))?;
            linger(0.6);
        }
        client.click(press("Build(CycleSort)"))?;
        linger(0.5);
        for view in ["Grid", "Compact", "List"] {
            client.click(press(format!("Build(SetPoolView({view}))")))?;
            linger(0.6);
        }
        let window = client.screen()?.window();
        for side in [0.25, 0.75] {
            for step in 0_u8..30 {
                let y = window.h * (0.3 + 0.015 * f32::from(step));
                client.hover(at(window.w * side, y))?;
                client.scroll(at(window.w * side, y), if round == 1 { -1.0 } else { 1.0 })?;
            }
            linger(0.5);
        }
        add_and_take_away(client)?;
    }
    client.hover(at(1.0, 1.0))?;
    linger(1.0);
    Ok(())
}

fn add_and_take_away(client: &Client) -> Result<()> {
    let adds: Vec<String> = client
        .screen()?
        .controls()
        .iter()
        .filter(|control| control.press.starts_with("Build(AddFromPool("))
        .take(4)
        .map(|control| control.press.clone())
        .collect();
    for add in adds {
        client.click(press(add))?;
        linger(0.5);
    }
    for edit in [
        "Build(AddRow(3))",
        "Build(RemoveRow(3))",
        "Build(RemoveRow(4))",
    ] {
        client.click(press(edit))?;
        linger(0.5);
    }
    Ok(())
}

fn long_lists(client: &Client, _root: &Path) -> Result<()> {
    client.play_offline()?;
    seed(client, &json!({"friends": 500, "lines": 450}))?;
    client.click(press("Header(Friends)"))?;
    linger(1.5);
    let window = client.screen()?.window();
    let (x, y) = (window.w - 170.0, window.h * 0.6);
    for _ in 0..2 {
        for direction in [1.0, -1.0] {
            for _ in 0..80 {
                client.scroll(at(x, y), direction * 2.0)?;
            }
            linger(0.5);
        }
    }
    client.click(at(x, 133.0))?;
    linger(0.3);
    for word in ["Friend 1", "Friend 42"] {
        client.type_text(word)?;
        linger(0.5);
        for _ in 0..word.len() {
            client.press("Backspace")?;
            linger(0.1);
        }
    }
    client.press("Escape")?;
    client.click(press("Header(Friends)"))?;
    linger(1.0);
    Ok(())
}

fn seed(client: &Client, what: &Value) -> Result<()> {
    let answer = client.devctl().post("seed", what)?;
    if answer.get("ok") != Some(&Value::Bool(true)) {
        bail!("the seed was refused: {answer}");
    }
    Ok(())
}

/// The kind of question on the table: the first key of `interaction.pending`.
fn question(screen: &Screen) -> String {
    let pending = screen.at("/interaction/pending");
    pending
        .as_object()
        .and_then(|fields| fields.keys().next().cloned())
        .or_else(|| pending.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Plays until the game is over: lands and spells where it can, every attacker
/// and a block, and an answer to whatever is asked. The moves need not be good;
/// they have to go through the code a player's moves go through.
pub(super) fn play(
    client: &Client,
    limit: Duration,
    mut along: impl FnMut(&Screen) -> Result<()>,
) -> Result<()> {
    let started = Instant::now();
    let mut tried: Vec<u64> = Vec::new();
    let mut spells = 0;
    let mut turn = 0;
    let mut asked = BTreeMap::new();
    let mut moved = (Value::Null, Instant::now());
    while started.elapsed() < limit {
        let screen = client.screen()?;
        if screen.at("/view/seq") != &moved.0 {
            moved = (screen.at("/view/seq").clone(), Instant::now());
        } else if moved.1.elapsed() > STUCK {
            bail!(
                "the game stands still at turn {turn}: {asked:?}, {}, browser {}, armed {}, \
                 mana run {}, outbox {}, error {}, arrangement {}, buttons {}",
                screen.at("/interaction/pending"),
                screen.at("/browser"),
                screen.at("/armed"),
                screen.at("/mana_run"),
                screen.at("/outbox"),
                screen.at("/last_error"),
                screen.at("/arrangement"),
                screen.at("/buttons")
            );
        }
        if screen.at("/phase") != "Playing" {
            println!("  played to turn {turn}, {spells} spells tried: {asked:?}");
            return Ok(());
        }
        *asked.entry(question(&screen)).or_insert(0_u32) += 1;
        if screen.at("/view/turn").as_u64() != Some(turn) {
            turn = screen.at("/view/turn").as_u64().unwrap_or(0);
            spells += tried.len();
            tried.clear();
        }
        along(&screen)?;
        let tries = asked.get(&question(&screen)).copied().unwrap_or(0);
        if moved.1.elapsed() > Duration::from_secs(8) {
            client.press("Escape")?;
        }
        answer(client, &screen, &mut tried, tries)?;
        linger(0.2);
    }
    println!("  stopped at turn {turn}: {asked:?}");
    Ok(())
}

fn answer(client: &Client, screen: &Screen, tried: &mut Vec<u64>, tries: u32) -> Result<()> {
    let mine = screen.at("/view/active") == &screen.at("/view/seat").clone();
    match question(screen).as_str() {
        "" => {}
        "Mulligan" => client.press("K")?,
        // A chooser the client opens itself, before the engine asks anything.
        "Priority" if dialog_row(screen, 0).is_some() => pick_something(client, screen, tries)?,
        "Priority" => {
            let legal = screen.at("/interaction/pending/Priority/legal");
            let main = screen.at("/view/step") == "Main" && mine;
            let empty = screen
                .at("/view/stack")
                .as_array()
                .is_some_and(Vec::is_empty);
            let land = legal["lands"][0].as_u64();
            let hand: Vec<_> = screen.cards_in("hand").collect();
            let spell = hand.iter().find(|card| !tried.contains(&card.object));
            match (main && empty, land, spell) {
                (true, Some(land), _) => {
                    let card = hand.iter().find(|card| card.object == land);
                    if let Some(card) = card {
                        let (x, y) = card.rect.centre();
                        client.click(at(x, y))?;
                    } else {
                        client.press("Space")?;
                    }
                }
                (true, None, Some(card)) if screen.at("/reachable").as_u64().unwrap_or(0) > 0 => {
                    tried.push(card.object);
                    let (x, y) = card.rect.centre();
                    client.click(at(x, y))?;
                }
                _ => client.press("Space")?,
            }
        }
        "ChooseAttackers" => {
            for card in own_side(screen).take(4) {
                let (x, y) = card.rect.centre();
                client.click(at(x, y))?;
            }
            client.press("Space")?;
            linger(0.2);
            if question(&client.screen()?) == "ChooseAttackers" {
                client.press("O")?;
            }
        }
        "ChooseBlockers" => {
            let attacker = screen
                .cards_in("table")
                .find(|card| card.rect.y < screen.window().h * 0.5);
            if let (Some(blocker), Some(attacker)) = (own_side(screen).next(), attacker) {
                let (x, y) = blocker.rect.centre();
                client.click(at(x, y))?;
                let (x, y) = attacker.rect.centre();
                client.click(at(x, y))?;
                client.press("Space")?;
            }
            linger(0.2);
            if question(&client.screen()?) == "ChooseBlockers" {
                client.press("O")?;
            }
        }
        "YesNo" => client.press("N")?,
        "DiscardChoice" => discard(client, screen)?,
        _ => pick_something(client, screen, tries)?,
    }
    Ok(())
}

/// Chooses as many cards as the question asks for, and confirms. It counts what
/// stands chosen after each click: a click on a chosen card takes it back, so
/// the first card again and again never reaches two.
fn discard(client: &Client, screen: &Screen) -> Result<()> {
    let asked = screen
        .at("/interaction/pending/DiscardChoice/count")
        .as_u64()
        .unwrap_or(1);
    for card in screen.cards_in("hand") {
        let chosen = client.screen()?.at("/interaction/selected").as_u64();
        if chosen.unwrap_or(0) >= asked {
            break;
        }
        let (x, y) = card.rect.centre();
        client.click(at(x, y))?;
        linger(0.2);
    }
    client.press("Space")
}

/// The row of a dialog to press on this try. The same row twice running, since a
/// row that casts arms first and sends on the second press; then the next, since
/// a row may offer what cannot be paid.
fn dialog_row(screen: &Screen, tries: u32) -> Option<(f32, f32)> {
    let buttons = screen.at("/buttons").as_array()?;
    let rows: Vec<&Value> = buttons
        .iter()
        .filter(|button| button["kind"] == "choice")
        .collect();
    let row = rows.get(usize::try_from(tries / 2).ok()? % rows.len().max(1))?;
    Some((row["at_x"].as_f64()? as f32, row["at_y"].as_f64()? as f32))
}

/// Answers a question this scene has no rule for, the way a person tries a dialog:
/// press a row where it draws rows, or take it as it stands, then pick with the
/// card cursor, then by number.
fn pick_something(client: &Client, screen: &Screen, tries: u32) -> Result<()> {
    if let Some((x, y)) = dialog_row(screen, tries) {
        client.click(at(x, y))?;
        linger(0.2);
        if client.screen()?.control("Confirm").is_some() {
            return client.click(press("Confirm"));
        }
        return Ok(());
    }
    let asked = &screen.at("/interaction/pending")[question(screen)];
    let least = asked["min"].as_u64().unwrap_or(1);
    let offered = asked["options"].as_array().map_or(&[][..], Vec::as_slice);
    let cards: Vec<&Card> = offered
        .iter()
        .filter_map(Value::as_u64)
        .filter_map(|object| screen.cards().iter().find(|card| card.object == object))
        .take(usize::try_from(least).unwrap_or(1))
        .collect();
    if tries.is_multiple_of(2) && !cards.is_empty() {
        for card in cards {
            let (x, y) = card.rect.centre();
            client.click(at(x, y))?;
        }
        linger(0.2);
        if client.screen()?.control("Confirm").is_some() {
            return client.click(press("Confirm"));
        }
        return client.press("Space");
    }
    match tries % 4 {
        0 if screen.control("Confirm").is_some() => client.click(press("Confirm"))?,
        0 => client.press("Space")?,
        1 => {
            client.press("E")?;
            client.press("Space")?;
        }
        2 => {
            client.press("D")?;
            client.press("E")?;
            client.press("Space")?;
        }
        _ => {
            client.press("Digit1")?;
            client.press("Space")?;
        }
    }
    Ok(())
}

/// This seat's permanents: the near half of the table.
fn own_side(screen: &Screen) -> impl Iterator<Item = &Card> {
    let middle = screen.window().h * 0.5;
    screen
        .cards_in("table")
        .filter(move |card| card.rect.y > middle)
}

fn whole_game(client: &Client, _root: &Path) -> Result<()> {
    client.play_offline()?;
    seed(client, &json!({"lines": 450}))?;
    client.click(press("Play(PlayHouse)"))?;
    client.wait_for("the table", Duration::from_secs(90), |s| {
        s.in_duel() && s.at_rest() && s.cards_in("hand").next().is_some()
    })?;
    let mut stood = false;
    play(client, LONGEST_GAME, |screen| {
        if stood || screen.at("/view/turn").as_u64().unwrap_or(0) < 6 {
            return Ok(());
        }
        if question(screen) != "Priority" {
            return Ok(());
        }
        stood = true;
        seed(client, &json!({"stack": 80}))?;
        linger(1.5);
        let window = screen.window();
        for _ in 0..30 {
            client.scroll(at(window.w * 0.82, window.h * 0.4), 2.0)?;
        }
        read_the_chat(client)
    })?;
    println!("  ended on {}", last_state(client));
    client.click(press("End(Replay)"))?;
    for _ in 0..8 {
        linger(3.0);
        stir(client)?;
    }
    Ok(())
}

/// Opens the chat over the table, reads the lobby's long tab and the log, and
/// shuts it.
fn read_the_chat(client: &Client) -> Result<()> {
    client.click(press("ToggleChat"))?;
    linger(1.0);
    if client.screen()?.control("GotIt").is_some() {
        client.click(press("GotIt"))?;
        linger(0.5);
    }
    let window = client.screen()?.window();
    for tab in ["Tab(Global)", "Tab(Log)", "Tab(All)"] {
        client.click(press(tab))?;
        linger(0.8);
        for _ in 0..25 {
            client.scroll(at(window.w * 0.5, window.h * 0.5), 2.0)?;
        }
    }
    client.click(press("ToggleChat"))?;
    linger(0.8);
    Ok(())
}

/// The names of the acceptance decks, which every build can deal.
pub(super) fn board(root: &Path) -> Result<String> {
    let decks = fs::read_to_string(root.join("data/acceptance-decks.txt")).context("the decks")?;
    let mut names: Vec<&str> = decks
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(count, _)| count.parse::<u32>().is_ok())
        .map(|(_, name)| name.trim())
        .collect();
    names.sort_unstable();
    names.dedup();
    let seated: Vec<String> = names
        .iter()
        .take(44)
        .map(|name| format!("0:{name}"))
        .collect();
    Ok(seated.join(";"))
}

fn full_board(client: &Client, _root: &Path) -> Result<()> {
    client.play_offline()?;
    client.click(press("Play(PlayHouse)"))?;
    client.wait_for("the table", Duration::from_secs(90), |s| {
        s.in_duel() && s.at_rest() && s.cards_in("hand").next().is_some()
    })?;
    client.press("K")?;
    linger(2.0);
    let screen = client.screen()?;
    for card in screen.cards_in("table").step_by(3).take(12) {
        let (x, y) = card.rect.centre();
        client.hover(at(x, y))?;
        linger(0.7);
    }
    play(client, LONGEST_GAME, |_| Ok(()))
}
