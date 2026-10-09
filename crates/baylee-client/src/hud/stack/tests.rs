use super::*;

#[test]
fn generic_cost_in_a_stack_sentence_uses_the_mana_font_entity() {
    let assets = Assets::<Font>::default();
    let fonts = UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: assets.reserve_handle(),
    };
    let mut app = App::new();
    let (_, text_box) = spawn_stack_sentence(
        &mut app.world_mut().commands(),
        &fonts,
        StackKey::Panel,
        ObjectId::new(1, 0),
        vec![TextBlock::Rules("{1}: Verhindere diesen Schaden.".into())],
        STACK_SENTENCE_LINES,
    );
    app.world_mut().flush();
    let sentence = app.world().get::<Children>(text_box).unwrap()[0];
    let first = app.world().get::<Children>(sentence).unwrap()[0];
    assert_eq!(app.world().get::<TextSpan>(first).unwrap().0, "\u{e606}");
    assert_eq!(
        app.world().get::<TextFont>(first).unwrap().font,
        bevy::text::FontSource::Handle(fonts.mana)
    );
}

#[test]
fn queued_heading_keeps_mana_and_tap_as_glyph_spans_before_truncation() {
    let spans = queued_heading_spans("{2}{R}, {T}: Deal 2 damage to any target.", 180.0, 13.0);
    assert_eq!(spans.iter().filter(|s| s.mark).count(), 3);
    assert!(
        spans
            .iter()
            .all(|s| !s.text.contains('{') && !s.text.contains('}'))
    );
    assert!(spans.iter().map(|s| s.text.chars().count()).sum::<usize>() <= budget(180.0, 13.0));
    assert_eq!(
        queued_heading_spans("Æther Vial", 180.0, 13.0)[0].text,
        "Æther Vial"
    );
}

/// A sentence's box is four lines tall on a desktop and two on a phone's
/// short window, the shell's own line between them.
#[test]
fn a_phone_s_short_window_shows_two_lines_of_a_sentence() {
    let phone = crate::shellkit::size::PHONE_HEIGHT;
    assert!((text_lines(phone - 1.0) - STACK_SENTENCE_LINES_PHONE).abs() < f32::EPSILON);
    assert!((text_lines(phone) - STACK_SENTENCE_LINES).abs() < f32::EPSILON);
}

/// A name that fits is left exactly as printed — the common case, and the
/// one where a stray ellipsis would be a lie about the card.
#[test]
fn a_short_name_is_not_cut() {
    assert_eq!(fit("Shock", 187.0, 15.0), "Shock");
}

/// The table waits for *you*, not for a pronoun in the wrong case.
///
/// Seen live as "wartet auf You" and, once the offline seat was named in
/// German, one slot away from "wartet auf Du". An opponent is still named,
/// because a name is what a name slot is for.
#[test]
fn the_table_waits_for_you_rather_than_for_your_name() {
    assert_eq!(waiting_line(Lang::De, "Du", true), "wartet auf dich");
    assert_eq!(waiting_line(Lang::En, "You", true), "waiting for you");
    assert_eq!(
        waiting_line(Lang::De, "sharp 1", false),
        "wartet auf sharp 1"
    );
}

/// A long one is cut *and* stays inside the budget it was cut to. The
/// first attempt appended the ellipsis to a full-budget slice and came
/// out one character wider than the room it was given.
#[test]
fn a_long_name_is_cut_to_fit() {
    let room = 187.0;
    let size = 15.0;
    let cut = fit("Asmoranomardicadaistinaculdacar", room, size);
    assert!(cut.ends_with('…'), "cut without saying so: {cut}");
    let budget = (room / (size * CHAR_WIDTH)) as usize;
    assert!(
        cut.chars().count() <= budget,
        "{cut} is {} chars, over the {budget} it had",
        cut.chars().count()
    );
}

/// A sentence is cut at a word, and the report's own line is the case.
///
/// Fails against the old code, which cut `…auf den Klin…` — the fragment
/// #132 was written from. A name may be cut mid-word and is; prose may
/// not, which is why the two have different functions.
#[test]
fn a_sentence_is_cut_at_a_word() {
    let line = "kannst du eine +1/+1-Marke auf den Klingenmeister legen";
    let cut = cut_words(line, 34);
    assert_eq!(cut, "kannst du eine +1/+1-Marke auf…");
    assert!(
        cut.chars().count() <= 34,
        "{cut} is over the budget it was given"
    );
}

/// A word that does not fit at all is cut through rather than dropped.
///
/// The budget shrinks as spans are spent, so the last span of a sentence
/// can be handed four characters — and backing off to the previous word
/// boundary there means backing off to nothing. A bare `…` says less
/// than a cut word does, and the same holds for one long compound with
/// no space in it anywhere.
#[test]
fn a_word_longer_than_the_budget_falls_back_to_the_character_cut() {
    assert_eq!(
        cut_words("Verzauberungskreatur", 8),
        cut("Verzauberungskreatur", 8)
    );
    assert!(cut_words("Verzauberungskreatur", 8).chars().count() <= 8);
    // And the same when the first word alone overruns: there is a space
    // in the string, but none of it is inside the budget.
    assert_eq!(
        cut_words("Verzauberungskreatur legen", 8),
        cut("Verzauberungskreatur", 8)
    );
}

/// Prose that fits is left exactly as printed, ellipsis and all absent.
#[test]
fn a_sentence_that_fits_is_not_cut() {
    assert_eq!(cut_words("Ziehe eine Karte.", 40), "Ziehe eine Karte.");
    // Exactly at the budget is not over it.
    assert_eq!(cut_words("Ziehe eine Karte.", 17), "Ziehe eine Karte.");
}

/// The cut lands on character boundaries. A byte-wise slice of a name
/// with an accent in it panics, and the pool has several.
#[test]
fn a_name_that_is_not_ascii_survives_the_cut() {
    let cut = fit("Æther Vial of Márton Stromgald’s Æther", 60.0, 13.0);
    assert!(cut.chars().count() < 20, "{cut} was not cut at all");
}

/// Sheoldred's German dmu printing, read from the catalog 2026-09-24:
/// three printed **lines**, which is what `baylee_core::oracle::sentences`
/// splits on and what `StackText::line` indexes into — not three
/// sentences of prose.
fn sheoldred_in_german() -> crate::cardtext::CardTexts {
    crate::cardtext::CardTexts::filed(crate::cardtext::fixture::german(
        "Sheoldred, the Apocalypse",
        "Sheoldred die Apokalypse",
        Some(
            "Todesberührung\n\
                 Immer wenn du eine Karte ziehst, erhältst du 2 Lebenspunkte dazu.\n\
                 Immer wenn ein Gegner eine Karte zieht, verliert er 2 Lebenspunkte.",
        ),
    ))
}

/// One permanent, and one stack ability per entry of `lines` — `None`
/// for an ability the host sent no line index for.
fn a_stack_of(lines: &[Option<u8>]) -> (baylee_client_core::BoardModel, PlayerView) {
    use baylee_client_core::board::Openings;
    use baylee_client_core::test_support::{ViewBuilder, printed, token};

    let card = crate::cardtext::fixture::card("Sheoldred, the Apocalypse");
    let on_stack: Vec<_> = lines
        .iter()
        .enumerate()
        .map(|(at, line)| {
            let mut ability = token(30 + at as u32, 0, "Sheoldred", 0, 0);
            ability.card = None;
            ability.stack_item = Some(baylee_view::StackItem::Ability {
                token: None,
                source: ObjectId::new(7, 0),
                ability: None,
                rules: Some(baylee_view::RulesFace { card, face: 0 }),
                text: line.map(|line| baylee_view::StackText {
                    face: 0,
                    line,
                    of: 3,
                }),
            });
            ability
        })
        .collect();
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![crate::cardtext::fixture::showing(
                printed(7, 0, "Sheoldred", 7),
                card,
            )],
        )
        .with_stack(on_stack)
        .build();
    let board = baylee_client_core::BoardModel::from_view(
        &view,
        Openings::none(),
        &[],
        crate::cardart::registry(),
    );
    (board, view)
}

/// Two abilities of one permanent are two different rows in the queue.
///
/// The defect #132 was reported from, run: a stack of five where two
/// rows read `Sheoldred, the Apocalypse` and nothing told them apart.
/// The test carries its own evidence — [`crate::face::name_of`] is what
/// the queue used to head these rows with, and it is asserted to give
/// one string for both, so the old behaviour fails beside the new one
/// passing. It goes through [`heading`], the door the row calls, rather
/// than through the lookup underneath it.
#[test]
fn two_abilities_of_one_permanent_are_two_different_queued_rows() {
    let texts = sheoldred_in_german();
    let (board, view) = a_stack_of(&[Some(1), Some(2)]);
    let mode = crate::face::FaceMode::default();
    let settings = crate::settings::ClientSettings::default();
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: Some(&view),
        widths: crate::face::Widths::of(None),
    };
    assert_eq!(board.stack.len(), 2, "two abilities are on the stack");

    let named = |item: &baylee_client_core::board::StackItem| {
        view.object(item.id).map_or_else(
            || item.name.clone(),
            |o| crate::face::name_of(o, &view, faces.texts),
        )
    };
    let headings: Vec<String> = board
        .stack
        .iter()
        .map(|item| heading(item, false, named(item), &faces))
        .collect();

    assert_ne!(
        headings[0], headings[1],
        "two triggers of one permanent must not be the same row twice"
    );
    // On the set, not on the order: `BoardModel` walks the stack top
    // first, and which end that is has nothing to do with this claim.
    for clause in ["Immer wenn du", "Immer wenn ein Gegner"] {
        assert!(
            headings.iter().any(|h| h.starts_with(clause)),
            "a row is headed by its own clause: {headings:?}"
        );
    }

    // The old heading, run: one string for both rows.
    assert_eq!(
        named(&board.stack[0]),
        named(&board.stack[1]),
        "the name the queue used to draw cannot tell these two apart"
    );
    // And the full row keeps that name, because it carries the sentence
    // already and the name is the one thing it does not otherwise say.
    for item in &board.stack {
        assert_eq!(
            heading(item, true, named(item), &faces),
            named(item),
            "a full row is headed by its name"
        );
    }
}

#[test]
fn expanded_oracle_height_keeps_virtual_queue_offsets_correct() {
    let top = Some(ObjectId::new(30, 0));
    let body = StackBody {
        top,
        rows: 20,
        floor: STACK_FULL_HEIGHT,
    };
    let node = ComputedNode {
        content_size: Vec2::new(704.0, (900.0 + 19.0 * STACK_ROW_HEIGHT) * 2.0),
        inverse_scale_factor: 0.5,
        ..default()
    };
    let height = body.full_height(&node, top);
    assert!((height - 900.0).abs() < f32::EPSILON);
    assert_eq!(
        window_start(800.0, height),
        0,
        "scrolling Oracle cannot discard its row"
    );
    assert!((rows_height(0, 3, height) - 1064.0).abs() < f32::EPSILON);
    assert!((body.full_height(&node, None) - STACK_FULL_HEIGHT).abs() < f32::EPSILON);
}

#[test]
fn copied_vesuvan_upkeep_uses_complete_source_oracle_without_a_mapped_line() {
    use baylee_client_core::board::Openings;
    use baylee_client_core::test_support::{ViewBuilder, printed};
    let card = crate::cardtext::fixture::card("Vesuvan Doppelganger");
    let mut trigger = printed(30, 0, "Llanowar Elves", 1);
    trigger.rules = Some(baylee_view::RulesFace {
        card: crate::cardtext::fixture::card("Llanowar Elves"),
        face: 0,
    });
    trigger.stack_item = Some(baylee_view::StackItem::Ability {
        source: ObjectId::new(7, 0),
        ability: Some(baylee_core::ids::AbilityRef { card, index: 0 }),
        rules: Some(baylee_view::RulesFace { card, face: 0 }),
        text: None,
        token: None,
    });
    let view = ViewBuilder::new(2).with_stack(vec![trigger]).build();
    let board = baylee_client_core::BoardModel::from_view(
        &view,
        Openings::none(),
        &[],
        crate::cardart::registry(),
    );
    let texts = crate::cardtext::CardTexts::default();
    let mode = crate::face::FaceMode::default();
    let settings = crate::settings::ClientSettings::default();
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: Some(&view),
        widths: crate::face::Widths::of(None),
    };
    let expected =
        baylee_client_core::card_face::split_blocks(baylee_cards::oracle::face(card, 0).unwrap());
    assert_eq!(
        stack_sentence(&board.stack[0], &faces),
        Some(expected.clone())
    );
    let fonts = UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    };
    let mut app = App::new();
    let (_, text_box) = spawn_stack_sentence(
        &mut app.world_mut().commands(),
        &fonts,
        StackKey::Panel,
        board.stack[0].id,
        expected,
        STACK_SENTENCE_LINES,
    );
    app.world_mut().flush();
    let sentence = app.world().get::<Children>(text_box).unwrap()[0];
    let visible: String = app
        .world()
        .get::<Children>(sentence)
        .unwrap()
        .iter()
        .filter_map(|child| app.world().get::<TextSpan>(child))
        .map(|span| span.0.as_str())
        .collect();
    assert!(visible.contains("At the beginning of your upkeep"));
    assert!(visible.contains("and it has this ability."));
    assert!(!visible.contains('…'));
    let mut unknown = board.stack[0].clone();
    if let baylee_client_core::board::StackKind::Ability { ability, .. } = &mut unknown.kind {
        *ability = None;
    }
    assert!(stack_sentence(&unknown, &faces).is_none());
}

#[test]
fn a_food_ability_draws_its_sentence_without_a_source_or_card_text() {
    let (mut board, view) = a_stack_of(&[None]);
    let token = baylee_cards::tokens::token_id(&baylee_cards::tokens::FOOD);
    board.stack[0].kind = baylee_client_core::board::StackKind::Ability {
        source: ObjectId::new(999, 0),
        ability: None,
        text: None,
        rules: None,
        token: Some(baylee_view::TokenAbility { token, index: 0 }),
    };
    let texts = crate::cardtext::CardTexts::default();
    let mode = crate::face::FaceMode::default();
    let settings = crate::settings::ClientSettings::default();
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: Some(&view),
        widths: crate::face::Widths::of(None),
    };
    assert_eq!(
        queued_ability_line(&board.stack[0], &faces).as_deref(),
        Some("{2}, {T}, Sacrifice this token: You gain 3 life.")
    );
    if let baylee_client_core::board::StackKind::Ability {
        token: Some(ref mut t),
        ..
    } = board.stack[0].kind
    {
        t.index = 1;
    }
    assert!(
        stack_sentence(&board.stack[0], &faces).is_none(),
        "never substitute another ability's sentence"
    );
}

/// A row never comes out blank because a lookup missed.
///
/// The host sends no line index for some abilities, and the catalog's
/// text arrives over a socket a client playing the house offline may not
/// have at all — so the heading falls back to the name.
#[test]
fn an_ability_with_no_sentence_keeps_the_name() {
    let texts = crate::cardtext::CardTexts::default();
    let (board, view) = a_stack_of(&[None]);
    let mode = crate::face::FaceMode::default();
    let settings = crate::settings::ClientSettings::default();
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: Some(&view),
        widths: crate::face::Widths::of(None),
    };
    assert!(queued_ability_line(&board.stack[0], &faces).is_none());
    assert_eq!(
        heading(&board.stack[0], false, "Sheoldred".to_string(), &faces),
        "Sheoldred"
    );
}

/// One Sheoldred trigger on the stack whose line the host counted in
/// other card text: line 0 of four, where this build prints her face in
/// three.
fn a_skewed_trigger(
    ability: Option<baylee_core::ids::AbilityRef>,
) -> (baylee_client_core::BoardModel, PlayerView) {
    use baylee_client_core::board::Openings;
    use baylee_client_core::test_support::{ViewBuilder, printed, token};

    let card = crate::cardtext::fixture::card("Sheoldred, the Apocalypse");
    let mut trigger = token(30, 0, "Sheoldred", 0, 0);
    trigger.card = None;
    trigger.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: ObjectId::new(7, 0),
        ability,
        rules: Some(baylee_view::RulesFace { card, face: 0 }),
        text: Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 4,
        }),
    });
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![crate::cardtext::fixture::showing(
                printed(7, 0, "Sheoldred", 7),
                card,
            )],
        )
        .with_stack(vec![trigger])
        .build();
    let board = baylee_client_core::BoardModel::from_view(
        &view,
        Openings::none(),
        &[],
        crate::cardart::registry(),
    );
    (board, view)
}

/// A host built against other card text sends a line this build would
/// read as the neighbouring sentence. The entry's `AbilityRef` places the
/// trigger in this build's own line table instead, so the row reads the
/// sentence the ability is — in German here, since that pairs — and not
/// its source's name, nor Deathtouch, which is what line 0 is here.
#[test]
fn a_line_counted_in_other_text_is_placed_by_the_ability_instead() {
    let card = crate::cardtext::fixture::card("Sheoldred, the Apocalypse");
    let index = (0..16)
        .find(|&i| baylee_cards::lines::ability_line(card, 0, i).is_some_and(|l| l.line == 2))
        .expect("the opponent-draw trigger has a line");
    let texts = sheoldred_in_german();
    let (board, view) = a_skewed_trigger(Some(baylee_core::ids::AbilityRef::new(card, index)));
    let mode = crate::face::FaceMode::default();
    let settings = crate::settings::ClientSettings::default();
    let faces = FaceCtx {
        texts: &texts,
        mode: &mode,
        settings: &settings,
        view: Some(&view),
        widths: crate::face::Widths::of(None),
    };
    assert_eq!(
        queued_ability_line(&board.stack[0], &faces).as_deref(),
        Some("Immer wenn ein Gegner eine Karte zieht, verliert er 2 Lebenspunkte.")
    );

    // The counter-test: without the handle there is nothing to place it
    // by, and the row keeps its source's name rather than a guess.
    let (board, view) = a_skewed_trigger(None);
    let faces = FaceCtx {
        view: Some(&view),
        ..faces
    };
    assert!(queued_ability_line(&board.stack[0], &faces).is_none());
    assert_eq!(
        heading(&board.stack[0], false, "Sheoldred".to_string(), &faces),
        "Sheoldred"
    );
}

/// A reminder is not what the one line is spent on, and the runs it is
/// cut out of do not leave a double space behind.
#[test]
fn a_queued_line_drops_the_reminder_and_the_gap_it_left() {
    let blocks = vec![
        TextBlock::Rules("Fliegend".to_string()),
        TextBlock::Reminder("kann nur von Kreaturen geblockt werden".to_string()),
        TextBlock::Rules("und Wachsamkeit".to_string()),
    ];
    assert_eq!(
        ability_line(&blocks).as_deref(),
        Some("Fliegend und Wachsamkeit")
    );
    assert_eq!(
        ability_line(&[TextBlock::Reminder("nur dies".to_string())]),
        None,
        "a sentence that is nothing but a reminder heads no row"
    );
}

/// A queued row is the same object as the full row it will be promoted
/// to, and a *different* key — which is the whole reason a resolution
/// eases instead of cutting.
#[test]
fn promotion_is_a_new_key() {
    let id = ObjectId::new(7, 0);
    assert_ne!(StackKey::Entry(id, false), StackKey::Entry(id, true));
    assert_eq!(StackKey::Entry(id, true), StackKey::Entry(id, true));
}

/// Progress is remembered per row and forgotten for a row that is not on
/// screen, so a rebuilt panel re-attaches to the arrival in progress and
/// an object that resolved leaves nothing behind.
#[test]
fn a_row_that_is_not_tracked_is_simply_drawn() {
    let mut motion = StackMotion::default();
    let id = ObjectId::new(3, 0);
    assert!((motion.progress(StackKey::Entry(id, true)) - 1.0).abs() < f32::EPSILON);
    motion.rows.push(Rise {
        key: StackKey::Entry(id, true),
        at: 0.4,
        promoted: false,
        cooled: 1.0,
    });
    assert!((motion.progress(StackKey::Entry(id, true)) - 0.4).abs() < f32::EPSILON);
    assert!((motion.progress(StackKey::Panel) - 1.0).abs() < f32::EPSILON);
}

/// A veil is the inverse of ink: opaque while the row arrives, gone once
/// it has. Without that the card picture — which is on a shared material
/// and cannot be faded itself — would pop in at full strength.
#[test]
fn a_veil_clears_as_the_ink_comes_up() {
    let key = StackKey::Panel;
    let ink = Arriving::ink(key, 0.88);
    let veil = Arriving::veil(key);
    assert!(ink.alpha(0.0).abs() < f32::EPSILON);
    assert!((veil.alpha(0.0) - 1.0).abs() < f32::EPSILON);
    assert!((ink.alpha(1.0) - 0.88).abs() < f32::EPSILON);
    assert!(veil.alpha(1.0).abs() < f32::EPSILON);
}

// ---- the system, actually run ---------------------------------------
//
// The arithmetic above is the easy half. "Declared but never wired" is a
// bug this client has shipped before, so the rest of these run the system
// in an `App` and assert on what a frame left behind.

use crate::prefs::Prefs;

/// An app with the system in it and nothing else.
fn harness() -> App {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<Prefs>()
        .init_resource::<StackMotion>()
        .add_systems(Update, ease_the_stack_in);
    app
}

/// One row: a fill that fades and a transform that lifts, plus a line of
/// text under it, which is the part no opacity inheritance would reach.
fn a_row(app: &mut App, key: StackKey) -> (Entity, Entity) {
    let row = app
        .world_mut()
        .spawn((
            Node::default(),
            BackgroundColor(palette::DIALOG_LIT),
            Arriving::fill(key, palette::DIALOG_LIT.alpha()),
            ArrivingRow {
                lift: ARRIVE_LIFT,
                from: ARRIVE_SCALE,
                rail: palette::CANDLE,
            },
        ))
        .id();
    let ink = app
        .world_mut()
        .spawn((
            Node::default(),
            TextColor(palette::INK),
            Arriving::ink(key, palette::INK.alpha()),
        ))
        .id();
    (row, ink)
}

/// A sixtieth of a second, the frame this client is tuned against.
fn a_frame(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
    app.update();
}

fn alpha_of(app: &App, row: Entity) -> f32 {
    app.world()
        .entity(row)
        .get::<BackgroundColor>()
        .unwrap()
        .0
        .alpha()
}

fn lift_of(app: &App, row: Entity) -> f32 {
    match app
        .world()
        .entity(row)
        .get::<UiTransform>()
        .unwrap()
        .translation
        .y
    {
        Val::Px(y) => y,
        other => panic!("the lift is not in pixels: {other:?}"),
    }
}

/// A row comes up from nothing, over several frames, and then holds
/// exactly where it belongs. Both ends matter: a fade that never finished
/// would leave the panel permanently dim.
#[test]
fn a_row_arrives_and_then_holds_still() {
    let mut app = harness();
    let key = StackKey::Entry(ObjectId::new(1, 0), true);
    let (row, ink) = a_row(&mut app, key);

    a_frame(&mut app);
    let part = alpha_of(&app, row);
    assert!(
        part > 0.0 && part < palette::DIALOG_LIT.alpha(),
        "one frame in, the row is part way: {part}"
    );
    assert!(lift_of(&app, row) < -0.5, "and still above its place");
    let text = app
        .world()
        .entity(ink)
        .get::<TextColor>()
        .unwrap()
        .0
        .alpha();
    assert!(
        text > 0.0 && text < 1.0,
        "the text fades with it, having no inheritance to ride: {text}"
    );

    for _ in 0..40 {
        a_frame(&mut app);
    }
    assert!(
        (alpha_of(&app, row) - palette::DIALOG_LIT.alpha()).abs() < 1e-4,
        "the row lands at the alpha it was drawn in"
    );
    assert!(lift_of(&app, row).abs() < 1e-4, "and at its resting place");
    let scale = app.world().entity(row).get::<UiTransform>().unwrap().scale;
    assert!((scale.x - 1.0).abs() < 1e-4, "at full size: {scale:?}");
}

/// The whole reason the progress is a resource: the HUD is rebuilt on
/// hover, and a row that restarted its fade every time the pointer moved
/// would flicker instead of arriving.
#[test]
fn a_rebuilt_row_carries_on_where_it_was() {
    let mut app = harness();
    let key = StackKey::Entry(ObjectId::new(2, 0), false);
    let (row, _) = a_row(&mut app, key);
    for _ in 0..4 {
        a_frame(&mut app);
    }
    let midway = alpha_of(&app, row);
    assert!(midway > 0.1, "far enough in to tell a restart from a fade");

    // What a `HudRevision` change does: the whole subtree goes and is
    // built again from scratch, at rest, with the same key.
    app.world_mut().entity_mut(row).despawn();
    let (again, _) = a_row(&mut app, key);
    a_frame(&mut app);
    assert!(
        alpha_of(&app, again) > midway,
        "the rebuild continued the arrival rather than restarting it"
    );
}

/// A row that steps down holds still.
///
/// This is the headline case — a spell lands on a stack that already had
/// one — and the key carries the row's shape, so the object that was on
/// top is under a *new* key the moment it is drawn queued. Seeded like an
/// arrival it would fade in beside the newcomer and the player would see
/// two spells land where one did.
#[test]
fn a_row_that_steps_down_does_not_announce_itself() {
    let mut app = harness();
    let old = ObjectId::new(8, 0);
    let (top, top_ink) = a_row(&mut app, StackKey::Entry(old, true));
    for _ in 0..40 {
        a_frame(&mut app);
    }

    // A spell lands: the panel is rebuilt, the old top in the queued
    // shape and the newcomer full above it.
    app.world_mut().entity_mut(top).despawn();
    app.world_mut().entity_mut(top_ink).despawn();
    let (stepped, _) = a_row(&mut app, StackKey::Entry(old, false));
    let (landed, _) = a_row(&mut app, StackKey::Entry(ObjectId::new(9, 0), true));
    a_frame(&mut app);

    assert!(
        (alpha_of(&app, stepped) - palette::DIALOG_LIT.alpha()).abs() < 1e-4,
        "the demoted row stood where it was"
    );
    assert!(lift_of(&app, stepped).abs() < 1e-4, "and did not drop in");
    let arriving = alpha_of(&app, landed);
    assert!(
        arriving > 0.0 && arriving < palette::DIALOG_LIT.alpha(),
        "while the spell that did land is still arriving: {arriving}"
    );
}

/// And it is carried at the progress it had, not at rest. The house AI
/// answers within a frame or two of priority, so the common demotion is
/// of a row that is still arriving; seeding that at 1.0 would snap a
/// half-faded spell to full on the frame the counter landed.
#[test]
fn a_row_demoted_mid_arrival_keeps_its_place_in_the_fade() {
    let mut app = harness();
    let old = ObjectId::new(11, 0);
    let (top, top_ink) = a_row(&mut app, StackKey::Entry(old, true));
    a_frame(&mut app);
    a_frame(&mut app);
    let partway = alpha_of(&app, top);
    assert!(
        partway > 0.0 && partway < palette::DIALOG_LIT.alpha() * 0.9,
        "the row under test has to still be arriving: {partway}"
    );

    // The answer lands before the first spell has finished arriving.
    app.world_mut().entity_mut(top).despawn();
    app.world_mut().entity_mut(top_ink).despawn();
    let (stepped, _) = a_row(&mut app, StackKey::Entry(old, false));
    a_row(&mut app, StackKey::Entry(ObjectId::new(12, 0), true));
    a_frame(&mut app);

    let carried = alpha_of(&app, stepped);
    assert!(
        carried > partway && carried < palette::DIALOG_LIT.alpha() * 0.95,
        "it continues from {partway}, it does not jump to full: {carried}"
    );
}

/// The other direction is not carried, and that is the point: a queued
/// row becoming full is what a resolution looks like from the panel.
#[test]
fn a_row_that_is_promoted_still_arrives() {
    let mut app = harness();
    let id = ObjectId::new(10, 0);
    let (queued, queued_ink) = a_row(&mut app, StackKey::Entry(id, false));
    for _ in 0..40 {
        a_frame(&mut app);
    }
    app.world_mut().entity_mut(queued).despawn();
    app.world_mut().entity_mut(queued_ink).despawn();
    let (full, _) = a_row(&mut app, StackKey::Entry(id, true));
    a_frame(&mut app);
    let part = alpha_of(&app, full);
    assert!(
        part > 0.0 && part < palette::DIALOG_LIT.alpha(),
        "a resolution is meant to be seen: {part}"
    );
}

/// And it arrives from the **other direction**, which is the difference
/// between a spell being cast and a spell resolving.
///
/// A promoted row grew out of the slot below it: the object above it has
/// resolved and left. Drawn as an arrival it lifted into place from
/// above — the movement of something landing on the stack, which is the
/// opposite of what happened. This is the one assertion that can tell the
/// two apart, because the alpha ramp is identical for both.
#[test]
fn a_promoted_row_comes_up_from_the_slot_it_was_in() {
    let mut app = harness();
    let id = ObjectId::new(14, 0);
    let (queued, queued_ink) = a_row(&mut app, StackKey::Entry(id, false));
    for _ in 0..40 {
        a_frame(&mut app);
    }
    app.world_mut().entity_mut(queued).despawn();
    app.world_mut().entity_mut(queued_ink).despawn();
    let (full, _) = a_row(&mut app, StackKey::Entry(id, true));
    a_frame(&mut app);

    let lift = lift_of(&app, full);
    assert!(
        lift > 0.0 && lift <= PROMOTE_LIFT,
        "a promotion starts below its place and rises: {lift}"
    );

    // The counter-test, and the reason the sign is worth asserting at
    // all: a spell that is merely *cast* still drops in from above.
    let (landed, _) = a_row(&mut app, StackKey::Entry(ObjectId::new(15, 0), true));
    a_frame(&mut app);
    assert!(
        lift_of(&app, landed) < 0.0,
        "an arrival comes from the other side"
    );
}

/// The promoted row's rail lands bright and cools into the accent.
///
/// The light was on the row that resolved a moment ago; a bright mark
/// appearing one slot lower and settling is the resolution drawn as the
/// movement it is. It is on the slower ramp deliberately, so it is still
/// settling after the row has stopped moving — which is what this asserts
/// by reading the border on the frame the movement is nearly over.
#[test]
fn the_rail_of_a_promoted_row_cools_into_place() {
    let mut app = harness();
    let id = ObjectId::new(16, 0);
    let (queued, queued_ink) = a_row(&mut app, StackKey::Entry(id, false));
    for _ in 0..40 {
        a_frame(&mut app);
    }
    app.world_mut().entity_mut(queued).despawn();
    app.world_mut().entity_mut(queued_ink).despawn();
    let (full, _) = a_row(&mut app, StackKey::Entry(id, true));

    let mut seen_warmer = false;
    for _ in 0..12 {
        a_frame(&mut app);
        let rail = app
            .world()
            .entity(full)
            .get::<BorderColor>()
            .expect("a Node always has one")
            .top
            .to_srgba();
        // Whiter than the accent it settles at: `INK` is brighter in
        // every channel; green carries the contrast against warm gold.
        if rail.green > palette::CANDLE.to_srgba().green * 1.1 {
            seen_warmer = true;
        }
    }
    assert!(seen_warmer, "the rail landed brighter than it rests");

    for _ in 0..60 {
        a_frame(&mut app);
    }
    let settled = app
        .world()
        .entity(full)
        .get::<BorderColor>()
        .expect("a Node always has one")
        .top
        .to_srgba();
    let accent = palette::CANDLE.to_srgba();
    assert!(
        (settled.red - accent.red).abs() < 0.02
            && (settled.green - accent.green).abs() < 0.02
            && (settled.blue - accent.blue).abs() < 0.02,
        "and cooled all the way to the accent: {settled:?}"
    );
}

/// A player who has asked for stillness gets the panel, not the arrival.
#[test]
fn holding_still_puts_the_row_straight_where_it_belongs() {
    let mut app = harness();
    app.world_mut().resource_mut::<Prefs>().edit().reduce_motion = true;
    let key = StackKey::Entry(ObjectId::new(3, 0), true);
    let (row, _) = a_row(&mut app, key);
    a_frame(&mut app);
    assert!(
        (alpha_of(&app, row) - palette::DIALOG_LIT.alpha()).abs() < 1e-4,
        "there on the first frame"
    );
    assert!(lift_of(&app, row).abs() < 1e-4);
}

/// A line of text keeps the background it does not have.
///
/// [`Node`] requires a [`BackgroundColor`], so every label in this panel
/// carries a transparent one; a fade that wrote whichever colour it found
/// turned each of those into an opaque black plate, and the first live
/// shot of the panel was eight of them where the words should be. The
/// [`Paints`] tag is what stops it, and this is the test that would have
/// caught it — the arithmetic tests all passed while the panel was
/// unreadable.
#[test]
fn fading_a_label_does_not_give_it_a_plate() {
    let mut app = harness();
    let key = StackKey::Entry(ObjectId::new(5, 0), true);
    let (_, ink) = a_row(&mut app, key);
    for _ in 0..3 {
        a_frame(&mut app);
    }
    let plate = app
        .world()
        .entity(ink)
        .get::<BackgroundColor>()
        .expect("a Node always has one")
        .0;
    assert!(
        plate.alpha().abs() < f32::EPSILON,
        "the label grew a background: {plate:?}"
    );
    assert!(
        app.world()
            .entity(ink)
            .get::<TextColor>()
            .unwrap()
            .0
            .alpha()
            > 0.0,
        "while its ink did come up"
    );
}

/// The accent rail arrives with the row rather than standing there alone
/// while the row fades in behind it — and a row with no rail never grows
/// one, which is the same `Color::NONE` trap one component along.
#[test]
fn the_rail_arrives_with_its_row() {
    let mut app = harness();
    let key = StackKey::Entry(ObjectId::new(6, 0), true);
    let (row, _) = a_row(&mut app, key);
    a_frame(&mut app);
    let part = app.world().entity(row).get::<BorderColor>().unwrap().left;
    assert!(
        part.alpha() > 0.0 && part.alpha() < 1.0,
        "the rail comes up with the row: {part:?}"
    );
    for _ in 0..40 {
        a_frame(&mut app);
    }
    let rested = app.world().entity(row).get::<BorderColor>().unwrap().left;
    assert!((rested.alpha() - 1.0).abs() < 1e-4, "and lands lit");
}

/// A resolved spell leaves nothing behind. Without the prune, the *next*
/// object at that id and shape would find its arrival already finished
/// and appear with no animation at all.
#[test]
fn a_row_that_leaves_the_panel_is_forgotten() {
    let mut app = harness();
    let key = StackKey::Entry(ObjectId::new(4, 0), true);
    let (row, ink) = a_row(&mut app, key);
    a_frame(&mut app);
    assert_eq!(app.world().resource::<StackMotion>().rows.len(), 1);

    app.world_mut().entity_mut(row).despawn();
    app.world_mut().entity_mut(ink).despawn();
    a_frame(&mut app);
    assert!(
        app.world().resource::<StackMotion>().rows.is_empty(),
        "the panel emptied and took its progress with it"
    );
}
