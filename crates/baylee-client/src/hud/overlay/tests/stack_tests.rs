//! The slip under the preview, and the stack panel.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

// ---- the slip under the preview -------------------------------------
//
// `hud::slip` is tested on its own arithmetic and on its own system;
// what those cannot say is whether `sync_overlay` ever builds one.
// "Declared but never wired" is a bug this client has shipped before.

/// Ondu Cleric's German zen printing, read from the catalog 2026-09-24.
const ONDU_CLERIC: &str = "Immer wenn der Ondu-Kleriker oder ein anderer Verbündeter unter deiner Kontrolle ins Spiel kommt, kannst du soviele Lebenspunkte dazuerhalten, wie du Verbündete kontrollierst.";

/// A duel with one ability on the stack, its source on the battlefield,
/// and the pointer on the stack entry.
fn hovering_the_stack(hovered: bool) -> (Duel, crate::cardtext::CardTexts) {
    use baylee_client_core::test_support::{ViewBuilder, printed, statics, token};

    let texts = crate::cardtext::CardTexts::filed(crate::cardtext::fixture::german(
        "Ondu Cleric",
        "Ondu-Kleriker",
        Some(ONDU_CLERIC),
    ));
    let card = crate::cardtext::fixture::card("Ondu Cleric");
    let mut ability = token(30, 0, "Ondu Cleric", 0, 0);
    ability.card = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: ObjectId::new(7, 0),
        ability: None,
        rules: Some(baylee_view::RulesFace { card, face: 0 }),
        text: Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 1,
        }),
    });
    let mut duel = Duel {
        interaction: Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(baylee_engine::choice::LegalActions::default()),
            },
            PlayerId::new(0),
        )),
        statics: Some(statics(8)),
        hovered: hovered.then(|| ObjectId::new(30, 0)),
        ..Duel::default()
    };
    duel.view = Some(
        ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![crate::cardtext::fixture::showing(
                    printed(7, 0, "Ondu Cleric", 7),
                    card,
                )],
            )
            .with_stack(vec![ability])
            .build(),
    );
    crate::rebuild_board(&mut duel);
    (duel, texts)
}

#[test]
fn copied_oracle_row_grows_inside_the_existing_scroll_viewport() {
    let (mut duel, texts) = hovering_the_stack(false);
    let card = crate::cardtext::fixture::card("Vesuvan Doppelganger");
    let mut view = duel.view.clone().unwrap();
    view.stack[0].stack_item = Some(baylee_view::StackItem::Ability {
        source: ObjectId::new(7, 0),
        ability: Some(baylee_core::ids::AbilityRef { card, index: 0 }),
        rules: Some(baylee_view::RulesFace { card, face: 0 }),
        text: None,
        token: None,
    });
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    let mut app = overlay_with(duel, texts);
    let mut rows = app
        .world_mut()
        .query_filtered::<&Node, With<super::super::StackRowCard>>();
    let row = rows.single(app.world()).unwrap();
    assert_eq!(
        row.height,
        Val::Auto,
        "the complete Oracle determines row height"
    );
    assert_eq!(row.min_height, px(stack::STACK_FULL_HEIGHT));
    assert_eq!(row.max_height, Val::Auto, "no second clipping ceiling");
    let mut bodies = app
        .world_mut()
        .query_filtered::<&Node, With<stack::StackBody>>();
    assert_eq!(
        bodies.single(app.world()).unwrap().overflow,
        Overflow::scroll_y()
    );
}

#[test]
#[allow(clippy::float_cmp)] // offsets are copied exactly, without arithmetic
fn a_deep_stack_keeps_scroll_and_selection_across_hover_and_language_changes() {
    let (mut duel, texts) = hovering_the_stack(false);
    let mut view = duel.view.clone().unwrap();
    let template = view.stack[0].clone();
    view.stack = (100..2100)
        .map(|id| {
            let mut entry = template.clone();
            entry.id = ObjectId::new(id, 0);
            entry
        })
        .collect();
    duel.receive_view(view);
    let mut app = overlay_with(duel, texts);
    let scroll = 24_600.0;
    let mut body = app
        .world_mut()
        .query_filtered::<&mut ScrollPosition, With<stack::StackBody>>();
    body.single_mut(app.world_mut()).unwrap().y = scroll;
    app.update();
    let mut body = app
        .world_mut()
        .query_filtered::<(&ScrollPosition, &Children), With<stack::StackBody>>();
    let (position, children) = body.single(app.world()).unwrap();
    assert_eq!(position.y, scroll);
    assert!(
        children.len() <= 18,
        "only the visible window and spacers are spawned"
    );
    let mark = ObjectId::new(1800, 0);
    {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        duel.stack_selected = Some(mark);
        duel.hovered = Some(mark);
    }
    app.world_mut()
        .resource_mut::<crate::settings::ClientSettings>()
        .lang = "de".into();
    app.update();
    assert_eq!(body.single(app.world()).unwrap().0.y, scroll);
    assert_eq!(app.world().resource::<Duel>().stack_selected, Some(mark));
    assert!(
        said(&mut app)
            .iter()
            .any(|line| line.contains(Phrase::StackRunTo.text(Lang::De)))
    );
    let root = body.single(app.world()).unwrap().1[0];
    app.update();
    assert_eq!(
        body.single(app.world()).unwrap().1[0],
        root,
        "an idle frame retains the tree"
    );
}

fn overlay_with(duel: Duel, texts: crate::cardtext::CardTexts) -> App {
    // The stack panel draws pictures, and asking for one is an
    // `AssetServer::load` — which spawns on the IO pool and panics
    // without it. Idempotent, so several tests may ask.
    bevy::tasks::IoTaskPool::get_or_init(Default::default);
    let mut app = bar_of(duel);
    app.insert_resource(texts);
    app.world_mut().resource_mut::<HudRevision>().set_changed();
    app.update();
    app
}

/// The whole point of the feature, end to end: the pointer is on a stack
/// entry, and the sentence the row abbreviates is written out under the
/// preview.
#[test]
fn hovering_a_stack_entry_writes_its_sentence_out() {
    let (duel, texts) = hovering_the_stack(true);
    let mut app = overlay_with(duel, texts);
    let said = said(&mut app);
    // Three, and the third is the interesting one. The row says it cut
    // to four lines; the sheet says it whole; and the preview's *card*
    // says it too, because no art has arrived in a headless test and a
    // card with no picture falls back to a constructed face. That third
    // one is exactly why `slip::says` asks `FaceCtx::always` — the
    // player's own choice — rather than "did a face come back": the
    // fallback is the ordinary case offline, and suppressing the sheet
    // there would silence it wherever a gateway serves no art.
    assert_eq!(
        said.iter()
            .filter(|line| line.contains("Immer wenn der Ondu-Kleriker"))
            .count(),
        3,
        "the row, the fallback face and the sheet: {said:?}"
    );
    let mut sheets = app
        .world_mut()
        .query_filtered::<Entity, With<super::super::slip::Washing>>();
    assert!(
        sheets.iter(app.world()).count() > 0,
        "and it is on a sheet the wash can reach"
    );
    // And the sheet is *in* the bubble. A slip spawned and never added to
    // anything is still in the world, still carries its `Washing`, and
    // still answers `said` — so without this the test passes on a sheet
    // nobody can see.
    let mut orphans = app
        .world_mut()
        .query_filtered::<Entity, (With<super::super::slip::Washing>, Without<ChildOf>)>();
    assert_eq!(
        orphans.iter(app.world()).count(),
        0,
        "a sheet hanging off nothing is drawn nowhere"
    );
}

/// The counter-test, without which the one above would pass on a client
/// that drew a sheet under every preview it ever opened.
#[test]
fn a_pointer_on_nothing_opens_no_sheet() {
    let (duel, texts) = hovering_the_stack(false);
    let mut app = overlay_with(duel, texts);
    let said = said(&mut app);
    // Once, and it is the row's own: the panel abbreviates whether
    // anyone is looking or not, and the sheet is what the looking buys.
    assert_eq!(
        said.iter()
            .filter(|line| line.contains("Immer wenn der Ondu-Kleriker"))
            .count(),
        1,
        "the row says it and nothing else does: {said:?}"
    );
    let mut sheets = app
        .world_mut()
        .query_filtered::<Entity, With<super::super::slip::Washing>>();
    assert_eq!(sheets.iter(app.world()).count(), 0);
}

/// A stack entry outlives the game it belonged to.
///
/// The owner conceded as a function test with one of Sheoldred's
/// triggers on the stack, and the end screen came up with the trigger
/// still drawn beside it — an entry whose whole job is to say that
/// something is about to happen, in a game where nothing will. The
/// prompt bar has stopped *whole* at `GameOver` for exactly this reason,
/// and the stack panel was the fourth reader of `Duel::ending` that
/// nobody had connected.
#[test]
fn the_stack_panel_stops_with_the_prompt_bar_when_the_game_is_over() {
    let (duel, texts) = hovering_the_stack(false);
    let mut app = overlay_with(duel, texts);
    let running = said(&mut app);
    assert!(
        running.iter().any(|line| line.contains("Ondu")),
        "the premise: a running game draws the stack it has: {running:?}"
    );

    let (mut duel, texts) = hovering_the_stack(false);
    duel.interaction = Some(baylee_client_core::Interaction::new(
        baylee_engine::choice::Pending::GameOver(GameResult {
            winner: Some(Victor::Player(PlayerId::new(0))),
            reason: EndReason::LastPlayerStanding,
        }),
        PlayerId::new(0),
    ));
    crate::rebuild_board(&mut duel);
    let mut app = overlay_with(duel, texts);
    let over = said(&mut app);
    assert!(
        !over.iter().any(|line| line.contains("Ondu")),
        "the same stack is still drawn under a finished game: {over:?}"
    );
}

/// The stack stands in the window's top-right corner, and the report
/// button (#309) is over everything there: the panel starts under it,
/// or its head and first entry are what the button covers.
#[test]
fn the_stack_stands_under_the_report_button() {
    let mut duel = duel_with_a_stack();
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    crate::rebuild_board(&mut duel);
    let mut app = overlay_with(duel, crate::cardtext::CardTexts::default());
    let mut q = app.world_mut().query::<(&Node, &ZIndex)>();
    let tops: Vec<Val> = q
        .iter(app.world())
        .filter(|(_, z)| z.0 == Z_STACK)
        .map(|(node, _)| node.top)
        .collect();
    assert_eq!(tops.len(), 1, "the premise: one stack panel");
    let corner = report_corner(Vec2::new(1280.0, 720.0));
    assert!(
        matches!(tops[0], Val::Px(top) if top >= corner.max.y),
        "the panel's top {:?} is under the button's foot {}",
        tops[0],
        corner.max.y
    );
}

/// The stack's head names the seat the table is waiting for, and it is
/// the seat the **engine asked** rather than the one holding priority.
///
/// `waiting_line` has been tested since it was written, and that test
/// says nothing at all about this: it is a pure function over a name, so
/// it passed just as well while `spawn_stack_panel` fed it a seat the
/// host had taken from `priority_holder`. A seat asked to declare
/// blockers or to discard holds no priority, so the head fell silent on
/// exactly the questions a player most needs pointing at — the defect
/// #81 is about, one surface further along than the caret.
///
/// So this one runs the system and reads the words off the tree. The
/// counter-half is the seat nobody is being asked: the line is absent
/// once the game is over, which is the only question the engine asks
/// nobody, and its absence is what proves the assertion is reading this
/// line rather than some constant of the panel's.
#[test]
fn the_stack_says_which_seat_the_table_is_waiting_for() {
    let named = |awaiting: Option<u8>| {
        let mut duel = duel_with_a_stack();
        let view = duel.view.as_mut().expect("the seat has a view");
        view.awaiting = awaiting.map(PlayerId::new);
        // The default roster seats only the viewing player, and the
        // line under test names somebody else.
        let mut statics = baylee_client_core::test_support::statics(8);
        statics.seats.push(baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: "sharp 1".to_string(),
            is_ai: true,
            away: false,
            team: None,
        });
        duel.statics = Some(statics);
        crate::rebuild_board(&mut duel);
        said(&mut overlay_with(
            duel,
            crate::cardtext::CardTexts::default(),
        ))
    };

    let opponent = named(Some(1));
    assert!(
        opponent.contains(&"waiting for sharp 1".to_string()),
        "the head names the seat being asked: {opponent:?}"
    );

    let me = named(Some(0));
    assert!(
        me.contains(&Phrase::WaitingForYou.text(Lang::En).to_string()),
        "and addresses this seat rather than naming it: {me:?}"
    );

    let nobody = named(None);
    assert!(
        !nobody.iter().any(|line| line.starts_with("waiting for")),
        "nothing is asked of anyone, so the head says nothing: {nobody:?}"
    );
}

// ---- a long sentence scrolls in its box (the owner, 08.10.2026) -----
//
// *"Sometimes the effects on the stack are quite long and you can't see the
// target. The effect text area should then be scrollable, including a
// (visible) scrollbar."* Bevy's layout does not run in this harness, so a
// box is told how big it and its sentence are, as `hud::scroll`'s tests do.

/// The Ondu Cleric trigger, aimed at the other seat: a long German sentence
/// and a target, with the system that keeps the sentence's box.
fn aimed_entry(hovered: bool) -> App {
    let (mut duel, texts) = hovering_the_stack(hovered);
    let mut view = duel.view.clone().unwrap();
    view.stack[0].targets = vec![baylee_core::ids::TargetRef::Player(PlayerId::new(1))];
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    let mut app = overlay_with(duel, texts);
    app.init_resource::<stack::StackTextScroll>()
        .add_systems(Update, stack::stack_text.after(sync_overlay));
    app.update();
    app
}

/// The sentence's box, and the bar beside it.
fn sentence_box(app: &mut App) -> (Entity, Entity) {
    let text_box = app
        .world_mut()
        .query_filtered::<Entity, With<stack::StackTextBox>>()
        .single(app.world())
        .expect("one full row, one box");
    let bar = app
        .world_mut()
        .query::<(Entity, &stack::StackTextBar)>()
        .iter(app.world())
        .find(|(_, bar)| bar.text_box == text_box)
        .expect("the box has its bar")
        .0;
    (text_box, bar)
}

/// Tells the box how tall it is drawn and how tall its sentence is, as a
/// layout would.
fn lay_out(app: &mut App, text_box: Entity, content: f32) {
    let view = stack::STACK_SENTENCE_LINES * stack::STACK_SENTENCE_LINE;
    app.world_mut().entity_mut(text_box).insert(ComputedNode {
        size: Vec2::new(220.0, view.min(content)),
        content_size: Vec2::new(220.0, content),
        ..default()
    });
}

fn shown(app: &App, bar: Entity) -> Visibility {
    *app.world()
        .get::<Visibility>(bar)
        .expect("a bar has a visibility")
}

fn offset_of(app: &App, text_box: Entity) -> f32 {
    app.world()
        .get::<ScrollPosition>(text_box)
        .expect("a box keeps its offset")
        .y
}

fn ancestors(app: &App, mut entity: Entity) -> Vec<Entity> {
    let mut line = Vec::new();
    while let Some(parent) = app.world().get::<ChildOf>(entity).map(ChildOf::parent) {
        line.push(parent);
        entity = parent;
    }
    line
}

/// The owner's report, as a tree: the sentence stands whole in a box that
/// scrolls, the row grows to hold it (a fixed 164 px clip was what hid the
/// targets), and the targets stand above the box, outside it, so no length
/// of sentence takes them out of sight.
#[test]
fn a_long_sentence_scrolls_in_its_box_under_targets_that_stay_in_sight() {
    let mut app = aimed_entry(false);
    let (text_box, _) = sentence_box(&mut app);
    let node = app.world().get::<Node>(text_box).unwrap();
    assert_eq!(node.overflow, Overflow::scroll_y(), "the box scrolls");
    assert_eq!(
        node.max_height,
        px(stack::STACK_SENTENCE_LINES * stack::STACK_SENTENCE_LINE),
        "and is four of the sentence's lines tall at most"
    );
    let (row, height) = app
        .world_mut()
        .query_filtered::<(Entity, &Node), With<super::super::StackRowCard>>()
        .single(app.world())
        .map(|(row, node)| (row, node.height))
        .unwrap();
    assert_eq!(height, Val::Auto, "the row holds what it draws");

    // Whole, and never cut: the clause a cut dropped is the one a player
    // most often needed.
    let mut words = String::new();
    let mut spans = app.world_mut().query::<(Entity, &TextSpan)>();
    for (span, text) in spans.iter(app.world()) {
        if ancestors(&app, span).contains(&text_box) {
            words.push_str(&text.0);
        }
    }
    assert!(words.contains(ONDU_CLERIC), "the whole sentence: {words:?}");
    assert!(!words.contains('…'), "and nothing cut from it");

    // The target, above the box and outside it.
    let arrow = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find(|(arrow, text)| text.0 == "→" && ancestors(&app, *arrow).contains(&row))
        .expect("the row points at its target")
        .0;
    let line = ancestors(&app, arrow);
    assert!(!line.contains(&text_box), "the target does not scroll");
    let (targets, body) = (line[0], line[1]);
    let scroller = ancestors(&app, text_box)[0];
    assert_eq!(ancestors(&app, scroller)[0], body, "one column holds both");
    let order = app.world().get::<Children>(body).unwrap();
    let at = |e: Entity| order.iter().position(|c| c == e).unwrap();
    assert!(
        at(targets) < at(scroller),
        "the target stands above the text"
    );
}

/// A sentence that runs over its box puts its scrollbar up; one that fits
/// leaves it down — hidden, not taken out, so the text does not reflow when
/// it comes. The first half fails on a client that never shows the bar, the
/// second on one that always does.
#[test]
fn the_scrollbar_stands_only_while_the_sentence_runs_over() {
    let mut app = aimed_entry(false);
    let (text_box, bar) = sentence_box(&mut app);
    assert_eq!(shown(&app, bar), Visibility::Hidden, "not measured yet");
    lay_out(&mut app, text_box, 200.0);
    app.update();
    assert_eq!(shown(&app, bar), Visibility::Inherited, "it runs over");
    assert!(app.world().resource::<stack::StackTextScroll>().runs_over);
    lay_out(&mut app, text_box, 40.0);
    app.update();
    assert_eq!(shown(&app, bar), Visibility::Hidden, "it fits");
    assert!(!app.world().resource::<stack::StackTextScroll>().runs_over);
    assert_eq!(
        app.world().get::<Node>(bar).unwrap().display,
        Display::Flex,
        "hidden, never out of the layout"
    );
}

/// The arrows and the page keys scroll the sentence of the entry the cursor
/// is on — a line, and a box less a line — and only while it runs over.
#[test]
fn the_focused_entry_s_sentence_scrolls_under_the_arrows_and_page_keys() {
    use baylee_client_core::prefs::Keymap;
    fn fired(key: KeyCode) -> crate::keys::Fired {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(key);
        crate::keys::Fired::of(&keys, &Keymap::standard())
    }
    fn take(app: &mut App, key: KeyCode) -> bool {
        let mut scroll = *app.world().resource::<stack::StackTextScroll>();
        let took = stack::stack_text_keys(
            fired(key),
            app.world().resource::<Duel>(),
            Some(&mut scroll),
        );
        app.insert_resource(scroll);
        app.update();
        took
    }
    let mut app = aimed_entry(true);
    let (text_box, _) = sentence_box(&mut app);
    // Fits: the arrows are left to whatever else wants them.
    lay_out(&mut app, text_box, 40.0);
    app.update();
    assert!(!take(&mut app, KeyCode::ArrowDown), "nothing to scroll");

    lay_out(&mut app, text_box, 200.0);
    app.update();
    assert!(take(&mut app, KeyCode::ArrowDown));
    let line = offset_of(&app, text_box);
    assert!(
        (line - stack::STACK_SENTENCE_LINE).abs() < 0.01,
        "a line: {line}"
    );
    assert!(take(&mut app, KeyCode::PageDown));
    let view = stack::STACK_SENTENCE_LINES * stack::STACK_SENTENCE_LINE;
    let paged = offset_of(&app, text_box);
    assert!(
        (paged - (line + view - stack::STACK_SENTENCE_LINE)).abs() < 0.01,
        "a box less a line: {paged}"
    );
    assert!(take(&mut app, KeyCode::PageUp));
    assert!(take(&mut app, KeyCode::ArrowUp));
    assert!(
        offset_of(&app, text_box).abs() < 0.01,
        "and back to the top"
    );

    // The cursor elsewhere: the keys are not the entry's.
    app.world_mut().resource_mut::<Duel>().hovered = None;
    app.update();
    assert!(
        !take(&mut app, KeyCode::ArrowDown),
        "the cursor is elsewhere"
    );
}

/// The overlay is rebuilt on every hover change. A sentence scrolled down
/// stands where it was in the rebuilt row, its bar up from the first frame
/// rather than a layout later.
#[test]
fn a_rebuilt_row_keeps_its_sentence_where_it_was_scrolled() {
    let mut app = aimed_entry(false);
    let (text_box, _) = sentence_box(&mut app);
    lay_out(&mut app, text_box, 200.0);
    app.update();
    app.world_mut()
        .get_mut::<ScrollPosition>(text_box)
        .unwrap()
        .y = 30.0;
    app.update();
    app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(30, 0));
    app.update();
    let (rebuilt, bar) = sentence_box(&mut app);
    assert_ne!(rebuilt, text_box, "the hover rebuilt the row");
    assert!((offset_of(&app, rebuilt) - 30.0).abs() < f32::EPSILON);
    assert_eq!(shown(&app, bar), Visibility::Inherited);
}

/// Writes to the box, its bar and the store, counted by a system of their
/// own so a test can hold them still.
#[derive(Resource, Default)]
struct Writes(usize);

fn count_writes(
    mut writes: ResMut<Writes>,
    bars: Query<(), (With<stack::StackTextBar>, Changed<Visibility>)>,
    boxes: Query<(), (With<stack::StackTextBox>, Changed<ScrollPosition>)>,
    scroll: Res<stack::StackTextScroll>,
) {
    writes.0 += bars.iter().count() + boxes.iter().count() + usize::from(scroll.is_changed());
}

/// At rest nothing is written: no bar, no box, no store — each would wake
/// the layout, or what watches it, every frame. The first half is the
/// counter-test: a scroll is counted.
#[test]
fn a_scrolled_sentence_at_rest_writes_nothing() {
    let mut app = aimed_entry(false);
    let (text_box, _) = sentence_box(&mut app);
    lay_out(&mut app, text_box, 200.0);
    app.init_resource::<Writes>()
        .add_systems(Update, count_writes.after(stack::stack_text));
    app.update();
    app.world_mut()
        .get_mut::<ScrollPosition>(text_box)
        .unwrap()
        .y = 12.0;
    app.update();
    app.update();
    assert!(
        app.world().resource::<Writes>().0 > 0,
        "a scroll is a write"
    );
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0, "an idle frame wrote");
}
