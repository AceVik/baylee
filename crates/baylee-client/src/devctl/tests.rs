use super::state::refusal_json;
use super::*;
use bevy::ecs::message::Messages;

#[test]
fn the_snapshot_names_browser_rows_and_their_controls() {
    let (mut app, tx) = harness();
    let mut duel = Duel::default();
    assert!(duel.browser.toggle_by_hand());
    app.insert_resource(duel);
    app.world_mut().spawn((
        crate::hud::TrayCard {
            object: baylee_core::ids::ObjectId::new(9, 0),
        },
        ComputedNode::default(),
        UiGlobalTransform::default(),
    ));
    app.world_mut().spawn((
        crate::hud::TrayFilter,
        ComputedNode::default(),
        UiGlobalTransform::default(),
    ));
    let answers = ask(&tx, "/state", "{}");
    app.update();
    let snapshot: serde_json::Value = serde_json::from_str(&answers.try_recv().unwrap()).unwrap();
    assert_eq!(snapshot["browser"]["open"], true);
    assert_eq!(snapshot["browser"]["typing"], false);
    assert_eq!(snapshot["browser"]["filter"], "");
    let buttons = snapshot["buttons"].as_array().unwrap();
    let row = buttons
        .iter()
        .find(|row| row["kind"] == "browser-card")
        .unwrap();
    assert_eq!(row["object"], 9);
    assert_eq!(row["label"], "9");
    assert_eq!(row["at_x"], 0.0);
    assert_eq!(row["at_y"], 0.0);
    assert!(
        buttons
            .iter()
            .any(|row| { row["kind"] == "browser-control" && row["label"] == "Filter" })
    );
}

/// A key named either way round says the same thing.
///
/// `harness_alias` already let a caller write `2` for `Digit2`, but only
/// the *physical* code went both ways: `Digit2` reported no logical key
/// at all, so a reader that takes its digits as characters — the ability
/// sheet, the subtype filter, a number entry — saw the short spelling and
/// not the canonical one. The harness must not disagree with itself
/// about one key.
#[test]
fn a_key_named_either_way_round_produces_the_same_character() {
    assert_eq!(logical_key("Digit2"), logical_key("2"));
    assert_eq!(logical_key("KeyG"), logical_key("g"));
    assert_eq!(logical_key("Digit0"), Key::Character("0".into()));
    assert_eq!(logical_key("KeyA"), Key::Character("a".into()));
    // Still no logical key where there is none to report.
    assert!(matches!(logical_key("F5"), Key::Unidentified(_)));
    assert!(matches!(logical_key("ShiftLeft"), Key::Unidentified(_)));
}

/// The probe can tell who wrote a refusal, and not merely what it says.
///
/// The case that matters is the third one: a `Verbatim` carrying the
/// *same words* a `Said` renders. In English those two are one string,
/// so a probe reporting only the sentence calls them equal — and the
/// difference between them is the whole of #121. A caller watching for
/// a regression of that bug would have had nothing to watch.
#[test]
fn a_refusal_says_who_wrote_it_and_not_only_what_it_says() {
    use baylee_client_core::i18n::Phrase;

    assert_eq!(refusal_json(None, Lang::De), "null", "nothing refused");

    let mine = Refusal::Said(Phrase::DeedWithdrawn);
    assert_eq!(
        refusal_json(Some(&mine), Lang::De),
        r#"{"said":"DeedWithdrawn","text":"Die Engine bietet das nicht mehr an"}"#,
        "a sentence this client owns is named and translated"
    );

    // Who wrote it is `said`, and never the language: a sentence the
    // engine is known to send reads in the player's language
    // (`i18n::server`), and only a diagnostic this client has never seen
    // keeps the engine's own words.
    let theirs = Refusal::Verbatim("illegal action for your seat".to_string());
    assert_eq!(
        refusal_json(Some(&theirs), Lang::De),
        r#"{"said":null,"text":"Diese Aktion ist für deinen Platz gerade nicht möglich."}"#,
        "another process's known sentence is not named, and is translated"
    );
    let unknown = Refusal::Verbatim("a diagnostic from a newer engine".to_string());
    assert_eq!(
        refusal_json(Some(&unknown), Lang::De),
        r#"{"said":null,"text":"a diagnostic from a newer engine"}"#,
        "an unknown one is neither named nor translated"
    );

    // The two that a one-field probe could not separate.
    let echo = Refusal::Verbatim(Phrase::DeedWithdrawn.text(Lang::En).to_string());
    assert_eq!(
        refusal_json(Some(&echo), Lang::En),
        r#"{"said":null,"text":"The engine no longer offers that"}"#,
    );
    assert_eq!(
        refusal_json(Some(&mine), Lang::En),
        r#"{"said":"DeedWithdrawn","text":"The engine no longer offers that"}"#,
    );
    assert_ne!(
        refusal_json(Some(&mine), Lang::En),
        refusal_json(Some(&echo), Lang::En),
        "same words, different authors — the probe has to say so, or it \
         cannot refuse a regression of the bug it is watching"
    );
}

#[test]
fn a_request_body_yields_its_fields() {
    let body = r#"{"x":100.5,"y":-2,"button":"left","press":true,"shift":false}"#;
    assert_eq!(field(body, "x"), Some("100.5"));
    assert_eq!(field(body, "y"), Some("-2"));
    assert_eq!(field(body, "button"), Some("left"));
    assert!(flag(body, "press"));
    assert!(!flag(body, "shift"));
    assert!(!flag(body, "ctrl"), "a missing flag is not a set one");
    assert_eq!(field(body, "path"), None);
}

/// A lens onto a duel at a plausible window, and the local seat it looks
/// at — everything the rect tests need and nothing else.
fn a_table() -> (crate::table::Lens, baylee_client_core::layout::SeatSlot) {
    use crate::table::Canvas;
    use baylee_client_core::layout::TableLayout;
    let canvas = Canvas::hud(Vec2::new(1728.0, 1052.0));
    let seats: Vec<_> = (0..2).map(baylee_core::ids::PlayerId::new).collect();
    let table = TableLayout::new(&seats, canvas.aspect(), None);
    let lens =
        crate::table::Lens::new(crate::table::CameraRig::home(&table, canvas), canvas.window);
    let slot = *table.local().expect("a local seat");
    (lens, slot)
}

/// A key spelled the way a caller spells it.
///
/// The counter-half is what makes this worth a test: an alias that
/// accepted anything would turn a typo into a key press somewhere else on
/// the board, and the whole reason this exists is that a *refused* key
/// and a key that did nothing are indistinguishable from outside.
#[test]
fn a_bare_letter_is_the_key_it_obviously_means() {
    use bevy::prelude::KeyCode;
    for (name, want) in [
        ("Y", KeyCode::KeyY),
        ("y", KeyCode::KeyY),
        ("N", KeyCode::KeyN),
        ("1", KeyCode::Digit1),
    ] {
        assert_eq!(super::harness_alias(name), Some(want), "{name}");
    }
    for name in ["", "KeyY", "Yes", "-", "Space"] {
        assert_eq!(super::harness_alias(name), None, "{name}");
    }
}

/// A press that is held does not have to say `press` as well.
///
/// The counter-halves are the point again: a bare move must stay a bare
/// move, or every `/pointer` call that only aims the cursor would press
/// the button under it — and `release` must keep winning over `press`,
/// because a call that sends both cannot mean "press and then hold".
#[test]
fn a_held_press_does_not_have_to_say_press_as_well() {
    let word = |body: &str| {
        super::button_deed(body)
            .expect("a well-formed body")
            .map(|deed| deed.word)
    };
    assert_eq!(word(r#"{"x":1,"y":2}"#), None, "a move is only a move");
    assert_eq!(word(r#"{"x":1,"y":2,"press":true}"#), Some("clicked"));
    assert_eq!(word(r#"{"hold":true}"#), Some("held"), "hold implies press");
    assert_eq!(word(r#"{"press":true,"hold":true}"#), Some("held"));
    assert_eq!(word(r#"{"release":true}"#), Some("released"));
    assert_eq!(
        word(r#"{"press":true,"release":true}"#),
        Some("released"),
        "release wins over press"
    );
    assert!(
        super::button_deed(r#"{"press":true,"button":"thumb"}"#).is_err(),
        "an unknown button is still refused"
    );
}

/// Every string in the dump is JSON, apostrophes and em dashes included.
///
/// Against `serde_json` rather than against a written-out expectation,
/// because the claim is that a parser accepts it and not that it looks a
/// particular way. `str::escape_default` passes neither test: it writes
/// `\'` and `\u{2014}`, and the dev board's `Earth King's Lieutenant` was
/// enough to make the whole `/state` answer unreadable — every field in
/// it, not only the name.
#[test]
fn a_name_with_an_apostrophe_in_it_is_still_json() {
    for text in [
        "Earth King's Lieutenant",
        "a \"quoted\" name",
        "a back\\slash",
        "an em — dash",
        "a\nnewline",
    ] {
        let json = format!("{{\"name\":{}}}", quoted(text));
        let back: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{json} is not JSON: {e}"));
        assert_eq!(back["name"], text, "it came back changed: {json}");
    }
}

/// A card's box is as big as a card is drawn.
///
/// Against a scale measured from the felt itself — two points a table
/// unit apart, projected — rather than against a number written down
/// here, because the camera's distance is computed from the window and
/// any constant would be a copy of it. A rect built from a unit quad, or
/// from extents instead of half-extents, misses by a factor and fails.
#[test]
fn a_cards_box_is_the_size_a_card_is_drawn() {
    use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};
    let (lens, slot) = a_table();
    let at = slot.lane_center(baylee_client_core::layout::LaneKind::Creatures);
    let middle = lens.project(at).expect("the lane is in front of the eye");
    let across = (lens.project(at + Vec2::X).expect("and so is a unit east") - middle).length();
    let along = (lens.project(at + Vec2::Y).expect("and a unit north") - middle).length();

    let card = Transform::from_translation(crate::table::to_world(at, crate::table::CARD_LIFT))
        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
    let (_, size) = crate::table::card_box(&lens, &card).expect("a card on it");

    let want = Vec2::new(CARD_WIDTH * across, CARD_HEIGHT * along);
    assert!(
        (size.x - want.x).abs() < want.x * 0.05 && (size.y - want.y).abs() < want.y * 0.05,
        "a card covers {size:?}, and a card's worth of felt covers {want:?}"
    );
}

/// A tapped card covers a wider, shorter box, and the rect says so.
///
/// The claim is that the corners are turned by the card's own transform
/// rather than assumed to be axis-aligned around it: a tapped permanent
/// is the commonest thing on a board and it is a quarter turn over.
#[test]
fn a_tapped_card_reports_the_box_it_actually_covers() {
    let (lens, slot) = a_table();
    let at = slot.lane_center(baylee_client_core::layout::LaneKind::Creatures);
    let world = crate::table::to_world(at, crate::table::CARD_LIFT);
    let flat = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let upright = Transform::from_translation(world).with_rotation(flat);
    let tapped = Transform::from_translation(world)
        .with_rotation(flat * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2));

    let (_, standing) =
        crate::table::card_box(&lens, &upright).expect("a card in front of the eye");
    let (_, turned) = crate::table::card_box(&lens, &tapped).expect("the same card, tapped");
    assert!(
        standing.y > standing.x,
        "an untapped card is taller than it is wide: {standing:?}"
    );
    assert!(
        turned.x > turned.y,
        "a tapped one is wider than it is tall: {turned:?}"
    );
}

/// Builds an app carrying only what `pump` reads, plus the job channel.
fn harness() -> (App, Sender<Job>) {
    let (tx, rx) = channel();
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_message::<MouseButtonInput>()
        .add_message::<KeyboardInput>()
        .add_message::<WindowEvent>()
        .add_message::<CursorMoved>()
        .add_message::<bevy::input::mouse::MouseWheel>()
        .init_resource::<Time<Virtual>>()
        .insert_resource(DevControl {
            jobs: Mutex::new(rx),
            held: Vec::new(),
            clicking: Vec::new(),
            stepping: None,
            shell_jobs: Vec::new(),
            frame: 0,
        })
        .init_resource::<perf::Probe>()
        .init_resource::<perf::Hidden>()
        .add_systems(Update, pump);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    (app, tx)
}

/// Queues one request and hands back the channel its answer will arrive
/// on — which is not always the same frame.
fn ask(tx: &Sender<Job>, path: &str, body: &str) -> Receiver<String> {
    let (reply, answers) = channel();
    tx.send(Job {
        path: path.to_string(),
        body: body.to_string(),
        reply,
    })
    .unwrap();
    answers
}

/// A hundred milliseconds, which is what one frame of the clock harness
/// is worth in raw time.
const RAW: std::time::Duration = std::time::Duration::from_millis(100);

/// The harness with a real clock in it, wound by hand.
///
/// `TimeUpdateStrategy` is bevy's own seam for this, and it is what lets
/// the three clock routes be tested on what the picture does rather than
/// on a flag. It is a *second* harness rather than the first one grown,
/// because `TimePlugin` also takes over when message buffers are swapped
/// — it holds them an extra frame so a fixed-update schedule cannot miss
/// one — and the click and wheel tests read exactly that buffer.
fn clock_harness() -> (App, Sender<Job>) {
    let (mut app, tx) = harness();
    app.add_plugins(bevy::time::TimePlugin)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(RAW));
    (app, tx)
}

/// How far the virtual clock moved on the frame just run.
///
/// The assertion that matters is on the *delta*, never on the flag: a
/// route that set `paused` and left the clock running would pass every
/// test written against `is_paused`, and the whole point of these three
/// endpoints is what the picture does.
///
/// It lags a request by one frame, and honestly so. Bevy sets the clock
/// in `First` and `pump` runs in `PreUpdate`, so the frame a route is
/// answered on already has its delta.
fn advanced(app: &App) -> std::time::Duration {
    app.world().resource::<Time<Virtual>>().delta()
}

/// Everything the window events of one frame said, as short tags.
fn window_events(app: &App) -> Vec<String> {
    app.world()
        .resource::<Messages<WindowEvent>>()
        .iter_current_update_messages()
        .map(|event| match event {
            WindowEvent::CursorMoved(moved) => {
                format!("move {} {}", moved.position.x, moved.position.y)
            }
            WindowEvent::MouseButtonInput(input) => match input.state {
                ButtonState::Pressed => "press".to_string(),
                ButtonState::Released => "release".to_string(),
            },
            other => format!("{other:?}"),
        })
        .collect()
}

/// The regression this endpoint was rebuilt for: a click that reported
/// success without ever reaching bevy's picking backend. Picking reads
/// `WindowEvent`, and it pairs a press with the *last cursor location it
/// saw*, so the move must be a message of its own and must come first.
///
/// Five frames, and the two that are not the press and the release are
/// the repairs `AL6a` asked for. The move is written **twice**, a whole
/// frame apart, because a single one is sometimes simply lost — measured
/// live as a pointer put on a card and `hovered` read fifteen times as
/// `None`, where sending the same move again named the object at once.
/// And the last frame writes nothing: the answer leaves after the frame
/// in which the release was *read*, not the one in which it was written,
/// or a caller that clicks and then asks `/state` is shown the board from
/// before its own click.
#[test]
fn a_click_is_the_move_twice_then_a_press_a_release_and_a_frame_to_read_it() {
    let (mut app, tx) = harness();
    let (reply, answers) = channel();
    tx.send(Job {
        path: "/pointer".to_string(),
        body: r#"{"x":40,"y":60,"press":true}"#.to_string(),
        reply,
    })
    .unwrap();

    app.update();
    assert_eq!(window_events(&app), ["move 40 60"]);
    assert!(
        answers.try_recv().is_err(),
        "the caller is answered when the click is finished, not when it starts"
    );

    app.update();
    assert_eq!(
        window_events(&app),
        ["move 40 60"],
        "the same move again, a frame later: repeated sending is what repairs a lost one"
    );

    app.update();
    assert_eq!(window_events(&app), ["press"]);
    assert!(
        app.world()
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Left)
    );

    app.update();
    assert_eq!(window_events(&app), ["release"]);
    assert!(
        !app.world()
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Left)
    );
    assert!(
        answers.try_recv().is_err(),
        "the release has been written and not yet read by anything"
    );

    app.update();
    assert!(
        window_events(&app).is_empty(),
        "the settling frame writes nothing; it exists to be read in"
    );
    assert!(answers.try_recv().unwrap().contains("\"clicked\":true"));
}

/// A held press stays down, and the release that lifts it is a call of
/// its own.
///
/// Without this the harness could not photograph anything that exists
/// only while a button is down — a drag, or a card giving way under the
/// finger — because press and release were one call and a screenshot
/// cannot be asked for in between. The answer word says which of the
/// three a caller got, so a script cannot mistake a hold for a click.
#[test]
fn a_press_can_be_held_and_let_go_of_separately() {
    let (mut app, tx) = harness();
    let (reply, answers) = channel();
    tx.send(Job {
        path: "/pointer".to_string(),
        body: r#"{"x":40,"y":60,"press":true,"hold":true}"#.to_string(),
        reply,
    })
    .unwrap();

    app.update();
    assert_eq!(window_events(&app), ["move 40 60"]);
    app.update();
    assert_eq!(window_events(&app), ["move 40 60"], "aimed twice");
    app.update();
    assert_eq!(window_events(&app), ["press"]);
    app.update();
    assert!(answers.try_recv().unwrap().contains("\"held\":true"));

    // The button is still down, which is the whole point: the ordinary
    // click would have let go by now.
    assert_eq!(window_events(&app), [] as [String; 0]);
    assert!(
        app.world()
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Left)
    );

    let (reply, answers) = channel();
    tx.send(Job {
        path: "/pointer".to_string(),
        body: r#"{"release":true}"#.to_string(),
        reply,
    })
    .unwrap();
    // The job is drained after the stages have been played, so its own
    // first stage is the next frame's — and the aim comes before the
    // release for a call that carries no coordinates too, because the
    // cursor is put back where it already was and the picking backend is
    // reminded of it.
    app.update();
    app.update();
    app.update();
    assert_eq!(window_events(&app), ["release"]);
    assert!(
        !app.world()
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Left)
    );
    app.update();
    assert!(answers.try_recv().unwrap().contains("\"released\":true"));
}

/// The same regression one gesture along: a wheel written only as a
/// `MouseWheel` message scrolled nothing at all, because it is picking
/// that turns a wheel into the `Pointer<Scroll>` a list listens for, and
/// picking reads `WindowEvent`. Both, or neither is any use.
#[test]
fn a_wheel_is_written_where_picking_reads_it() {
    let (mut app, tx) = harness();
    let (reply, answers) = channel();
    tx.send(Job {
        path: "/scroll".to_string(),
        body: r#"{"y":-6}"#.to_string(),
        reply,
    })
    .unwrap();
    app.update();
    assert!(answers.try_recv().unwrap().contains("\"lines\":-6"));

    let plain: Vec<f32> = app
        .world()
        .resource::<Messages<bevy::input::mouse::MouseWheel>>()
        .iter_current_update_messages()
        .map(|wheel| wheel.y)
        .collect();
    assert_eq!(plain, [-6.0], "nothing else reads the window event");
    let mirrored = window_events(&app);
    assert_eq!(mirrored.len(), 1, "{mirrored:?}");
    assert!(mirrored[0].contains("MouseWheel"), "{mirrored:?}");
}

/// A sideways wheel is written sideways, and says so; its `y` is then
/// nothing rather than the three lines a bare `/scroll` turns.
#[test]
fn a_wheel_can_be_turned_sideways() {
    let (mut app, tx) = harness();
    let (reply, answers) = channel();
    tx.send(Job {
        path: "/scroll".to_string(),
        body: r#"{"x":2}"#.to_string(),
        reply,
    })
    .unwrap();
    app.update();
    let answer = answers.try_recv().unwrap();
    assert!(
        answer.contains("\"x\":2") && answer.contains("\"lines\":0"),
        "{answer}"
    );
    let plain: Vec<(f32, f32)> = app
        .world()
        .resource::<Messages<bevy::input::mouse::MouseWheel>>()
        .iter_current_update_messages()
        .map(|wheel| (wheel.x, wheel.y))
        .collect();
    assert_eq!(plain, [(2.0, 0.0)]);
}

/// A move without `press` presses nothing — the hover path, which is how
/// a card preview is opened — and it is sent twice and answered late all
/// the same.
///
/// It used to be answered on the frame it was written, which made it the
/// one call the caller could not trust: a hover read straight after was
/// read a frame before anything had looked at the move, and the lost
/// move `AL6a` measured was a bare one. The repeat costs two frames and
/// buys a `/pointer` whose answer means the pointer is there.
#[test]
fn a_move_without_a_press_is_only_a_move_and_is_still_sent_twice() {
    let (mut app, tx) = harness();
    let (reply, answers) = channel();
    tx.send(Job {
        path: "/pointer".to_string(),
        body: r#"{"x":10,"y":20}"#.to_string(),
        reply,
    })
    .unwrap();
    app.update();
    assert_eq!(window_events(&app), ["move 10 20"]);
    assert!(
        answers.try_recv().is_err(),
        "answered once the move has been read, not once it has been written"
    );
    app.update();
    assert_eq!(window_events(&app), ["move 10 20"]);
    app.update();
    assert!(window_events(&app).is_empty(), "and nothing was pressed");
    assert!(answers.try_recv().unwrap().contains("\"clicked\":false"));
    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>();
    let cursor = windows.single(app.world()).unwrap().cursor_position();
    assert_eq!(cursor, Some(Vec2::new(10.0, 20.0)));
}

/// A key goes in as `just_pressed` for one frame and is released on the
/// next, the way a real key is — a stuck modifier would change what every
/// later chord means.
#[test]
fn a_key_is_held_for_exactly_one_frame() {
    let (mut app, tx) = harness();
    let (reply, _answers) = channel();
    tx.send(Job {
        path: "/key".to_string(),
        body: r#"{"name":"Space","shift":true}"#.to_string(),
        reply,
    })
    .unwrap();
    app.update();
    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(keys.pressed(KeyCode::Space));
    assert!(keys.pressed(KeyCode::ShiftLeft));
    app.update();
    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(!keys.pressed(KeyCode::Space));
    assert!(!keys.pressed(KeyCode::ShiftLeft));
}

/// The measuring routes answer through the same pump: `/perf` reports a
/// window and starts a new one, `/hide` names what it took away,
/// `/executor` switches every schedule it can reach, and a path that is
/// no route is still refused by name.
#[test]
fn the_measuring_routes_answer_and_an_unknown_path_is_refused() {
    let (mut app, tx) = harness();
    let perf = ask(&tx, "/perf", r#"{"reset":true}"#);
    let hide = ask(&tx, "/hide", r#"{"what":"felt"}"#);
    let executor = ask(&tx, "/executor", r#"{"single":true}"#);
    let nowhere = ask(&tx, "/nowhere", "{}");
    app.update();
    let perf = perf.try_recv().expect("answered");
    assert!(
        perf.contains("\"frame_ms\"") && perf.contains("\"entities\""),
        "{perf}"
    );
    assert_eq!(
        hide.try_recv().expect("answered"),
        r#"{"ok":true,"hidden":["felt"]}"#
    );
    assert!(
        executor
            .try_recv()
            .expect("answered")
            .contains("\"single\":true")
    );
    assert!(
        nowhere
            .try_recv()
            .expect("answered")
            .contains("no such endpoint: /nowhere")
    );
}

/// A tenth speed is a tenth of the picture, not a flag saying so.
#[test]
fn a_slowed_clock_moves_a_tenth_as_far() {
    let (mut app, tx) = clock_harness();
    let answers = ask(&tx, "/timescale", r#"{"speed":0.1}"#);
    app.update();
    assert!(answers.try_recv().unwrap().contains("\"speed\":0.1"));
    app.update();
    assert_eq!(advanced(&app), RAW / 10);

    // Zero is not a pause, and a refused request must leave the clock
    // where it was rather than half-applying itself.
    let refused = ask(&tx, "/timescale", r#"{"speed":0}"#);
    app.update();
    assert!(refused.try_recv().unwrap().contains("\"error\""));
    app.update();
    assert_eq!(advanced(&app), RAW / 10);
}

/// The harness has to keep answering while the picture is stopped, which
/// is why `pump` counts frames and not seconds. A paused clock that took
/// the harness with it would be a screenshot nobody could ever ask for.
#[test]
fn a_pause_stops_the_picture_and_not_the_harness() {
    let (mut app, tx) = clock_harness();
    let paused = ask(&tx, "/pause", "{}");
    app.update();
    assert!(paused.try_recv().unwrap().contains("\"paused\":true"));
    app.update();
    assert_eq!(advanced(&app), std::time::Duration::ZERO);

    let health = ask(&tx, "/health", "{}");
    app.update();
    let answer = health.try_recv().expect("a stopped clock still answers");
    assert!(answer.contains("\"paused\":true"), "got {answer}");
    assert!(answer.contains("\"frame\":3"), "got {answer}");

    let running = ask(&tx, "/pause", r#"{"paused":false}"#);
    app.update();
    assert!(running.try_recv().unwrap().contains("\"paused\":false"));
    app.update();
    assert_eq!(advanced(&app), RAW);
}

/// A step is counted in frames and answered at the end of them, for the
/// reason a click is: a caller that was told "ok" up front would take its
/// screenshot of the frame it started from.
#[test]
fn a_step_runs_the_frames_it_asked_for_and_then_stops_again() {
    let (mut app, tx) = clock_harness();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();

    let stepped = ask(&tx, "/step", r#"{"frames":3}"#);
    app.update();
    for frame in 1..=3 {
        assert!(
            stepped.try_recv().is_err(),
            "answered before frame {frame} of 3"
        );
        app.update();
        assert_eq!(
            advanced(&app),
            RAW,
            "the clock was still stopped on frame {frame} of 3"
        );
    }

    assert!(stepped.try_recv().unwrap().contains("\"frames\":3"));
    app.update();
    assert_eq!(
        advanced(&app),
        std::time::Duration::ZERO,
        "the clock was left running after the step counted out"
    );
}

/// Two steps at once would share one countdown and one reply channel, so
/// the second is refused rather than quietly stealing the first.
#[test]
fn a_second_step_is_refused_while_the_first_is_running() {
    let (mut app, tx) = clock_harness();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    let first = ask(&tx, "/step", r#"{"frames":4}"#);
    app.update();

    let second = ask(&tx, "/step", r#"{"frames":1}"#);
    app.update();
    assert!(second.try_recv().unwrap().contains("\"error\""));
    assert!(first.try_recv().is_err(), "the first step lost its answer");
}
