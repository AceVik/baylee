use super::*;
use baylee_client_core::gamelog::PlayerSpan;
use baylee_core::ids::ObjectId;
use baylee_view::{DayNight, GameStatic, LogEntry, LogEvent, LogTail, SeatIdentity};
use bevy::ecs::system::RunSystemOnce;

fn fonts() -> UiFonts {
    UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    }
}

/// The panel stands on the right strip and covers none of it: the mana
/// pool stays readable while it is up, because it stays up (#264).
///
/// Read out of the two nodes, as the menu's test reads its own: the
/// claim is about where two nodes are drawn.
#[test]
fn the_panel_stands_on_the_strip_and_covers_none_of_it() {
    let strip = super::super::strip_node(super::super::StripSide::Right);
    let panel = root_node();
    let (Val::Px(strip_bottom), Val::Px(strip_h), Val::Px(bottom)) =
        (strip.bottom, strip.height, panel.bottom)
    else {
        panic!("the strip and the panel are placed in pixels");
    };
    assert!(
        (bottom - (strip_bottom + strip_h)).abs() < 0.01,
        "the panel's bottom is {bottom}, the strip's top {}",
        strip_bottom + strip_h
    );
    assert_eq!(panel.right, strip.right, "and against the same margin");
}

/// A wide window gets the panel's own size, and a narrow one the room
/// it has: bounded both ways, so a panel that ignored the window and one
/// that always shrank both fail.
#[test]
fn a_narrow_window_gets_the_room_there_is() {
    // A share of a middling window, and the ceiling of a large one.
    let (w, h) = log_size((1600.0, 1000.0));
    assert!((w - 1600.0 * LOG_W_SHARE).abs() < 0.01, "{w}");
    assert!((h - LOG_H_MAX).abs() < 0.01, "{h}");
    assert_eq!(log_size((4000.0, 2000.0)), (LOG_W_MAX, LOG_H_MAX));
    assert_eq!(log_size((1000.0, 560.0)), (LOG_W, LOG_H));
    let (w, h) = log_size((300.0, 200.0));
    assert!(
        (w - (300.0 - 2.0 * EDGE)).abs() < 0.01,
        "a 300 wide window's panel is {w} wide"
    );
    assert!(
        (h - (200.0 - (STRIP_H - STRIP_LIP))).abs() < 0.01,
        "a 200 high band's panel is {h} high"
    );
}

/// The panel stands under the report button (#309) in every window,
/// down to a phone's on its side: grown up from the strip, its top is
/// the button's foot at the highest, and a short window gets a shorter
/// panel rather than one reaching into the corner.
#[test]
fn the_panel_stands_under_the_report_button() {
    let Val::Px(bottom) = root_node().bottom else {
        panic!("the panel is placed in pixels");
    };
    for window in [
        Vec2::new(1728.0, 1052.0),
        Vec2::new(1280.0, 720.0),
        Vec2::new(1024.0, 640.0),
        Vec2::new(800.0, 600.0),
        Vec2::new(932.0, 430.0),
        Vec2::new(430.0, 932.0),
        Vec2::new(360.0, 640.0),
    ] {
        let (_, height) = log_size(log_band(window));
        let top = window.y - bottom - height;
        let corner = report_corner(window);
        assert!(
            top >= corner.max.y,
            "in a {window} window the panel's top is {top}, over the \
                 button's foot at {}",
            corner.max.y
        );
    }
}

fn line(text: &str, names: &[(usize, usize)], times: u32) -> LogLine {
    LogLine {
        at: 0,
        players: Vec::new(),
        index: 0,
        turn: 1,
        times,
        header: false,
        text: text.to_string(),
        names: names
            .iter()
            .map(|&(start, end)| NameSpan {
                token: None,
                range: start..end,
                id: ObjectId::new(5, 0),
                card: None,
            })
            .collect(),
        ability: None,
        subject: None,
        crowned: false,
        phase: None,
        follows: false,
    }
}

/// Each name is a piece of its own, the words round it are kept, and a
/// range the sentence cannot be cut at costs the weight and not the line.
#[test]
fn a_name_is_its_own_piece() {
    let bolt = line("You cast Lightning Bolt at Ana", &[(9, 23), (27, 30)], 1);
    assert_eq!(
        pieces(&bolt),
        [
            ("You cast ".to_string(), Piece::Words),
            ("Lightning Bolt".to_string(), Piece::Name),
            (" at ".to_string(), Piece::Words),
            ("Ana".to_string(), Piece::Name),
        ]
    );
    let first = line("Ana drew a card", &[(0, 3)], 1);
    assert_eq!(
        pieces(&first),
        [
            ("Ana".to_string(), Piece::Name),
            (" drew a card".to_string(), Piece::Words)
        ]
    );
    // Inside the `ö`, which is bytes 2 and 3: not a place to cut.
    let broken = line("Björn drew", &[(0, 3)], 1);
    assert_eq!(pieces(&broken), [("Björn drew".to_string(), Piece::Words)]);
}

fn bolt_identity() -> CardIdentity {
    CardIdentity {
        index: CardIndex::new(7),
        print: baylee_core::ids::PrintRef::new(3),
        face: 0,
    }
}

/// Every player a line names is bold, "you" as much as a name; a card
/// the seat was shown is a link in brackets, and so is a token; the
/// words between are kept (#300).
#[test]
fn players_are_bold_and_what_the_seat_was_shown_is_a_link() {
    let mut said = line(
        "You cast Lightning Bolt at Ana's Goblin",
        &[(9, 23), (33, 39)],
        1,
    );
    said.names[0].card = Some(bolt_identity());
    said.names[1].token = Some(12);
    said.players = vec![
        PlayerSpan {
            range: 0..3,
            player: PlayerId::new(0),
        },
        PlayerSpan {
            range: 27..30,
            player: PlayerId::new(1),
        },
    ];
    let bolt = LogLink {
        card: Some(bolt_identity()),
        token: None,
    };
    let goblin = LogLink {
        card: None,
        token: Some(12),
    };
    assert_eq!(
        pieces(&said),
        [
            ("You".to_string(), Piece::Player),
            (" cast ".to_string(), Piece::Words),
            ("[Lightning Bolt]".to_string(), Piece::Link(bolt)),
            (" at ".to_string(), Piece::Words),
            ("Ana".to_string(), Piece::Player),
            ("'s ".to_string(), Piece::Words),
            ("[Goblin]".to_string(), Piece::Link(goblin)),
        ]
    );
    assert_eq!(
        bolt.art(ArtSize::Normal),
        Some(ImageKey::new(bolt_identity().print, 0, ArtSize::Normal)),
        "the preview is the printing the line named"
    );
    assert_eq!(
        goblin.art(ArtSize::Small),
        Some(ImageKey::token(12, ArtSize::Small))
    );
}

/// The hidden-information guard (#300): a card the line did not show the
/// seat is no link, has nothing to preview, and puts nothing on the row
/// that the pointer could hover into a preview. A face-down card is
/// named, a weight up, and that is all; a hidden card is not even named.
#[test]
fn a_card_the_seat_was_not_shown_is_no_link() {
    let face_down = line("Ana cast a face-down card", &[(9, 25)], 1);
    assert_eq!(face_down.names[0].card, None);
    assert_eq!(face_down.names[0].token, None);
    assert_eq!(
        pieces(&face_down),
        [
            ("Ana cast ".to_string(), Piece::Words),
            ("a face-down card".to_string(), Piece::Name),
        ]
    );
    assert_eq!(LogLink::of(&face_down.names[0]), None);
    let hidden = line("Ana drew a card", &[], 1);
    assert_eq!(
        pieces(&hidden),
        [("Ana drew a card".to_string(), Piece::Words)]
    );

    let mut app = App::new();
    app.insert_resource(fonts());
    let row = app
        .world_mut()
        .run_system_once(move |mut commands: Commands, fonts: Res<UiFonts>| {
            spawn_line(
                &mut commands,
                &fonts,
                Lang::En,
                &face_down,
                &PANEL_INKS,
                None,
            )
        })
        .expect("spawns");
    app.update();
    let mut links = app.world_mut().query::<&LogLink>();
    assert_eq!(
        links.iter(app.world()).count(),
        0,
        "a link on a face-down card"
    );
    let mut hoverable = app.world_mut().query::<&Pickable>();
    assert!(
        hoverable.iter(app.world()).all(|pick| !pick.is_hoverable),
        "a row with nothing to preview answers the pointer"
    );
    assert!(app.world().get_entity(row).is_ok());
}

/// A pointer event aimed at one entity, from where the pointer stands.
fn at<E: crate::test_pointer::Aim>(entity: Entity, event: E) -> E::Event {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::window::WindowRef;
    crate::test_pointer::aim(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Primary
                    .normalize(Some(Entity::PLACEHOLDER))
                    .expect("a window"),
            ),
            position: Vec2::new(40.0, 50.0),
        },
        event,
        entity,
    )
}

/// The pointer on a link is what the preview follows: onto it, off it,
/// and off it by the link's going, which no pointer reports (#300).
#[test]
fn the_hover_follows_the_pointer_onto_a_link_and_off_it() {
    let mut app = App::new();
    app.init_resource::<Duel>()
        .add_message::<PointerOver>()
        .add_message::<PointerOut>()
        .add_systems(Update, hover_log_links);
    let link = LogLink {
        card: Some(bolt_identity()),
        token: None,
    };
    let span = app.world_mut().spawn(link).id();
    let words = app.world_mut().spawn_empty().id();
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    let hovered = |app: &App| app.world().resource::<Duel>().hovered_log;

    app.world_mut()
        .write_message(at(words, crate::test_pointer::Over { hit: hit.clone() }));
    app.update();
    assert_eq!(hovered(&app), None, "words are no link");

    app.world_mut()
        .write_message(at(span, crate::test_pointer::Over { hit: hit.clone() }));
    app.update();
    assert_eq!(
        hovered(&app),
        Some(LogHover {
            span,
            link,
            at: Vec2::new(40.0, 50.0),
            card: None,
        })
    );
    app.world_mut()
        .write_message(at(span, crate::test_pointer::Out { hit: hit.clone() }));
    app.update();
    assert_eq!(hovered(&app), None, "left by the pointer");

    app.world_mut()
        .write_message(at(span, crate::test_pointer::Over { hit }));
    app.update();
    assert!(hovered(&app).is_some());
    app.world_mut().despawn(span);
    app.update();
    assert_eq!(hovered(&app), None, "left by the link's going");
}

/// A harness for [`hover_log_links`] with one card lens laid out the way
/// the UI schedule leaves it: 480 x 670 physical pixels centred on
/// (600, 400), at half a logical pixel per physical one. Returns the app
/// and the lens.
fn lens_over_a_card() -> (App, Entity) {
    let mut app = App::new();
    app.init_resource::<Duel>()
        .add_message::<PointerOver>()
        .add_message::<PointerOut>()
        .add_systems(Update, hover_log_links);
    let lens = app
        .world_mut()
        .spawn((
            LogLink {
                card: Some(bolt_identity()),
                token: None,
            },
            CardLens,
            ComputedNode {
                size: Vec2::new(480.0, 670.0),
                inverse_scale_factor: 0.5,
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(600.0, 400.0)),
        ))
        .id();
    (app, lens)
}

fn point_at(app: &mut App, entity: Entity) -> Option<LogHover> {
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    app.world_mut()
        .write_message(at(entity, crate::test_pointer::Over { hit }));
    app.update();
    app.world().resource::<Duel>().hovered_log
}

/// A link that lies over a whole card carries that card's box, in the
/// logical window pixels the preview is placed in: the node's physical
/// box, centred, scaled by its inverse scale factor. Without it the
/// preview would open beside the pointer, wherever on the card it came in.
#[test]
fn a_link_over_a_card_hands_over_the_cards_box_in_logical_pixels() {
    let (mut app, lens) = lens_over_a_card();
    let hover = point_at(&mut app, lens).expect("the lens is a link");
    assert_eq!(hover.span, lens, "the hover names the lens");
    assert_eq!(
        hover.card,
        Some(Rect::from_center_size(
            Vec2::new(300.0, 200.0),
            Vec2::new(240.0, 335.0)
        )),
        "the card's box is its physical box halved"
    );
}

/// The UI scale is part of what turns a node's pixels into the window's
/// logical ones (`PreviewAt::in_ui` divides it back out): at a scale of
/// 2 and an inverse scale factor of 0.5 the two cancel.
#[test]
fn the_cards_box_takes_the_ui_scale_into_account() {
    let (mut app, lens) = lens_over_a_card();
    app.insert_resource(UiScale(2.0));
    let hover = point_at(&mut app, lens).expect("the lens is a link");
    assert_eq!(
        hover.card,
        Some(Rect::from_center_size(
            Vec2::new(600.0, 400.0),
            Vec2::new(480.0, 670.0)
        )),
        "factor 0.5 times scale 2 leaves the physical box"
    );
}

/// A word in a line is a link without a card: it has no box to open the
/// preview at, so it keeps the pointer (`card: None`), even when the
/// node happens to be laid out.
#[test]
fn a_link_that_is_not_a_card_has_no_box() {
    let (mut app, lens) = lens_over_a_card();
    app.world_mut().entity_mut(lens).remove::<CardLens>();
    let hover = point_at(&mut app, lens).expect("the word is a link");
    assert_eq!(hover.card, None, "a word is not a card's picture");
    assert_eq!(hover.at, Vec2::new(40.0, 50.0), "it stays at the pointer");
}

/// A preview must not outlive its panel: shut with a link under the
/// pointer, the panel keeps its lines (only hidden), so no `Out` comes and
/// the preview of a line hovered as `L` shut the panel stayed on the
/// table. A span has no visibility of its own and reads its sentence's.
/// On the old code the hover was cleared only by the link's going, so the
/// hidden case below kept its `Some`.
#[test]
fn the_hover_is_dropped_when_the_panel_is_shut_under_it() {
    let mut app = App::new();
    app.init_resource::<Duel>()
        .add_message::<PointerOver>()
        .add_message::<PointerOut>()
        .add_systems(Update, hover_log_links);
    let link = LogLink {
        card: Some(bolt_identity()),
        token: None,
    };
    let sentence = app.world_mut().spawn(InheritedVisibility::VISIBLE).id();
    let span = app.world_mut().spawn((link, ChildOf(sentence))).id();
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    let hovered = |app: &App| app.world().resource::<Duel>().hovered_log;

    app.world_mut()
        .write_message(at(span, crate::test_pointer::Over { hit: hit.clone() }));
    app.update();
    app.update();
    assert!(
        hovered(&app).is_some(),
        "a link in a visible sentence stays hovered"
    );

    app.world_mut()
        .entity_mut(sentence)
        .insert(InheritedVisibility::HIDDEN);
    app.update();
    assert_eq!(hovered(&app), None, "its sentence went hidden under it");

    // The pointer is over a link in a shut panel (events do not stop at
    // a hidden node's door in every backend): refused at once.
    app.world_mut()
        .write_message(at(span, crate::test_pointer::Over { hit }));
    app.update();
    assert_eq!(hovered(&app), None, "and a hidden link is not hovered");

    // A span that does carry a visibility of its own is read as itself.
    let own = app
        .world_mut()
        .spawn((link, InheritedVisibility::HIDDEN))
        .id();
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    app.world_mut()
        .write_message(at(own, crate::test_pointer::Over { hit }));
    app.update();
    assert_eq!(hovered(&app), None, "a span hidden itself");
}

/// A turn's heading stands for its beginning: no caption for it nor for
/// the beginning phase; each later phase is captioned before its first
/// line only; a new turn starts again; a line with no phase gives none.
#[test]
fn a_phase_is_captioned_before_its_first_line_only() {
    let mut run = PhaseRun::default();
    let in_phase = |turn: u32, phase: Option<Phase>| {
        let mut l = line("x", &[], 1);
        l.turn = turn;
        l.phase = phase;
        l
    };
    let mut head = in_phase(1, None);
    head.header = true;
    assert_eq!(run.heading(&head), None, "a turn's heading is no caption");
    assert_eq!(
        run.heading(&in_phase(1, Some(Phase::Beginning))),
        None,
        "the beginning phase has none of its own"
    );
    assert_eq!(
        run.heading(&in_phase(1, Some(Phase::FirstMain))),
        Some(Phase::FirstMain),
        "the first line of a later phase"
    );
    assert_eq!(
        run.heading(&in_phase(1, Some(Phase::FirstMain))),
        None,
        "not the second"
    );
    assert_eq!(
        run.heading(&in_phase(1, None)),
        None,
        "a line with no phase gives none and does not end the phase"
    );
    assert_eq!(run.heading(&in_phase(1, Some(Phase::FirstMain))), None);
    assert_eq!(
        run.heading(&in_phase(1, Some(Phase::Combat))),
        Some(Phase::Combat)
    );
    assert_eq!(
        run.heading(&in_phase(1, Some(Phase::FirstMain))),
        Some(Phase::FirstMain),
        "a phase met again after another is a new run"
    );

    // A new turn resets it: the same phase on turn 2 is captioned again,
    // and its heading makes the beginning quiet once more.
    assert_eq!(
        run.heading(&in_phase(2, Some(Phase::FirstMain))),
        Some(Phase::FirstMain),
        "the same phase, a turn later"
    );
    let mut head = in_phase(3, None);
    head.header = true;
    assert_eq!(run.heading(&head), None);
    assert_eq!(run.heading(&in_phase(3, Some(Phase::Beginning))), None);
    assert_eq!(
        run.heading(&in_phase(3, Some(Phase::Ending))),
        Some(Phase::Ending)
    );
}

/// The log reads a modal card's modes through the cast chooser's door:
/// Archdruid's Charm's third mode is its English exile sentence with no
/// bullet, and a card with no modes has none. On the old code the
/// lookup had no `mode` and answered `None` for every card.
#[test]
fn the_log_says_a_modes_words_as_the_cast_chooser_does() {
    let charm = baylee_cards::decks::by_name("Archdruid's Charm").expect("in the pool");
    let bolt = baylee_cards::decks::by_name("Lightning Bolt").expect("in the pool");
    let rules = |card| RulesFace { card, face: 0 };
    let lookup = LogTexts(None);
    assert_eq!(
        lookup.mode(rules(charm), 2).as_deref(),
        Some("Exile target artifact or enchantment.")
    );
    assert_eq!(lookup.mode(rules(bolt), 0), None, "a card with no modes");
    assert_eq!(lookup.text(charm, 0), None, "and no text filed");
}

/// A line that happened more than once says how often, after the
/// sentence, and a line that happened once says nothing.
#[test]
fn a_repeat_says_how_often() {
    assert_eq!(
        times(&line("You drew", &[], 3), Lang::En),
        Some(" (×3)".into())
    );
    assert_eq!(
        times(&line("Du ziehst", &[], 2), Lang::De),
        Some(" (×2)".into())
    );
    assert_eq!(times(&line("You drew", &[], 1), Lang::En), None);
}

fn tail(from: u32, count: u32) -> LogTail {
    LogTail {
        from,
        entries: (from..from + count)
            .map(|i| LogEntry {
                at: 0,
                turn: 1,
                repeat: 1,
                event: LogEvent::Mulliganed {
                    player: PlayerId::new(u8::try_from(i % 2).unwrap_or(0)),
                },
                phase: None,
            })
            .collect(),
    }
}

/// A panel over a book, with `sync_log` running.
fn panel_over(lines: u32) -> (App, Entity) {
    panel_with(&tail(0, lines), None)
}

/// A panel over a book of `entries`, read by seat 0 with `statics` as
/// its roster.
fn panel_with(entries: &LogTail, statics: Option<GameStatic>) -> (App, Entity) {
    let mut app = App::new();
    let mut duel = Duel::default();
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    duel.log.append(entries, &view);
    duel.view = Some(view);
    duel.statics = statics;
    duel.log_open = true;
    app.insert_resource(duel)
        .insert_resource(fonts())
        .insert_resource(crate::settings::ClientSettings::default())
        .init_resource::<LogRevision>()
        .add_systems(Update, sync_log);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let panel = {
        let mut commands = Commands::new(&mut queue, app.world());
        spawn_log_panel(&mut commands)
    };
    queue.apply(app.world_mut());
    app.update();
    (app, panel)
}

/// A book of `events` from line `from` on, all in turn 1.
fn book(from: u32, events: Vec<LogEvent>) -> LogTail {
    LogTail {
        from,
        entries: events
            .into_iter()
            .map(|event| LogEntry {
                at: 0,
                turn: 1,
                repeat: 1,
                event,
                phase: None,
            })
            .collect(),
    }
}

fn mulligan(seat: u8) -> LogEvent {
    LogEvent::Mulliganed {
        player: PlayerId::new(seat),
    }
}

/// A line about the table: nobody's.
fn dawn() -> LogEvent {
    LogEvent::DayNight { now: DayNight::Day }
}

/// Two seats on two teams.
fn two_teams() -> GameStatic {
    GameStatic {
        decision_secs: None,
        reconnect_secs: None,
        view_version: baylee_view::VIEW_VERSION,
        game_id: "g".into(),
        your_seat: PlayerId::new(0),
        seats: (0..2)
            .map(|seat| SeatIdentity {
                player: PlayerId::new(seat),
                display_name: format!("seat {seat}"),
                is_ai: false,
                away: false,
                team: Some(seat),
            })
            .collect(),
        prints: vec![],
    }
}

/// Each row's swatch colour, in order.
fn swatches(app: &mut App) -> Vec<Color> {
    let rows = rows(app);
    let world = app.world();
    rows.iter()
        .map(|row| {
            let swatch = world.get::<Children>(*row).expect("a row's parts")[0];
            world.get::<BackgroundColor>(swatch).expect("a swatch").0
        })
        .collect()
}

/// A line wears the colour of the seat it is about, the one that seat's
/// bar wears: the reader's own, another seat's team's, and none for a line
/// about the table. Both ways a line is drawn: with the panel as it
/// opens, and arriving after it.
///
/// The reader's team is not the reader's colour, so a swatch that asked
/// only for teams would fail here too.
#[test]
fn a_line_wears_the_colour_of_the_seat_it_is_about() {
    let opened = book(0, vec![mulligan(0), mulligan(1), dawn()]);
    let (mut app, _) = panel_with(&opened, Some(two_teams()));
    let (own, theirs) = (palette::ACTIVE, team_color(Some(1)));
    assert_ne!(own, team_color(Some(0)), "the test cannot tell them apart");
    assert_eq!(
        swatches(&mut app),
        [own, theirs, Color::NONE],
        "the lines drawn as the panel opened"
    );

    {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        let view = duel.view.clone().expect("a view");
        duel.log
            .append(&book(3, vec![dawn(), mulligan(1), mulligan(0)]), &view);
    }
    app.update();
    assert_eq!(
        swatches(&mut app)[3..],
        [Color::NONE, theirs, own],
        "the lines that arrived after it"
    );
}

fn rows(app: &mut App) -> Vec<Entity> {
    let mut lists = app.world_mut().query_filtered::<&Children, With<LogList>>();
    let world = app.world();
    lists
        .iter(world)
        .flat_map(|kids| kids.iter().collect::<Vec<_>>())
        .filter(|entity| world.get::<virtual_list::LogSpacer>(*entity).is_none())
        .collect()
}

/// Lines that arrive are added under the ones drawn, and nothing drawn
/// is written again; a change of language writes everything again.
///
/// The row entities are the measurement: a panel that redrew the whole
/// log for each new line would show the same rows and hold new entities.
#[test]
fn new_lines_are_added_and_nothing_drawn_is_drawn_again() {
    let (mut app, _) = panel_over(3);
    let first = rows(&mut app);
    assert_eq!(first.len(), 3, "three lines, three rows");

    {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        let view = duel.view.clone().expect("a view");
        duel.log.append(&tail(3, 2), &view);
    }
    app.update();
    let second = rows(&mut app);
    assert_eq!(second.len(), 5, "the two new lines were not added");
    assert_eq!(
        second[..3],
        first[..],
        "the three drawn rows were drawn again for two new lines"
    );

    app.world_mut()
        .resource_mut::<crate::settings::ClientSettings>()
        .lang = "de".to_string();
    app.update();
    let third = rows(&mut app);
    assert_eq!(third.len(), 5);
    assert!(
        !third.contains(&first[0]),
        "a new language left the lines written in the old one"
    );
}

/// An empty book says so, and its first line takes that sentence's place
/// rather than standing under it.
#[test]
fn an_empty_log_says_so_until_its_first_line() {
    let (mut app, _) = panel_over(0);
    assert_eq!(rows(&mut app).len(), 1, "the empty log's one sentence");
    {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        let view = duel.view.clone().expect("a view");
        duel.log.append(&tail(0, 2), &view);
    }
    app.update();
    assert_eq!(
        rows(&mut app).len(),
        2,
        "the empty log's sentence stayed under the lines that ended it"
    );
}

/// The list's scroll range ends at its last line, measured by Bevy's own
/// layout with the shipped fonts: lines that wrap, lines that do not, a
/// turn's heading, in a list narrower than most of them.
///
/// Bounded both ways, so a range that stops short of the last line fails
/// as well as one that runs past it into blank space.
#[test]
fn the_list_ends_at_its_last_line() {
    let (mut app, fonts) = crate::face::tests::layout_app();
    // A turn's heading last, as a turn that has only just begun leaves
    // it: the one row whose own reach nothing under it covers. Its seat
    // has a long name, so that it wraps and its wrapped height is the one
    // held to account.
    let said = [
        "Lightning Bolt deals 3 damage to Ana",
        "You draw a card",
        "steady 1 puts an ability of Charming Prince on the stack",
        "You are at 37 life (was 40)",
        "Esper Sentinel moves from the battlefield into exile, and then from exile back onto the battlefield under its owner's control",
        "Turn 14 · Maximiliane von Hohenzollern-Sigmaringen's turn",
    ];
    let list = {
        let mut commands = app.world_mut().commands();
        let list = commands
            .spawn(Node {
                width: px(260),
                height: px(120),
                flex_direction: FlexDirection::Column,
                row_gap: px(LOG_ROW_GAP),
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .id();
        for (at, text) in said.iter().enumerate() {
            let mut line = line(text, &[], 1);
            line.header = at == said.len() - 1;
            let row = spawn_line(
                &mut commands,
                &fonts,
                Lang::En,
                &line,
                &PANEL_INKS,
                Some(Color::NONE),
            );
            commands.entity(list).add_child(row);
        }
        list
    };
    app.world_mut().flush();
    app.update();
    app.update();
    let world = app.world();
    let kids: Vec<Entity> = world.get::<Children>(list).expect("rows").iter().collect();
    let rows: f32 = kids
        .iter()
        .map(|row| world.get::<ComputedNode>(*row).expect("laid out").size().y)
        .sum();
    // The heading's margin is part of the column too.
    let spaced = rows + LOG_ROW_GAP * (kids.len() - 1) as f32 + LOG_TURN_GAP;
    let range = world
        .get::<ComputedNode>(list)
        .expect("laid out")
        .content_size()
        .y;
    assert!(
        rows > 6.0 * LOG_LINE_PT,
        "the rows were never laid out, so the range proves nothing: {rows}"
    );
    assert!(
        (range - spaced).abs() < 1.0,
        "the list scrolls to {range}, and its lines end at {spaced}"
    );
}

/// A list and its pill, laid out as `size` inside `content`.
fn laid_out(app: &mut App, content: f32) -> (Entity, Entity) {
    let list = app
        .world_mut()
        .spawn((
            LogList,
            LogFollow::default(),
            ScrollPosition::default(),
            ComputedNode {
                size: Vec2::new(100.0, 200.0),
                content_size: Vec2::new(100.0, content),
                inverse_scale_factor: 1.0,
                ..default()
            },
        ))
        .id();
    let pill = app
        .world_mut()
        .spawn((
            LogNewBelow,
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    (list, pill)
}

/// The pill's two sentences, as the real pill carries them: new lines
/// below shown, back to the latest hidden.
fn pill_words(app: &mut App) -> (Entity, Entity) {
    let word = |app: &mut App, unseen: bool| {
        app.world_mut()
            .spawn((
                LogPillWords { unseen },
                Node {
                    display: if unseen { Display::Flex } else { Display::None },
                    ..default()
                },
            ))
            .id()
    };
    (word(app, true), word(app, false))
}

/// A list scrolled more than half its height above its end offers the
/// way back to the latest line, in the words for that; lines that
/// arrived under it make the pill say so instead; at the end it is not
/// there. The half is exact: 100 of a 200 high list is not yet "away",
/// and 101 is.
#[test]
fn a_list_scrolled_well_up_offers_the_way_back_in_its_own_words() {
    let mut app = App::new();
    app.add_systems(Update, follow_the_log);
    let (list, pill) = laid_out(&mut app, 1000.0);
    let (new_lines, latest) = pill_words(&mut app);
    let shown =
        |app: &App, e: Entity| app.world().get::<Node>(e).expect("a node").display != Display::None;
    let scroll = |app: &mut App, y: f32| {
        app.world_mut()
            .get_mut::<ScrollPosition>(list)
            .expect("a list")
            .y = y;
        app.update();
    };

    // At its end: following, nothing to offer.
    app.update();
    assert!(!shown(&app, pill), "a list at its end has no pill");

    // A little up, and exactly half a list up (end is 800, list is 200).
    scroll(&mut app, 750.0);
    assert!(!shown(&app, pill), "50 up is not away");
    scroll(&mut app, 700.0);
    assert!(!shown(&app, pill), "exactly half a list up is not yet away");

    // Past half: the pill, saying "back to the latest" and not "new".
    scroll(&mut app, 699.0);
    assert!(shown(&app, pill), "more than half a list up shows the pill");
    assert!(shown(&app, latest), "with the way back");
    assert!(!shown(&app, new_lines), "and not the new-lines sentence");
    scroll(&mut app, 300.0);
    assert!(shown(&app, pill) && shown(&app, latest) && !shown(&app, new_lines));

    // Lines arrive under it: the pill's other sentence.
    app.world_mut()
        .get_mut::<LogFollow>(list)
        .expect("a list")
        .unseen = true;
    app.update();
    assert!(shown(&app, pill), "new lines show the pill");
    assert!(shown(&app, new_lines), "saying so");
    assert!(!shown(&app, latest), "and not the other sentence");

    // A little up with new lines under it: the pill, new-lines words.
    scroll(&mut app, 750.0);
    assert!(shown(&app, pill) && shown(&app, new_lines) && !shown(&app, latest));

    // Pressed (stuck), the list goes to its end and the pill goes.
    app.world_mut()
        .get_mut::<LogFollow>(list)
        .expect("a list")
        .stuck = true;
    app.update();
    assert!(!shown(&app, pill), "at the end the pill is gone");
    assert!(
        !app.world().get::<LogFollow>(list).expect("a list").unseen,
        "and the lines are seen"
    );
    assert!(
        shown(&app, latest) && !shown(&app, new_lines),
        "words follow `unseen`"
    );
}

/// The list keeps to its end while the player leaves it there, lets go
/// when they scroll up, lights the pill when lines arrive under it, and
/// goes back to the end when the pill is pressed.
#[test]
fn the_list_follows_its_newest_line_until_the_player_scrolls_up() {
    let mut app = App::new();
    app.add_systems(Update, follow_the_log);
    let (list, pill) = laid_out(&mut app, 1000.0);
    let at = |app: &App| app.world().get::<ScrollPosition>(list).expect("a list").y;
    let shown = |app: &App| app.world().get::<Node>(pill).expect("a pill").display != Display::None;

    app.update();
    assert!((at(&app) - 800.0).abs() < 0.01, "it did not go to its end");

    // The wheel.
    app.world_mut()
        .get_mut::<ScrollPosition>(list)
        .expect("a list")
        .y = 300.0;
    app.update();
    assert!(
        (at(&app) - 300.0).abs() < 0.01,
        "it took the list back from the player"
    );

    // Lines arrive: the sync marks them unseen, and the list grows.
    app.world_mut()
        .get_mut::<LogFollow>(list)
        .expect("a list")
        .unseen = true;
    app.world_mut()
        .get_mut::<ComputedNode>(list)
        .expect("a list")
        .content_size
        .y = 1200.0;
    app.update();
    assert!(
        (at(&app) - 300.0).abs() < 0.01,
        "new lines took the list away"
    );
    assert!(shown(&app), "lines arrived under it and nothing said so");

    // The pill.
    app.world_mut()
        .get_mut::<LogFollow>(list)
        .expect("a list")
        .stuck = true;
    app.update();
    assert!(
        (at(&app) - 1000.0).abs() < 0.01,
        "the pill did not take it to its end"
    );
    assert!(!shown(&app), "the pill stayed after the lines were seen");

    // And a wheel back to the end follows again: to within the slack
    // of it, where a wheel's last notch leaves it.
    app.world_mut()
        .get_mut::<ScrollPosition>(list)
        .expect("a list")
        .y = 500.0;
    app.update();
    app.world_mut()
        .get_mut::<ScrollPosition>(list)
        .expect("a list")
        .y = 998.0;
    app.update();
    app.world_mut()
        .get_mut::<ComputedNode>(list)
        .expect("a list")
        .content_size
        .y = 1400.0;
    app.update();
    assert!(
        (at(&app) - 1200.0).abs() < 0.01,
        "scrolled back to the end, it did not follow the next line"
    );
}
