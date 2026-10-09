use super::*;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SubtypeSet, TypeSet};

/// The shipped Regular cut, read the way the client reads it.
fn regular() -> Font {
    let path = format!(
        "{}/assets/fonts/AlegreyaSans-Regular.ttf",
        env!("CARGO_MANIFEST_DIR")
    );
    Font::from_bytes(std::fs::read(path).expect("the bundled Regular"))
}

/// A handle for each face, all different, so a test can tell which one a
/// line is set in.
fn test_fonts() -> UiFonts {
    use bevy::asset::uuid_handle;
    UiFonts {
        text: uuid_handle!("b259f0c1-0000-4000-8000-000000000001"),
        medium: uuid_handle!("b259f0c1-0000-4000-8000-000000000002"),
        bold: uuid_handle!("b259f0c1-0000-4000-8000-000000000003"),
        italic: uuid_handle!("b259f0c1-0000-4000-8000-000000000004"),
        medium_italic: uuid_handle!("b259f0c1-0000-4000-8000-000000000005"),
        serif: uuid_handle!("b259f0c1-0000-4000-8000-000000000006"),
        serif_italic: uuid_handle!("b259f0c1-0000-4000-8000-000000000007"),
        icons: uuid_handle!("b259f0c1-0000-4000-8000-000000000008"),
        mana: uuid_handle!("b259f0c1-0000-4000-8000-000000000009"),
    }
}

/// A creature face with this name and type line, costing {1}{G}.
fn creature(name: &str, type_line: &str) -> CardFace {
    CardFace {
        name: name.to_owned(),
        cost: vec![ManaSymbol::Generic(1), ManaSymbol::Green],
        type_line: type_line.to_owned(),
        body: Vec::new(),
        stats: Some(Stats::PowerToughness {
            power: 2,
            toughness: 2,
            damage: 0,
        }),
        colors: ColorSet::EMPTY,
        types: TypeSet::CREATURE,
        subtypes: SubtypeSet::EMPTY,
        text_pending: false,
        credit: None,
    }
}

/// The widths are the shipped font's own advances, summed: read off the
/// file's `hmtx` for these strings, "Llanowar Elves" is 5.739 em, the
/// German "Llanowarelfen" 5.579, and the ellipsis a cut line ends on
/// 0.581. A character the font lacks is a full em; before the font, the
/// average answers.
#[test]
fn the_widths_are_the_shipped_font_s_own() {
    let font = regular();
    let widths = Widths::of(Some(&font));
    assert!(widths.measured());
    for (text, em) in [
        ("Llanowar Elves", 5.739),
        ("Llanowarelfen", 5.579),
        ("…", 0.581),
        ("\u{6f22}", 1.0),
    ] {
        let got = widths.width(text);
        assert!((got - em).abs() < 1e-3, "{text:?} measures {got}, not {em}");
    }
    let average = Widths::of(None);
    assert!(!average.measured());
    assert!(
        (average.width("Llanowar Elves") - textface::average_width("Llanowar Elves")).abs() < 1e-6
    );
}

/// The average is a stand-in, and on a real name it answers the
/// one-line-or-two question differently from the font — which is why a
/// face fitted by it is fitted again when the font arrives.
#[test]
fn the_average_and_the_font_can_disagree_about_a_name_s_lines() {
    use textface::fit_name;
    let font = regular();
    let widths = Widths::of(Some(&font));
    let name = "Abandoned Campground";
    let guessed = fit_name(name, textface::average_width);
    let measured = fit_name(name, |s| widths.width(s));
    assert_eq!(guessed.lines.len(), 1);
    assert_eq!(measured.lines.len(), 2, "{measured:?}");
}

/// The table's face writes the first sentence of its rules in its text
/// box (WP6): at most two lines, the second cut at a word, its symbols
/// in the Mana font, standing where the text box begins and ending
/// inside it.
#[test]
fn the_table_face_writes_its_first_sentence_in_its_text_box() {
    use bevy::ecs::world::CommandQueue;
    use bevy::sprite::Anchor;

    let font = regular();
    let widths = Widths::of(Some(&font));
    let mut face = creature("Llanowar Elves", "Creature — Elf Druid");
    face.body = vec![
        TextBlock::Rules(
            "{T}: Draw a card for each Elf you control, then discard two cards at random \
                 from your hand. Then untap it."
                .to_owned(),
        ),
        TextBlock::Reminder("Not this.".to_owned()),
    ];
    let fit = WorldFit::of(&face, &widths);
    assert_eq!(
        fit.sentence.len(),
        textface::SENTENCE_LINES,
        "{:?}",
        fit.sentence
    );
    assert!(
        fit.sentence[0].starts_with("{T}: Draw"),
        "{:?}",
        fit.sentence
    );
    let last = fit.sentence.last().expect("a line");
    assert!(last.ends_with(textface::ELLIPSIS), "cut at a word: {last}");
    assert!(
        !fit.sentence.concat().contains("untap"),
        "past the first sentence"
    );
    for line in &fit.sentence {
        assert!(
            widths.marked(line) * textface::SENTENCE_EM <= textface::line_width() + 1e-6,
            "{line} runs past the box"
        );
    }

    let mut world = World::new();
    let card = world.spawn_empty().id();
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let word = textface::face_word(
        ColorSet::from_slice(&[MagicColor::Green]),
        TypeSet::CREATURE,
        SubtypeSet::EMPTY,
        textface::Depths::table(fit.lines()),
    );
    let plate = Plate::Fight {
        power: 2,
        toughness: 2,
        damage: 0,
    };
    let texts = spawn_world(&mut commands, card, &face, &fit, word, plate, &test_fonts());
    queue.apply(&mut world);
    let root = texts
        .iter()
        .copied()
        .find(|&e| {
            world
                .entity(e)
                .get::<Text2d>()
                .is_some_and(|t| t.0.is_empty())
        })
        .expect("the sentence's root");
    let spans: Vec<String> = world
        .entity(root)
        .get::<Children>()
        .expect("its spans")
        .iter()
        .map(|e| world.entity(e).get::<TextSpan>().expect("a span").0.clone())
        .collect();
    assert!(spans.concat().contains("Draw a card"), "{spans:?}");
    let tap = baylee_client_core::manapip::symbol("T").expect("the tap symbol");
    let baylee_client_core::manapip::Pip::Solid { glyph, .. } = tap else {
        panic!("{tap:?}")
    };
    assert_eq!(spans[0], glyph.to_string(), "the symbol in the Mana font");
    let at = world
        .entity(root)
        .get::<Transform>()
        .expect("a place")
        .translation;
    let y = (baylee_client_core::layout::CARD_HEIGHT * 0.5 - at.y) / crate::table::DOWN_THE_CARD;
    let regions = Regions::table(fit.lines());
    assert_eq!(
        *world.entity(root).get::<Anchor>().expect("an anchor"),
        Anchor::TOP_LEFT
    );
    assert!(
        (y - regions.text_box[1] - BAR_PAD).abs() < 1e-4,
        "the sentence stands at {y}, the text box at {:?}",
        regions.text_box
    );
    #[allow(clippy::cast_precision_loss)]
    let deep = fit.sentence.len() as f32 * LINE_BOX * textface::SENTENCE_EM;
    assert!(y + deep <= regions.text_box[3], "it runs out of its box");
}

/// Each line of the table's face stands inside its part of the card:
/// the name in the name bar, the cost on the art box's first line, the
/// type line in the type bar, the colour's symbol on its disc. A long
/// name takes two lines and the name bar its two-line height.
#[test]
fn the_table_face_stands_in_its_bars() {
    use bevy::ecs::world::CommandQueue;
    use bevy::sprite::Anchor;

    let font = regular();
    let widths = Widths::of(Some(&font));
    // The word is the material's: a green card's art box is light enough
    // for the dark ink under its cost, a black one's only for the light.
    for (name, lines, color, cost_ink) in [
        ("Llanowar Elves", 1, MagicColor::Green, textface::INK),
        (
            "Okina, Temple to the Grandfathers",
            2,
            MagicColor::Black,
            textface::LIGHT_INK,
        ),
    ] {
        let mut world = World::new();
        let card = world.spawn_empty().id();
        let face = creature(name, "Legendary Creature — Elf Druid Warrior");
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let fit = WorldFit::of(&face, &widths);
        let word = textface::face_word(
            ColorSet::from_slice(&[color]),
            TypeSet::CREATURE,
            SubtypeSet::EMPTY,
            textface::Depths::table(fit.lines()),
        );
        let texts = spawn_world(
            &mut commands,
            card,
            &face,
            &fit,
            word,
            // What the ledge shows for this 2/2.
            Plate::Fight {
                power: 2,
                toughness: 2,
                damage: 0,
            },
            &test_fonts(),
        );
        queue.apply(&mut world);
        assert_eq!(fit.lines(), lines, "{name}");
        let regions = Regions::table(lines);

        // Each text's box on the card, in card widths with y down.
        let boxes: Vec<(String, [f32; 4])> = texts
            .iter()
            .map(|&text| {
                let entity = world.entity(text);
                let words = entity.get::<Text2d>().expect("a Text2d").0.clone();
                let em = match entity.get::<TextFont>().expect("a font").font_size {
                    bevy::text::FontSize::Px(px) => px / PX_PER_UNIT,
                    other => panic!("{other:?}"),
                };
                let at = entity.get::<Transform>().expect("a place").translation;
                let anchor = entity.get::<Anchor>().expect("an anchor").as_vec();
                let rows: Vec<&str> = words.split('\n').collect();
                let w = rows.iter().map(|r| widths.width(r)).fold(0.0, f32::max) * em;
                #[allow(clippy::cast_precision_loss)]
                let h = rows.len() as f32 * LINE_BOX * em;
                // Back from the card's space to card widths from its
                // top-left, then from the anchor to the box's corner.
                let x = at.x / baylee_client_core::layout::CARD_WIDTH + 0.5;
                let y = (baylee_client_core::layout::CARD_HEIGHT * 0.5 - at.y)
                    / crate::table::DOWN_THE_CARD;
                let x0 = x - (anchor.x + 0.5) * w;
                let y0 = y - (0.5 - anchor.y) * h;
                (words, [x0, y0, x0 + w, y0 + h])
            })
            .collect();

        let within = |label: &str, part: [f32; 4]| {
            let (words, b) = boxes
                .iter()
                .find(|(words, _)| words.contains(label))
                .unwrap_or_else(|| panic!("{name}: no text holding {label:?}"));
            assert!(
                b[0] >= part[0] - 1e-4
                    && b[1] >= part[1] - 1e-4
                    && b[2] <= part[2] + 1e-4
                    && b[3] <= part[3] + 1e-4,
                "{name}: {words:?} at {b:?} leaves {part:?}"
            );
        };
        within(name.split(' ').next().expect("a word"), regions.name_bar);
        within("1 G", regions.band);
        // Whatever the type line was fitted to — here its subtypes alone.
        within(&fit.kind.lines[0], regions.type_bar);
        // The colour's symbol on its disc, in the art box (WP6).
        let Some(baylee_client_core::manapip::Pip::Solid { glyph, .. }) =
            Some(baylee_client_core::manapip::of_color(color))
        else {
            unreachable!("a colour is one glyph")
        };
        within(&glyph.to_string(), regions.band);
        // The plate says the body, so the face does not; and there are
        // no rules to give a sentence.
        assert_eq!(boxes.len(), 4, "{name}: {boxes:?}");

        // Dark on the light bars; the cost in the ink its art box lets it.
        for &text in &texts {
            let entity = world.entity(text);
            let words = &entity.get::<Text2d>().expect("a Text2d").0;
            let ink = entity.get::<TextColor>().expect("an ink").0;
            let want = if words.contains("1 G") {
                Color::srgb_from_array(cost_ink)
            } else {
                FACE_INKS.0
            };
            assert_eq!(ink, want, "{name}: {words:?}");
        }
    }
}

/// A face laid out for the overlay and spawned into a world, with its
/// nodes by what they hold.
fn overlay_face(face: &CardFace, width: f32, detail: Detail) -> (World, UiFace, Entity) {
    use bevy::ecs::world::CommandQueue;
    let font = regular();
    let widths = Widths::of(Some(&font));
    let laid = UiFace::lay(face, Lang::En, width, detail, &widths, 0);
    let mut world = World::new();
    let card = world.spawn(Node::default()).id();
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    spawn_ui(&mut commands, card, Lang::En, face, &laid, &test_fonts());
    queue.apply(&mut world);
    (world, laid, card)
}

/// Every node under `root`, depth first.
fn nodes_under(world: &World, root: Entity) -> Vec<Entity> {
    let mut all = Vec::new();
    let mut open = vec![root];
    while let Some(entity) = open.pop() {
        if let Some(children) = world.entity(entity).get::<Children>() {
            for &child in children {
                all.push(child);
                open.push(child);
            }
        }
    }
    all
}

/// Where a node's top-left stands, in card widths.
fn corner_of(node: &Node, width: f32) -> Vec2 {
    let px = |v: Val| match v {
        Val::Px(px) => px / width,
        other => panic!("{other:?}"),
    };
    Vec2::new(px(node.left), px(node.top))
}

/// The overlay's face stands where `textface` put it (#259): the name in
/// the name bar with the cost at its right end, the type line in the type
/// bar, the rules text in the text box with its scrollbar hidden, and
/// no node the pointer could take from the card around it.
#[test]
fn the_overlay_face_stands_in_its_bars() {
    let mut face = creature("Llanowar Elves", "Creature — Elf Druid");
    face.body = vec![TextBlock::Rules("{T}: Add {G}.".to_owned())];
    let width = 308.0;
    let (world, laid, card) = overlay_face(&face, width, Detail::Full);
    let inside =
        |at: Vec2, [x0, y0, x1, y1]: [f32; 4]| at.x >= x0 && at.x <= x1 && at.y >= y0 && at.y <= y1;
    let nodes = nodes_under(&world, card);
    let text_of = |words: &str| {
        nodes
            .iter()
            .copied()
            .find(|&e| world.entity(e).get::<Text>().is_some_and(|t| t.0 == words))
            .unwrap_or_else(|| panic!("no {words:?}"))
    };
    let node = |e: Entity| world.entity(e).get::<Node>().expect("a node");

    let name = corner_of(node(text_of("Llanowar Elves")), width);
    assert!(inside(name, laid.regions.name_bar), "{name}");
    let kind = corner_of(node(text_of(&laid.kind.lines[0])), width);
    assert!(inside(kind, laid.regions.type_bar), "{kind}");

    let text_box = nodes
        .iter()
        .copied()
        .find(|&e| world.entity(e).contains::<FaceTextBox>())
        .expect("a full face has a text box");
    let at = corner_of(node(text_box), width);
    assert!((at.y - laid.regions.text_box[1]).abs() < 1e-5, "{at}");
    assert!(matches!(node(text_box).overflow, o if o == Overflow::scroll_y()));

    let track = nodes
        .iter()
        .copied()
        .find(|&e| world.entity(e).contains::<FaceScrollbar>())
        .expect("a text box has its scrollbar");
    assert_eq!(
        node(track).display,
        Display::None,
        "until the text runs over"
    );

    // The cost at the name bar's right end, measured from the right.
    let pips = nodes
        .iter()
        .copied()
        .find(|&e| {
            matches!(node(e).right, Val::Px(_)) && node(e).flex_direction == FlexDirection::Row
        })
        .expect("the cost's row");
    let Val::Px(right) = node(pips).right else {
        unreachable!()
    };
    let edge = 1.0 - right / width;
    assert!(
        (edge - (laid.regions.name_bar[2] - TEXT_INSET)).abs() < 1e-5,
        "{edge}"
    );

    for e in nodes {
        assert!(
            world.entity(e).contains::<Pickable>(),
            "{:?} can take the pointer from the card",
            world.entity(e).get::<Name>()
        );
    }
}

/// A preview wears its band (the subtype words over the keyword chips,
/// read off its own keyword line), the set and rarity beside its type
/// line and its credit at the foot (WP6); and long rules show their
/// scrollbar and their `▾` from the frame they are spawned on.
#[test]
fn a_preview_wears_its_band_its_credit_and_its_scrollbar() {
    let mut face = creature("Tidecaller Adept", "Creature — Merfolk Wizard Ally");
    face.subtypes = SubtypeSet::from_slice(&[
        baylee_core::generated::subtypes::creature::MERFOLK,
        baylee_core::generated::subtypes::creature::WIZARD,
        baylee_core::generated::subtypes::creature::ALLY,
    ]);
    face.body = vec![
        TextBlock::Rules("Flying, vigilance".to_owned()),
        TextBlock::Rules("{T}: Draw a card for each Ally you control.".to_owned()),
    ];
    face.credit = Some(baylee_client_core::card_face::Credit {
        set: "zen".to_owned(),
        set_name: "Zendikar".to_owned(),
        rarity: "common".to_owned(),
        artist: "Ryan Pancoast".to_owned(),
    });
    let (world, laid, card) = overlay_face(&face, 308.0, Detail::Full);
    let nodes = nodes_under(&world, card);
    let texts: Vec<&str> = nodes
        .iter()
        .filter_map(|&e| world.entity(e).get::<Text>().map(|t| t.0.as_str()))
        .collect();
    for want in [
        "MERFOLK · WIZARD · ALLY",
        "Flying",
        "vigilance",
        "ZEN · C",
        "Zendikar · Ryan Pancoast",
    ] {
        assert!(texts.contains(&want), "no {want:?} in {texts:?}");
    }
    assert_eq!(laid.layout, Layout::Preview);
    let shown = |marker: fn(&World, Entity) -> bool| {
        nodes
            .iter()
            .find(|&&e| marker(&world, e))
            .map(|&e| world.get::<Node>(e).expect("a node").display)
    };
    let track = |w: &World, e: Entity| w.entity(e).contains::<FaceScrollbar>();
    let more = |w: &World, e: Entity| w.entity(e).contains::<FaceMore>();
    assert_eq!(shown(track), Some(Display::None), "short rules fit");
    assert_eq!(shown(more), Some(Display::None));

    face.body = vec![TextBlock::Rules("Draw a card. ".repeat(120))];
    let (world, laid, card) = overlay_face(&face, 308.0, Detail::Full);
    assert!(laid.overflows());
    let nodes = nodes_under(&world, card);
    for (what, found) in [
        (
            "scrollbar",
            nodes
                .iter()
                .find(|&&e| world.entity(e).contains::<FaceScrollbar>()),
        ),
        (
            "caret",
            nodes
                .iter()
                .find(|&&e| world.entity(e).contains::<FaceMore>()),
        ),
    ] {
        let node = world.get::<Node>(*found.expect(what)).expect("a node");
        assert_eq!(node.display, Display::Flex, "the {what} waits for a layout");
    }
}

/// A compact face is a small card: its keyword strip and one line of
/// rules, the first sentence, cut at a word where it runs past the
/// line; never the whole text.
#[test]
fn a_small_card_reads_one_line() {
    let mut face = creature("Tidecaller Adept", "Creature — Merfolk Wizard Ally");
    face.body = vec![
        TextBlock::Rules("Flying".to_owned()),
        TextBlock::Rules(
            "{T}: Draw a card for each Ally you control, then discard a card for each \
                 creature an opponent controls. Then do it again."
                .to_owned(),
        ),
    ];
    let font = regular();
    let widths = Widths::of(Some(&font));
    let laid = UiFace::lay(&face, Lang::En, 92.0, Detail::Compact, &widths, 0);
    assert_eq!(laid.layout, Layout::Small);
    assert_eq!(laid.chips, ["Flying"]);
    // The first rules block is the keyword line, and that is the line.
    assert_eq!(laid.line.as_deref(), Some("Flying"));
    face.body.remove(0);
    let laid = UiFace::lay(&face, Lang::En, 92.0, Detail::Compact, &widths, 0);
    let line = laid.line.expect("a line");
    assert!(line.starts_with("{T}: Draw"), "{line}");
    assert!(line.ends_with(textface::ELLIPSIS), "{line}");
    assert!(!line.contains("again"), "{line}");
}

/// A compact face writes no rules text and has no box to scroll, and a
/// card whose corner shows no plate says its numbers itself.
#[test]
fn a_compact_overlay_face_has_no_text_box() {
    let face = creature("Llanowar Elves", "Creature — Elf Druid");
    let (world, laid, card) = overlay_face(&face, 92.0, Detail::Compact);
    let nodes = nodes_under(&world, card);
    assert!(laid.body.is_none());
    assert!(
        !nodes
            .iter()
            .any(|&e| world.entity(e).contains::<FaceTextBox>())
    );
    assert!(
        nodes
            .iter()
            .any(|&e| world.entity(e).get::<Text>().is_some_and(|t| t.0 == "2/2")),
        "no plate in hand, so the face says 2/2"
    );

    // A card whose corner plates the same body is not told it twice.
    let font = regular();
    let plate = Plate::Fight {
        power: 2,
        toughness: 2,
        damage: 0,
    };
    let laid = UiFace::lay(
        &face,
        Lang::En,
        92.0,
        Detail::Compact,
        &Widths::of(Some(&font)),
        plate.packed(),
    );
    assert_eq!(laid.stats, None);
}

/// The word the overlay's material is keyed by carries the depths the
/// fit chose: a name that takes two lines is a deeper name bar, drawn
/// under the lines that need it. And the rules text steps down to fit.
#[test]
fn the_overlay_s_word_is_its_own_fit() {
    use textface::{Depths, FACE_NAME_SHIFT};
    let font = regular();
    let widths = Widths::of(Some(&font));
    let lay =
        |face: &CardFace, width: f32| UiFace::lay(face, Lang::En, width, Detail::Full, &widths, 0);
    let short = lay(&creature("Elves", "Creature — Elf"), 92.0);
    let long = lay(
        &creature("Okina, Temple to the Grandfathers", "Legendary Land"),
        92.0,
    );
    assert_eq!(short.name.lines.len(), 1);
    assert_eq!(long.name.lines.len(), 2);
    let depth = |laid: &UiFace| laid.word >> FACE_NAME_SHIFT & 0xff;
    assert!(depth(&long) > depth(&short));
    assert_eq!(
        depth(&long),
        u32::from(Depths::of(long.sizes.name_bar(2), 0.0).name)
    );

    // As the rules grow, they step down inside the preview's layout,
    // then take the long one, then scroll there.
    let wordy = |n: usize| {
        let mut face = creature("Elves", "Creature — Elf");
        face.body = vec![TextBlock::Rules("Flying. ".repeat(n))];
        lay(&face, 308.0)
    };
    let stepped = (10..80).map(wordy).find(|laid| {
        let px = laid.body.expect("a full face") * 308.0;
        laid.layout == Layout::Preview && px < 19.0
    });
    let px = stepped
        .expect("a text that steps down")
        .body
        .expect("a body")
        * 308.0;
    assert!((textface::LONG_PX..19.0).contains(&px), "{px}");
    assert!(
        (10..80)
            .map(wordy)
            .any(|laid| laid.layout == Layout::Long && !laid.overflows()),
        "no text took the long layout and fitted there"
    );
    let plain = lay(&creature("Elves", "Creature — Elf"), 308.0);
    let own = plain.body.expect("a full face") * 308.0;
    assert!((own - 19.096).abs() < 1e-3, "{own}");
    assert_eq!(plain.layout, Layout::Preview);
    assert_eq!(textface::layout_of(plain.word), Layout::Preview);

    // Rules that would go under 16 px at the full band take the long
    // layout, and the word says so.
    let mut long_rules = creature("Elves", "Creature — Elf");
    long_rules.body = vec![TextBlock::Rules("Flying. ".repeat(120))];
    let long = lay(&long_rules, 308.0);
    assert_eq!(long.layout, Layout::Long);
    assert_eq!(textface::layout_of(long.word), Layout::Long);
    assert!(long.overflows(), "nine hundred and sixty characters scroll");
    // And a compact face is a small card's.
    let small = UiFace::lay(&plain_face(), Lang::En, 92.0, Detail::Compact, &widths, 0);
    assert_eq!(textface::layout_of(small.word), Layout::Small);
}

/// A creature face with no rules, for the tests that need only a face.
fn plain_face() -> CardFace {
    creature("Elves", "Creature — Elf")
}

/// The rules text is drawn in the font the fit measured it in, at the
/// size the fit chose. It was drawn through `hud::tf`, which sets a line
/// `UI_SCALE` larger than it is asked for and a weight up when small, so
/// a text fitted to its box ran a fifth over it (#259).
#[test]
fn the_rules_text_is_drawn_as_it_was_fitted() {
    use bevy::text::{FontSize, FontSource};
    let mut face = creature("Llanowar Elves", "Creature — Elf Druid");
    face.body = vec![
        TextBlock::Rules("{T}: Add {G}.".to_owned()),
        TextBlock::Reminder("It taps for mana.".to_owned()),
    ];
    let width = 308.0;
    let (world, laid, card) = overlay_face(&face, width, Detail::Full);
    let fonts = test_fonts();
    let px = laid.body.expect("a full face") * width;
    let text_box = nodes_under(&world, card)
        .into_iter()
        .find(|&e| world.entity(e).contains::<FaceTextBox>())
        .expect("a full face has a text box");
    let mut words = 0;
    for e in nodes_under(&world, text_box) {
        let Some(font) = world.entity(e).get::<TextFont>() else {
            continue;
        };
        if font.font == FontSource::Handle(fonts.mana.clone()) {
            // A mark's glyph, on its disc.
            continue;
        }
        let said = world.entity(e).get::<Text>();
        assert_eq!(
            font.font,
            FontSource::Handle(fonts.text.clone()),
            "{said:?}"
        );
        assert_eq!(font.font_size, FontSize::Px(px), "{said:?}");
        words += 1;
    }
    assert!(words >= 3, "{words} runs of words");
}

/// An app that lays the interface out as the client does, with the
/// shipped fonts in it: bevy's own layout, headless, and the scrollbars
/// shown or hidden by what it measured.
pub(crate) fn layout_app() -> (App, UiFonts) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::text::TextPlugin,
        bevy::ui::UiPlugin,
        bevy::window::WindowPlugin {
            primary_window: None,
            ..default()
        },
        // Asked for by `UiPlugin`'s focus and picking systems, which a
        // measurement never uses.
        bevy::input::InputPlugin,
        bevy::picking::DefaultPickingPlugins,
    ));
    app.init_asset::<Image>()
        .init_asset::<bevy::image::TextureAtlasLayout>()
        .add_systems(
            PostUpdate,
            show_scrollbars.after(bevy::ui::UiSystems::Layout),
        );
    let mut store = app.world_mut().resource_mut::<Assets<Font>>();
    let mut load = |file: &str| {
        let path = format!("{}/assets/fonts/{file}", env!("CARGO_MANIFEST_DIR"));
        store.add(Font::from_bytes(std::fs::read(path).expect(file)))
    };
    let fonts = UiFonts {
        text: load("AlegreyaSans-Regular.ttf"),
        medium: load("AlegreyaSans-Medium.ttf"),
        bold: load("AlegreyaSans-Bold.ttf"),
        italic: load("AlegreyaSans-Italic.ttf"),
        medium_italic: load("AlegreyaSans-MediumItalic.ttf"),
        serif: load("Faustina.ttf"),
        serif_italic: load("Faustina-Italic.ttf"),
        icons: load("fa-solid-900.ttf"),
        mana: load("mana.ttf"),
    };
    (app, fonts)
}

/// What became of one face's rules text in [`layout_app`].
struct Measured {
    /// The size the fit set it at, and the size it would have had.
    fitted: f32,
    own: f32,
    /// Whether the fit's model said it fits its box.
    fits: bool,
    /// How deep the model and bevy stand it, in pixels, against the room.
    model: f32,
    real: f32,
    room: f32,
    /// Whether bevy's layout showed the scrollbar.
    bar: bool,
    /// Whether the scrollbar stood there from the first frame, before
    /// bevy had laid anything out (WP6).
    first: bool,
    /// The layout the rules chose.
    layout: Layout,
}

impl Measured {
    /// The acceptance (WP6): the text fits its box, or the box shows
    /// that it scrolls.
    fn fits_or_scrolls(&self) -> bool {
        self.real <= self.room + 0.5 || self.bar
    }
}

fn measure(
    app: &mut App,
    fonts: &UiFonts,
    face: (&CardFace, Lang),
    width: f32,
    plate: u32,
) -> Measured {
    measure_at(app, fonts, face, width, plate, textface::Step::DEFAULT)
}

fn measure_at(
    app: &mut App,
    fonts: &UiFonts,
    (face, lang): (&CardFace, Lang),
    width: f32,
    plate: u32,
    step: textface::Step,
) -> Measured {
    let (laid, model) = {
        let assets = app.world().resource::<Assets<Font>>();
        let widths = Widths::of(assets.get(&fonts.text)).at(step);
        let laid = UiFace::lay(face, lang, width, Detail::Full, &widths, plate);
        let em = laid.body.expect("a full face");
        let model = body_depth(&body_blocks(face, lang), em, width, &widths);
        (laid, model)
    };
    let room = body_room(&laid.regions, laid.stats, &laid.sizes);
    let card = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(width),
            height: Val::Px(width * 88.0 / 63.0),
            ..default()
        })
        .id();
    let mut commands = app.world_mut().commands();
    spawn_ui(&mut commands, card, lang, face, &laid, fonts);
    app.world_mut().flush();
    let track_of = |world: &World| {
        nodes_under(world, card)
            .into_iter()
            .find(|&e| world.entity(e).contains::<FaceScrollbar>())
            .expect("a full face has a scrollbar")
    };
    let shown = |world: &World| {
        world.get::<Node>(track_of(world)).expect("a node").display == Display::Flex
    };
    let first = shown(app.world());
    app.update();
    app.update();
    let world = app.world();
    let text_box = nodes_under(world, card)
        .into_iter()
        .find(|&e| world.entity(e).contains::<FaceTextBox>())
        .expect("a full face has a text box");
    let computed = world.get::<ComputedNode>(text_box).expect("laid out");
    let pad = 2.0 * BAR_PAD * width;
    let measured = Measured {
        fitted: laid.body.expect("a full face") * width,
        own: textface::ui_em_at(textface::UI_BODY, width, step) * width,
        fits: model <= room,
        model: model * width,
        real: computed.content_size().y * computed.inverse_scale_factor() - pad,
        room: room * width,
        bar: shown(world),
        first,
        layout: laid.layout,
    };
    app.world_mut().entity_mut(card).despawn();
    measured
}

/// The first face of pool card `index`, with its English Oracle text,
/// and the plate its corner would show.
fn pool_face(index: usize) -> Option<(CardFace, u32)> {
    pool_face_in(index, None)
}

/// [`pool_face`], with `text_in` (a language, and the card's rules text
/// in it) in place of the English Oracle where it is given.
fn pool_face_in(index: usize, text_in: Option<(&str, &str)>) -> Option<(CardFace, u32)> {
    use baylee_client_core::card_face::{CardText, Characteristics};
    let def = baylee_cards::by_index(baylee_core::ids::CardIndex::new(u32::try_from(index).ok()?))?;
    let printed = def.faces.first()?;
    let oracle: &str = baylee_cards::generated_oracle::ORACLE.get(index)?.first()?;
    let chars = Characteristics {
        name: printed.name.to_owned(),
        types: printed.types,
        supertypes: printed.supertypes,
        subtypes: SubtypeSet::from_slice(printed.subtypes),
        colors: ColorSet::EMPTY,
        power: printed.power,
        toughness: printed.toughness,
        loyalty: printed.loyalty,
        damage: 0,
    };
    let (lang, oracle) = text_in.unwrap_or(("en", oracle));
    let text = CardText {
        lang: lang.to_owned(),
        name: printed.name.to_owned(),
        type_line: String::new(),
        oracle_text: oracle.to_owned(),
        mana_cost: String::new(),
        english_name: printed.name.to_owned(),
    };
    let face = CardFace::build(
        &chars,
        Some(&printed.mana_cost),
        Some(printed_types(printed)),
        Some(&text),
    );
    let kind = if printed.loyalty.is_some() {
        cardplate::KIND_LOYALTY
    } else if printed.power.is_some() {
        cardplate::KIND_FIGHT
    } else {
        cardplate::KIND_NONE
    };
    Some((face, kind << cardplate::KIND_SHIFT))
}

/// The pool's cards with rules text, hardest to model first: the most
/// marks, then the longest.
fn hardest_first() -> Vec<usize> {
    use std::cmp::Reverse;
    let oracle = baylee_cards::generated_oracle::ORACLE;
    let mut cards: Vec<usize> = (0..oracle.len())
        .filter(|&i| oracle[i].first().is_some_and(|t| !t.is_empty()))
        .collect();
    cards.sort_by_key(|&i| {
        let text = oracle[i][0];
        (Reverse(text.matches('{').count()), Reverse(text.len()))
    });
    cards
}

/// A face whose rules text the fit says fits shows no scrollbar when
/// bevy lays it out: the fit's model errs deep and never shallow. And
/// every face fits or scrolls, with the scrollbar standing from the
/// first frame wherever bevy's layout shows it (WP6). Over the forty pool
/// cards whose text is hardest to model, at four preview widths, and at
/// the default preview's 308 at the smallest and the largest step.
#[test]
fn a_face_fitted_to_its_box_fits_it_in_bevy_s_layout() {
    let (mut app, fonts) = layout_app();
    let (mut fits, mut stepped, mut scrolled) = (0, 0, 0);
    let sizes = [231.0, 308.0, 384.0, 480.0]
        .map(|w| (w, textface::Step::DEFAULT))
        .into_iter()
        .chain([(308.0, textface::Step::XS), (308.0, textface::Step::XL)]);
    let sizes: Vec<_> = sizes.collect();
    for index in hardest_first().into_iter().take(40) {
        let (face, plate) = pool_face(index).expect("a pool card");
        for &(width, step) in &sizes {
            let m = measure_at(&mut app, &fonts, (&face, Lang::En), width, plate, step);
            let what = format!(
                "{} at {width}, {step:?}: fitted at {} px to {:.1} px of room, and bevy \
                     stands it {:.1} deep (the model said {:.1})",
                face.name, m.fitted, m.room, m.real, m.model
            );
            assert!(m.fits_or_scrolls(), "{what}: runs over with no scrollbar");
            assert!(!m.bar || m.first, "{what}: the scrollbar came a frame late");
            scrolled += usize::from(m.bar);
            if m.fits {
                fits += 1;
                stepped += usize::from(m.fitted < m.own);
                assert!(!m.bar, "{what}");
            }
        }
    }
    assert!(
        fits >= 30 && stepped >= 10 && scrolled > 0,
        "{fits} fitted, {stepped} of them stepped down, {scrolled} scrolled"
    );
}

/// How much of the pool's rules text still runs over at the floor, by
/// bevy's layout, in English — and that every face fits or scrolls, with
/// its scrollbar from the first frame (WP6), which this asserts. Over the
/// whole pool, so not in the gate: run it by name with `--ignored
/// --nocapture`.
///
/// In another language (#289): `BAYLEE_MEASURE_TEXTS` names a file of
/// `English name<TAB>rules text` lines (a newline in the text written
/// `\\n`), `BAYLEE_MEASURE_LANG` its language (`de` unless said), and
/// only the cards the file names are measured, in its words. The
/// catalog has them (`card_faces.printed_text` of the newest printing in
/// that language); nothing here reads the database.
#[test]
#[ignore = "a measurement over the whole pool; run by name"]
fn how_much_of_the_pool_runs_over_at_the_floor() {
    let (mut app, fonts) = layout_app();
    let printed: Option<std::collections::BTreeMap<String, String>> =
        std::env::var("BAYLEE_MEASURE_TEXTS").ok().map(|path| {
            std::fs::read_to_string(path)
                .expect("the texts file")
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .map(|(name, text)| (name.to_owned(), text.replace("\\n", "\n")))
                .collect()
        });
    let code = std::env::var("BAYLEE_MEASURE_LANG").unwrap_or_else(|_| "de".to_owned());
    let lang = if printed.is_some() {
        Lang::of(&code)
    } else {
        Lang::En
    };
    let cards: Vec<(CardFace, u32)> = hardest_first()
        .into_iter()
        .filter_map(|index| match &printed {
            None => pool_face(index),
            Some(texts) => {
                let (english, _) = pool_face(index)?;
                let text = texts.get(&english.name)?;
                pool_face_in(index, Some((&code, text)))
            }
        })
        .collect();
    let sizes = [231.0, 308.0, 384.0, 480.0]
        .map(|w| (w, textface::Step::DEFAULT))
        .into_iter()
        .chain([(308.0, textface::Step::XS), (308.0, textface::Step::XL)]);
    let mut broken = Vec::new();
    for (width, step) in sizes {
        let (mut over, mut own, mut long, mut worst) = (0, 0, 0, 0.0_f32);
        let (mut wrong, mut late, mut flicker) = (Vec::new(), 0, 0);
        for (face, plate) in &cards {
            let m = measure_at(&mut app, &fonts, (face, lang), width, *plate, step);
            over += usize::from(m.bar);
            long += usize::from(m.layout == Layout::Long);
            if m.fits && m.bar {
                wrong.push(format!(
                    "{} ({} px, room {:.1}, model {:.1}, bevy {:.1})",
                    face.name, m.fitted, m.room, m.model, m.real
                ));
            }
            late += usize::from(m.bar && !m.first);
            flicker += usize::from(m.first && !m.bar);
            if !m.fits_or_scrolls() {
                broken.push(format!("{} at {width}, {step:?}", face.name));
            }
            own += usize::from((m.fitted - m.own).abs() < 1e-3 && !m.bar);
            if m.real > 0.0 {
                worst = worst.max(m.real / m.model);
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let share = |n: usize| 100.0 * n as f32 / cards.len() as f32;
        println!(
            "{width} px, step {}: {} cards; {over} run over at the floor ({:.1}%); {own} \
                 fit at their own size ({:.1}%); {long} take the long layout ({:.1}%); bevy's \
                 depth against the model's at most {worst:.3}",
            step.number(),
            cards.len(),
            share(over),
            share(own),
            share(long),
        );
        println!(
            "  fitted and still running over: {} {wrong:?}; scrollbar a frame late: \
                 {late}; shown at first and gone after: {flicker}",
            wrong.len()
        );
    }
    assert!(broken.is_empty(), "runs over with no scrollbar: {broken:?}");
}

/// How much of a 308-pixel preview its rules text fills, over the twenty
/// longest rules texts of the pool (WP6): the text's depth as bevy lays
/// it out, up to its box, as a share of the card's height, beside the
/// box's own share and the size the text was set at. A measurement and
/// not a check: run it by name with `--ignored --nocapture`.
#[test]
#[ignore = "a measurement over the pool's longest texts; prints, asserts nothing"]
fn how_much_of_the_face_the_longest_texts_fill() {
    let (mut app, fonts) = layout_app();
    let oracle = baylee_cards::generated_oracle::ORACLE;
    let mut longest: Vec<usize> = (0..oracle.len())
        .filter(|&i| oracle[i].first().is_some_and(|t| !t.is_empty()))
        .collect();
    longest.sort_by_key(|&i| std::cmp::Reverse(oracle[i][0].len()));
    let width = 308.0;
    let height = width * 88.0 / 63.0;
    let (mut fill, mut room) = (0.0_f32, 0.0_f32);
    for index in longest.into_iter().take(20) {
        let (face, plate) = pool_face(index).expect("a pool card");
        let m = measure(&mut app, &fonts, (&face, Lang::En), width, plate);
        let filled = m.real.min(m.room) / height;
        fill += filled;
        room += m.room / height;
        println!(
            "{:<36} {:>4.1} px  fills {:>5.1}% of the face  box {:>5.1}%  {}",
            face.name,
            m.fitted,
            100.0 * filled,
            100.0 * m.room / height,
            if m.bar { "scrolls" } else { "fits" }
        );
    }
    println!(
        "mean: text fills {:.1}% of the face, its box is {:.1}%",
        100.0 * fill / 20.0,
        100.0 * room / 20.0
    );
}

/// The thumb stands in proportion to what is shown, never shorter than
/// its minimum, and there is none while the text fits.
#[test]
fn a_thumb_shows_the_share_and_the_place() {
    assert_eq!(thumb(100.0, 100.0, 0.0), None);
    assert_eq!(thumb(100.0, 100.4, 0.0), None, "half a pixel is a fit");
    assert_eq!(thumb(100.0, 200.0, 0.0), Some((0.0, 0.5)));
    assert_eq!(thumb(100.0, 200.0, 100.0), Some((0.5, 0.5)));
    let (top, length) = thumb(100.0, 100_000.0, 99_900.0).expect("a bar");
    assert!((length - THUMB_MIN).abs() < f32::EPSILON);
    assert!((top + length - 1.0).abs() < 1e-5, "at the end");
}

/// The system shows a scrollbar only while its box runs over, and stands
/// its thumb where the box has scrolled to.
#[test]
fn a_scrollbar_is_shown_while_its_text_runs_over() {
    let mut app = App::new();
    app.add_systems(Update, show_scrollbars);
    let text_box = app
        .world_mut()
        .spawn((
            FaceTextBox,
            ScrollPosition(Vec2::new(0.0, 100.0)),
            ComputedNode {
                size: Vec2::new(200.0, 100.0),
                content_size: Vec2::new(200.0, 200.0),
                ..default()
            },
        ))
        .id();
    let thumb = app
        .world_mut()
        .spawn((FaceScrollThumb, Node::default()))
        .id();
    let track = app
        .world_mut()
        .spawn((
            FaceScrollbar { text_box },
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .add_child(thumb)
        .id();
    app.update();
    let node = |e: Entity, app: &App| {
        app.world()
            .entity(e)
            .get::<Node>()
            .cloned()
            .expect("a node")
    };
    assert_eq!(node(track, &app).display, Display::Flex);
    assert_eq!(node(thumb, &app).top, Val::Percent(50.0));
    assert_eq!(node(thumb, &app).height, Val::Percent(50.0));

    app.world_mut().entity_mut(text_box).insert(ComputedNode {
        size: Vec2::new(200.0, 100.0),
        content_size: Vec2::new(200.0, 100.0),
        ..default()
    });
    app.update();
    assert_eq!(node(track, &app).display, Display::None);
}

/// A pool card's face is the row's words on the printed card (#259): the
/// name and type line the gateway served in the player's language, the
/// colours the row names, the numbers the registry prints — and without
/// a catalog, the English Oracle rather than an empty box.
#[test]
fn a_pool_card_s_face_is_the_row_s_words_on_the_printed_card() {
    use baylee_client_core::deckbuilder::PoolCard;
    let row = PoolCard {
        index: baylee_cards::decks::by_name("Birds of Paradise")
            .expect("in the pool")
            .get(),
        name: "Paradiesvögel".to_owned(),
        english_name: "Birds of Paradise".to_owned(),
        mana_cost: "{G}".to_owned(),
        colors: "G".to_owned(),
        type_line: "Kreatur — Vogel".to_owned(),
        oracle_text: "Fliegend".to_owned(),
        ..PoolCard::default()
    };
    let face = of_pool(&row);
    assert_eq!(face.name, "Paradiesvögel");
    assert_eq!(face.type_line, "Kreatur — Vogel");
    assert_eq!(face.cost, vec![ManaSymbol::Green]);
    assert_eq!(face.colors, ColorSet::of(MagicColor::Green));
    assert_eq!(
        face.stats,
        Some(Stats::PowerToughness {
            power: 0,
            toughness: 1,
            damage: 0
        })
    );
    assert_eq!(face.body, vec![TextBlock::Rules("Fliegend".to_owned())]);

    let bare = of_pool(&PoolCard {
        oracle_text: String::new(),
        ..row
    });
    let text: Vec<_> = bare.body.iter().map(TextBlock::text).collect();
    assert_eq!(text, ["Flying", "{T}: Add one mana of any color."]);
    assert!(!bare.text_pending);
}

/// A number the ledge already shows is not written on the face again,
/// and a number it does not show is.
#[test]
fn the_face_leaves_the_body_to_the_plate() {
    let body = Stats::PowerToughness {
        power: 3,
        toughness: 3,
        damage: 0,
    };
    let fight = Plate::Fight {
        power: 3,
        toughness: 3,
        damage: 0,
    };
    assert_eq!(world_stats(Some(body), fight.kind()), None);
    assert_eq!(
        world_stats(Some(Stats::Loyalty(4)), Plate::Loyalty(4).kind()),
        None
    );
    assert_eq!(
        world_stats(Some(body), Plate::Loyalty(4).kind()),
        Some(body),
        "an animated planeswalker's body is said nowhere else"
    );
    assert_eq!(world_stats(Some(body), Plate::None.kind()), Some(body));
    assert_eq!(world_stats(None, fight.kind()), None);
    // The overlay asks with the packed word its look carries.
    assert_eq!(fight.packed() >> cardplate::KIND_SHIFT, fight.kind());
    assert_eq!(
        Plate::None.packed() >> cardplate::KIND_SHIFT,
        Plate::None.kind()
    );
}

#[test]
fn pips_label_every_symbol_a_cost_can_contain() {
    let cost = ManaCost::parse("{2}{W}{U/B}{2/R}{G/P}{X}{S}{C}");
    for symbol in cost.symbols() {
        let label = pip_label(symbol);
        assert!(!label.is_empty(), "{symbol:?} has no label");
    }
}

/// A player reads the damaged toughness to decide a block, so it has to be
/// the number in front, with the printed one kept for context.
#[test]
fn a_damaged_creature_shows_what_is_left() {
    let stats = Stats::PowerToughness {
        power: 3,
        toughness: 4,
        damage: 3,
    };
    assert_eq!(stats_label(stats), "3/1 (4)");
    assert_eq!(
        stats_label(Stats::PowerToughness {
            power: 2,
            toughness: 2,
            damage: 0
        }),
        "2/2"
    );
    assert_eq!(stats_label(Stats::Loyalty(4)), "4");
}

/// Lethal damage is the one state that must be visible without reading
/// the numbers.
#[test]
fn lethal_damage_turns_the_numbers_red() {
    let lethal = Stats::PowerToughness {
        power: 1,
        toughness: 2,
        damage: 2,
    };
    assert_eq!(stats_color(lethal), FACE_INKS.1);
    assert_ne!(FACE_INKS.1, FACE_INKS.0);
    assert_eq!(
        stats_color(Stats::PowerToughness {
            power: 1,
            toughness: 2,
            damage: 1
        }),
        FACE_INKS.0
    );
}

/// The five independent reasons to draw the face. Each one alone is
/// enough, and none of them may need the others.
#[test]
fn every_reason_to_draw_the_face_stands_on_its_own() {
    use baylee_client_core::images::{ArtSize, ImageKey};
    use baylee_core::ids::PrintRef;

    let mut images = Assets::<Image>::default();
    let mut textures = crate::textures::CardTextures::new(&mut images, 1 << 20);
    let art = ImageKey::new(PrintRef::new(0), 0, ArtSize::Small);
    let quiet = FaceMode::default();
    let held = FaceMode {
        held: true,
        ..FaceMode::default()
    };
    let plain = crate::settings::ClientSettings::default();
    let latched = crate::settings::ClientSettings {
        prefer_text_view: true,
        ..crate::settings::ClientSettings::default()
    };

    // Art still in flight. This one used to read the other way — the
    // image won the moment a handle existed — and that is precisely how a
    // card came to be drawn as nothing: an unloaded texture cannot be
    // bound, so the material never prepares.
    assert!(wants_face(&quiet, &plain, &textures, Some(art)));

    textures.mark_arrived(art);
    // Art that is on the GPU: the image wins.
    assert!(!wants_face(&quiet, &plain, &textures, Some(art)));
    // The modifier, the latch, and a token with no printing at all.
    assert!(wants_face(&held, &plain, &textures, Some(art)));
    assert!(wants_face(&quiet, &latched, &textures, Some(art)));
    assert!(wants_face(&quiet, &plain, &textures, None));
    // And art that will never arrive.
    let lost = ImageKey::new(PrintRef::new(1), 0, ArtSize::Small);
    textures.mark_failed(lost, crate::textures::Failure::Load(1));
    assert!(wants_face(&quiet, &plain, &textures, Some(lost)));
}

/// A face's helper for the tests below: one card, its name translated.
fn german(english: &str, translated: &str) -> crate::cardtext::CardTexts {
    crate::cardtext::CardTexts::filed(crate::cardtext::fixture::german(english, translated, None))
}

/// An ability has no card of its own, so its name has to be looked up
/// through the permanent it came from. Nothing else in the client reaches
/// a printing that way, which is why this one is worth a test: the
/// obvious implementation answers `None` for every ability on the stack
/// and leaves the whole panel English.
#[test]
fn an_ability_is_named_through_the_permanent_it_came_from() {
    use baylee_client_core::test_support::{ViewBuilder, printed, token};

    let texts = german("Flooded Strand", "Gefluteter Strand");
    let card = crate::cardtext::fixture::card("Flooded Strand");
    let mut ability = token(30, 0, "Flooded Strand", 0, 0);
    ability.card = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: baylee_core::ids::ObjectId::new(7, 0),
        ability: None,
        rules: Some(baylee_view::RulesFace { card, face: 0 }),
        text: Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 1,
        }),
    });
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![crate::cardtext::fixture::showing(
                printed(7, 0, "Flooded Strand", 7),
                card,
            )],
        )
        .with_stack(vec![ability.clone()])
        .build();

    assert_eq!(
        name_of(&ability, &view, &texts),
        "Gefluteter Strand",
        "the ability borrows its source's printing"
    );
    // The source itself, which is the ordinary path.
    assert_eq!(
        name_of(
            view.object(baylee_core::ids::ObjectId::new(7, 0)).unwrap(),
            &view,
            &texts
        ),
        "Gefluteter Strand"
    );
}

/// The stack panel draws an ability as the *picture* of the permanent it
/// came from, and the picture carries a name. Found live: a Marsh Flats
/// ability read "Brackmarsch" in the row's title and "Marsh Flats" on the
/// thumbnail two inches to its left, because the thumbnail is built from
/// the stack object and a stack object for an ability has no `card` to
/// look text up by. The view is what closes it.
#[test]
fn the_picture_beside_an_ability_is_named_like_the_ability() {
    use baylee_client_core::test_support::{ViewBuilder, printed, token};

    let texts = german("Marsh Flats", "Brackmarsch");
    let card = crate::cardtext::fixture::card("Marsh Flats");
    let mut ability = token(30, 0, "Marsh Flats", 0, 0);
    ability.card = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: baylee_core::ids::ObjectId::new(7, 0),
        ability: None,
        rules: Some(baylee_view::RulesFace { card, face: 0 }),
        text: Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 1,
        }),
    });
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![crate::cardtext::fixture::showing(
                printed(7, 0, "Marsh Flats", 7),
                card,
            )],
        )
        .with_stack(vec![ability.clone()])
        .build();

    assert_eq!(
        of_object(&ability, Some(&view), &texts).name,
        "Brackmarsch",
        "the thumbnail borrows the source's printing, like the title above it"
    );
    // The counter-test: a caller with no view is drawing a permanent, and
    // a permanent carries its own printing.
    assert_eq!(
        of_object(
            view.object(baylee_core::ids::ObjectId::new(7, 0)).unwrap(),
            None,
            &texts
        )
        .name,
        "Brackmarsch"
    );
}

/// An ability outlives its source (CR 113.7a). There is then no printing
/// to ask, and the projected name is all there is — which is the honest
/// answer, not a bug to paper over.
#[test]
fn an_ability_whose_source_has_left_keeps_the_name_it_has() {
    use baylee_client_core::test_support::{ViewBuilder, token};

    let texts = german("Flooded Strand", "Gefluteter Strand");
    let mut ability = token(30, 0, "Flooded Strand", 0, 0);
    ability.card = None;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: baylee_core::ids::ObjectId::new(7, 0),
        ability: None,
        rules: None,
        text: None,
    });
    let view = ViewBuilder::new(2)
        .with_stack(vec![ability.clone()])
        .build();

    assert_eq!(name_of(&ability, &view, &texts), "Flooded Strand");
}

/// The table quad is tinted by colour identity, so two different decks
/// never read as the same wall of grey rectangles.
#[test]
fn the_table_face_is_tinted_by_colour_identity() {
    let red = table_color(ColorSet::from_slice(&[MagicColor::Red]));
    let blue = table_color(ColorSet::from_slice(&[MagicColor::Blue]));
    assert_ne!(red, blue);
    assert_ne!(red, PAPER);
}

/// Frames follow the printed convention: mono gets its colour, multicolour
/// gets gold, colourless gets grey.
#[test]
fn frames_follow_the_printed_convention() {
    let mono = frame_color(ColorSet::from_slice(&[MagicColor::Blue]));
    let gold = frame_color(ColorSet::from_slice(&[MagicColor::Blue, MagicColor::Red]));
    let colorless = frame_color(ColorSet::default());
    assert_ne!(mono, gold);
    assert_ne!(gold, colorless);
    assert_ne!(mono, colorless);
}
