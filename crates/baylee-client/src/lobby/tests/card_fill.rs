//! The card cache's fill as a player meets it: the block in Settings ›
//! Network & Gateway, the header's mark, and what the lobby tells the fill.

use super::super::card_fill::{
    FillBar, FillBoard, FillButtons, FillMark, FillPress, Readout, steering,
};
use super::settings::settings_at;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::cachefill::{
    Budget, FillChoices, Pace, Progress, Reach, Round, Source, Status, Stop, Why, plan, words,
};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::settings_map::Section;
use std::collections::HashSet;
use std::time::Duration;

/// A press on the block.
fn fill_press(press: FillPress) -> Press {
    Press::Settings(SettingsPress::Fill(press))
}

/// A reading the app will not read over: the test decides what it says.
fn board(app: &mut App, progress: Progress) {
    let mut board = app.world_mut().resource_mut::<FillBoard>();
    board.progress = progress;
    board.read_at = f64::MAX;
}

fn running() -> Progress {
    Progress {
        status: Status::Running,
        round: Some(Round::Thumbnails),
        rounds: 2,
        cards_done: 4_210,
        cards_total: 10_754,
        pictures_done: 5_433,
        pictures_total: 21_729,
        bytes: 412_000_000,
        cached: Some(1_130_000_000),
        budget: 2_000_000_000,
        elapsed: Duration::from_mins(72),
        speed: Some(1_240_000),
        left: Some(Duration::from_secs(2_410)),
        ..Progress::default()
    }
}

/// What a row, a preview and the table ask for, with the base taken off:
/// another test may move the process-wide base while this one runs.
fn tail(url: &str) -> String {
    ["/small/", "/normal/"]
        .iter()
        .find_map(|size| url.find(size))
        .map_or_else(|| url.to_string(), |at| url[at..].to_string())
}

/// The join: for every kind of card in the registry, what the deck builder's
/// row, its preview and the table build, signed in or not, is what the plan
/// files. The fill's own URLs are built by another path, and a difference of
/// one letter would fill a cache no draw reads.
#[test]
fn the_plan_holds_what_a_row_a_preview_and_the_table_ask_for() {
    use baylee_client_core::images::{
        ArtSize, Face, SCRYFALL_CDN, art_path, gateway_art_base, image_url, image_url_at,
    };
    let mirror = gateway_art_base("https://gw.example");
    let rows = baylee_cards::pool::rows();
    let sources: Vec<Source<'_>> = rows
        .iter()
        .filter(|row| row.coverage == "implemented")
        .map(|row| Source {
            name: &row.english_name,
            scryfall_id: row.scryfall_id,
            back: row.has_back_image,
            basic: row.basic_land,
        })
        .collect();
    let planned: HashSet<String> = plan(SCRYFALL_CDN, &sources, Reach::Everything)
        .iter()
        .map(|picture| art_path(&picture.url, SCRYFALL_CDN).into_owned())
        .collect();
    let mut checked = 0;
    let mut backs = 0;
    for row in rows.iter().filter(|row| row.coverage == "implemented") {
        let card = super::super::offline::pool_row(row);
        let hover = hover_of_card(&card);
        let large = hover.url.expect("every implemented card has a printing");
        assert!(planned.contains(&tail(&large)), "{large}");
        let thumbnail = crate::lobby::thumbnails::small(&large);
        assert!(planned.contains(&tail(&thumbnail)), "{thumbnail}");
        if let Some(back) = hover.back_url {
            backs += 1;
            assert!(planned.contains(&tail(&back)), "{back}");
        }
        // The table, for a printing nobody picked: the same id by way of a
        // print table's entry.
        let entry = baylee_view::PrintEntry {
            scryfall_id: row.scryfall_id.to_string(),
            lang: "en".to_string(),
            finish: baylee_view::Finish::Normal,
        };
        let on_table = image_url(&entry, Face::Front, ArtSize::Small).expect("a url");
        assert!(planned.contains(&tail(&on_table)), "{on_table}");
        // And signed in, where the same picture is drawn from a mirror.
        let at_mirror = image_url_at(&mirror, &entry, Face::Front, ArtSize::Small).expect("a url");
        assert!(
            planned.contains(art_path(&at_mirror, &mirror).as_ref()),
            "{at_mirror}"
        );
        checked += 1;
    }
    assert!(
        checked > 1_000 && backs > 50,
        "{checked} cards, {backs} backs"
    );
}

/// Who is signed in has no say: the fill asks the same of the front door and
/// of a session at a gateway with a mirror.
#[test]
fn the_fill_needs_nobody_signed_in() {
    let mut app = headless();
    let choices = Some(FillChoices::default());
    let state = app.world().resource::<LobbyState>();
    assert!(state.lobby.token().is_none());
    let at_the_door = steering(state, choices, DuelPhase::Closed);
    assert!(at_the_door.choices.on && !at_the_door.playing && !at_the_door.held);
    stocked(&mut app);
    let mut state = app.world_mut().resource_mut::<LobbyState>();
    state.art_cache = true;
    state.gateway = "http://gw.example".to_string();
    assert_eq!(steering(&state, choices, DuelPhase::Closed), at_the_door);
}

/// A game rests the fill in every phase of a table, and only then.
#[test]
fn every_phase_of_a_table_is_playing() {
    let mut app = headless();
    stocked(&mut app);
    let state = app.world().resource::<LobbyState>();
    let choices = Some(FillChoices::default());
    assert!(!steering(state, choices, DuelPhase::Closed).playing);
    for phase in [DuelPhase::Opening, DuelPhase::Playing, DuelPhase::Finished] {
        assert!(steering(state, choices, phase).playing, "{phase:?}");
    }
    assert!(
        !steering(state, None, DuelPhase::Closed).choices.on,
        "no settings file, no fill"
    );
}

/// Requirement 6: the rows stand in Network & Gateway, with the block's
/// numbers written into them, and the screen is not rebuilt for it.
#[test]
fn the_block_shows_every_number() {
    let mut app = settings_at(Section::Network);
    board(&mut app, running());
    app.update();
    let shown = labels(&mut app);
    for words in [
        "Downloading",
        "4,210 of 10,754 cards · thumbnails, round 1 of 2",
        "412 MB downloaded · 1 h 12 min so far · 1.2 MB/s · about 40 min left",
        "Cache: 1.1 GB of 2.0 GB",
        "25%",
    ] {
        assert!(shown.iter().any(|said| said == words), "{words}: {shown:?}");
    }
    for row in [
        Phrase::RowCardFill,
        Phrase::RowCardFillPace,
        Phrase::RowCardFillReach,
        Phrase::RowCardFillBudget,
    ] {
        let said = row.text(Lang::En);
        assert!(shown.iter().any(|s| s == said), "{said}: {shown:?}");
    }
    let mut bars = app.world_mut().query_filtered::<&Node, With<FillBar>>();
    let width = bars.single(app.world()).expect("one bar").width;
    assert_eq!(width, Val::Percent(25.0));
}

/// A reading writes text in place: the block's nodes survive it, because a
/// rebuild twice a second would cost the keyboard its place.
#[test]
fn a_new_reading_rebuilds_nothing() {
    let mut app = settings_at(Section::Network);
    board(&mut app, running());
    app.update();
    let block = |app: &mut App| {
        let mut bars = app.world_mut().query_filtered::<Entity, With<FillBar>>();
        bars.single(app.world()).expect("one bar")
    };
    let before = block(&mut app);
    let later = Progress {
        pictures_done: 6_000,
        ..running()
    };
    board(&mut app, later);
    app.update();
    assert_eq!(block(&mut app), before);
    let mut readouts = app.world_mut().query::<(&Readout, &Text)>();
    let share = readouts
        .iter(app.world())
        .find(|(readout, _)| **readout == Readout::Percent)
        .map(|(_, text)| text.0.clone());
    assert_eq!(share.as_deref(), Some("27%"));
}

/// Every control of the block is on the keyboard's way: a stop missing from
/// the screen's table is one Tab never reaches.
#[test]
fn every_control_of_the_block_can_be_reached_by_tab() {
    use crate::settingsui::keys::SETTINGS;
    use crate::shellkit::focus::Stop;
    let mut app = settings_at(Section::Network);
    let mut stops = app.world_mut().query::<&Stop>();
    let drawn: HashSet<&str> = stops
        .iter(app.world())
        .filter(|stop| stop.id.starts_with("card-fill"))
        .map(|stop| stop.id)
        .collect();
    assert!(drawn.len() >= 5, "{drawn:?}");
    for id in drawn {
        assert!(SETTINGS.stops.contains(&id), "{id}");
    }
}

/// Every choice writes the settings file's field, and the screen follows.
#[test]
fn the_choices_are_kept_for_this_device() {
    let mut app = settings_at(Section::Network);
    let choices = |app: &App| {
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .card_fill
    };
    assert_eq!(choices(&app), FillChoices::default());
    press(&mut app, fill_press(FillPress::Pace(Pace::Brisk)));
    assert_eq!(choices(&app).pace, Pace::Brisk);
    press(&mut app, fill_press(FillPress::Reach(Reach::Thumbnails)));
    assert_eq!(choices(&app).reach, Reach::Thumbnails);
    press(&mut app, fill_press(FillPress::Budget(Budget::Five)));
    assert_eq!(choices(&app).budget, Budget::Five);
    press(&mut app, fill_press(FillPress::Toggle));
    assert!(!choices(&app).on);
}

/// Pause lasts for this run, and which button stands follows the state.
#[test]
fn pause_and_resume_change_places() {
    let mut app = settings_at(Section::Network);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.settings_view.fill.buttons = FillButtons::Going;
    }
    board(&mut app, running());
    settle(&mut app);
    assert!(presses(&mut app).contains(&fill_press(FillPress::Pause)));
    let pause = Phrase::CardFillPause.text(Lang::En);
    assert!(labels(&mut app).iter().any(|said| said == pause));
    press(&mut app, fill_press(FillPress::Pause));
    let state = app.world().resource::<LobbyState>();
    assert!(state.settings_view.fill.held);
    assert!(
        steering(state, Some(FillChoices::default()), DuelPhase::Closed).held,
        "the fill is told"
    );
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.settings_view.fill.buttons = FillButtons::Paused;
    }
    settle(&mut app);
    let resume = Phrase::CardFillResume.text(Lang::En);
    assert!(labels(&mut app).iter().any(|said| said == resume));
}

/// A failure's reason is said, and it offers to try again.
#[test]
fn a_stop_says_why_and_offers_to_try_again() {
    let mut app = settings_at(Section::Network);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.settings_view.fill.buttons = FillButtons::Stopped;
    }
    board(
        &mut app,
        Progress {
            status: Status::Stopped(Stop::DiskFull),
            ..running()
        },
    );
    settle(&mut app);
    let said = Phrase::CardFillStopDisk.text(Lang::En);
    assert!(labels(&mut app).iter().any(|s| s == said));
    assert!(presses(&mut app).contains(&fill_press(FillPress::TryAgain)));
}

/// Emptying the cache takes two presses, and leaving disarms it.
#[test]
fn emptying_the_cache_asks_first() {
    let mut app = settings_at(Section::Network);
    let sure = Phrase::CardFillEmptySure.text(Lang::En);
    assert!(!labels(&mut app).iter().any(|said| said == sure));
    press(&mut app, fill_press(FillPress::Empty));
    settle(&mut app);
    assert!(labels(&mut app).iter().any(|said| said == sure));
    let warning = Phrase::CardFillEmptyAsk.text(Lang::En);
    assert!(labels(&mut app).iter().any(|said| said == warning));
    press(
        &mut app,
        Press::Settings(SettingsPress::Section(Section::Audio)),
    );
    let state = app.world().resource::<LobbyState>();
    assert!(!state.settings_view.fill.armed, "leaving disarms it");
}

/// The header's mark stands only while pictures are on their way, says how
/// far, and opens the block.
#[test]
fn the_mark_stands_while_pictures_are_on_their_way() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    settle(&mut app);
    let shown = |app: &mut App| {
        let mut marks = app.world_mut().query_filtered::<&Node, With<FillMark>>();
        marks
            .iter(app.world())
            .next()
            .expect("a mark in the header")
            .display
    };
    board(&mut app, Progress::default());
    app.update();
    assert_eq!(shown(&mut app), Display::None, "off");
    board(&mut app, running());
    app.update();
    assert_eq!(shown(&mut app), Display::Flex, "running");
    assert!(labels(&mut app).iter().any(|said| said == "25%"));
    board(
        &mut app,
        Progress {
            status: Status::Paused(Why::Match),
            ..running()
        },
    );
    app.update();
    assert_eq!(shown(&mut app), Display::None, "resting");
    board(&mut app, running());
    app.update();
    press(&mut app, Press::Header(HeaderPress::CardFill));
    app.update();
    press(&mut app, fill_press(FillPress::Show));
    let state = app.world().resource::<LobbyState>();
    assert!(state.settings.is_open());
    assert_eq!(state.settings_section(), Section::Network);
}

/// The hint carries the status and the count.
#[test]
fn the_marks_hint_says_how_far() {
    let said = words::hint(Lang::En, &running());
    assert_eq!(
        said,
        "Card pictures: Downloading · 4,210 of 10,754 cards · thumbnails, round 1 of 2"
    );
}
