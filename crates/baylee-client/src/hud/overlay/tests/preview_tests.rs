//! The hover preview: its back face, its attachments, where it stands.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// The overlay offers a back only for a card that has a second picture.
///
/// This predicate is the **second** implementation of a question
/// `PoolCard` also answers, and it counted the card's compiled faces
/// until #115 — so an Adventure, which prints two names on one piece of
/// card, was handed a back `ImageKey` and the shelf it points at answers
/// 404. Both now read one table in `baylee_cards::sides`; asking both
/// directions here is what would catch them coming apart again.
#[test]
fn only_a_card_with_a_second_picture_is_turned_over() {
    let (view, id) = hovering("Agadeem's Awakening");
    assert!(
        has_back_image(&view, id),
        "a modal double-faced card has a back to turn to"
    );
    let (view, id) = hovering("Murderous Rider");
    let adventure = baylee_cards::decks::by_name("Murderous Rider").expect("in the pool");
    assert!(
        baylee_cards::by_index(adventure).is_some_and(|def| def.faces.len() > 1),
        "the premise: this card compiles two faces, which is what used to decide"
    );
    assert!(
        !has_back_image(&view, id),
        "an Adventure prints both halves on one side"
    );
    let (view, id) = hovering("Lightning Bolt");
    assert!(!has_back_image(&view, id), "an ordinary card has no back");
}

#[test]
fn a_double_faced_card_in_hand_previews_its_actual_back() {
    for (name, double_faced) in [
        ("Khalni Ambush", true),
        ("Agadeem's Awakening", true),
        ("Murderous Rider", false),
        ("Lightning Bolt", false),
    ] {
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2)
            .with_hand(vec![(name, 3, 4)])
            .build();
        view.hand[0].card.index = baylee_cards::decks::by_name(name).expect("registered");
        let id = view.hand[0].id;
        assert!(
            view.object(id).is_none(),
            "hand cards are not public objects"
        );
        assert_eq!(has_back_image(&view, Some(id)), double_faced, "{name}");
        let front = ImageKey::new(view.hand[0].card.print, 0, ArtSize::Normal);
        assert_eq!(
            far_face(Some(front), has_back_image(&view, Some(id))).is_some(),
            double_faced
        );
    }
}

/// Nothing hovered, and a token, are both "no back" rather than a panic.
#[test]
fn a_token_and_an_empty_hover_have_no_back() {
    let (view, _) = hovering("Agadeem's Awakening");
    assert!(!has_back_image(&view, None), "nothing is hovered");
    let token = baylee_client_core::test_support::token(7, 0, "Goblin", 1, 1);
    let id = token.id;
    let view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_battlefield(0, [token])
        .build();
    assert!(!has_back_image(&view, Some(id)), "a token has no card");
}

/// A card in hand draws its name, cost, type line, keyword strip and one
/// line of rules, and no box of rules text: at the hand's width the whole
/// text would be six pixels, and hovering the card opens the preview, which
/// is where it is read (#259, WP6).
///
/// No art arrives in a headless test, so the card draws its face; the
/// first half proves it did, which is what keeps the second from passing
/// on a hand that drew nothing at all.
#[test]
fn a_card_in_hand_draws_no_rules_text() {
    use baylee_client_core::test_support::{ViewBuilder, statics};
    let mut duel = duel_saying(false, false);
    duel.view = Some(ViewBuilder::new(2).with_hand(vec![("Fire", 2, 4)]).build());
    duel.statics = Some(statics(8));
    crate::rebuild_board(&mut duel);
    let mut app = bar_of(duel);
    let world = app.world_mut();
    let cards: Vec<Entity> = world
        .query_filtered::<Entity, With<crate::hud::HandRowCard>>()
        .iter(world)
        .collect();
    assert_eq!(cards.len(), 1, "the one card in hand is in the row");
    let under: Vec<Entity> = {
        let mut children = world.query::<&Children>();
        let children = children.query(world);
        children.iter_descendants(cards[0]).collect()
    };
    let mut words = world.query::<&TextSpan>();
    let mut texts = world.query::<&Text>();
    assert!(
        under.iter().any(|e| {
            words.get(world, *e).is_ok_and(|s| !s.0.is_empty())
                || texts.get(world, *e).is_ok_and(|t| !t.0.is_empty())
        }),
        "the card drew its face, and a face has a name"
    );
    let mut boxes = world.query_filtered::<(), With<crate::face::FaceTextBox>>();
    assert!(
        !under.iter().any(|e| boxes.get(world, *e).is_ok()),
        "and no box of rules text under it"
    );
}

/// A text face is built when something it shows changes and never on an
/// idle frame (WP6): with a hovered permanent's preview up, sixty frames
/// of nothing build no face, and moving the hover builds them again — the
/// second half is what keeps the first from passing on an overlay that
/// built nothing at all.
#[test]
fn an_idle_preview_builds_no_face() {
    let (view, id) = hovering("Lightning Bolt");
    let mut duel = duel_saying(false, false);
    duel.view = Some(view);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    crate::rebuild_board(&mut duel);
    duel.hovered = id;
    let mut app = bar_of(duel);
    app.init_resource::<crate::face::FaceBuilds>();
    // A rebuild for the resource's own arrival, then the preview.
    app.world_mut().resource_mut::<Duel>().hovered = None;
    app.update();
    app.world_mut().resource_mut::<Duel>().hovered = id;
    app.update();
    let built = app.world().resource::<crate::face::FaceBuilds>().0;
    assert!(built > 0, "the hovered card's preview drew no face");
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<crate::face::FaceBuilds>().0,
        built,
        "an idle frame built a face"
    );
    app.world_mut().resource_mut::<Duel>().hovered = None;
    app.update();
    app.world_mut().resource_mut::<Duel>().hovered = id;
    app.update();
    assert!(
        app.world().resource::<crate::face::FaceBuilds>().0 > built,
        "a hover that came back built nothing"
    );
}

/// Hovering a permanent lays the cards attached to it beside its preview
/// (#305), which is where a player reads what the host's attachment mark
/// counts: a printed aura by its art, a Role, a token, by its
/// characteristics. And a permanent with nothing attached lays none.
#[test]
fn a_hovered_host_shows_what_is_attached_to_it_beside_its_preview() {
    use baylee_client_core::test_support::{ViewBuilder, printed, token};
    let (host, bare) = (ObjectId::new(5, 0), ObjectId::new(8, 0));
    let mut aura = printed(6, 0, "Pacifism", 3);
    aura.types = baylee_core::types::TypeSet::ENCHANTMENT;
    aura.attached_to = Some(host);
    let mut role = token(7, 0, "Monster Role", 0, 0);
    role.types = baylee_core::types::TypeSet::ENCHANTMENT;
    role.attached_to = Some(host);
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            [
                printed(5, 0, "Grizzly Bears", 2),
                aura,
                role,
                printed(8, 0, "Hill Giant", 4),
            ],
        )
        .build();
    let shown = |hovered: ObjectId| {
        let mut duel = duel_with(false);
        duel.view = Some(view.clone());
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        crate::rebuild_board(&mut duel);
        duel.hovered = Some(hovered);
        let mut app = bar_of(duel);
        app.update();
        let mut panels = app
            .world_mut()
            .query_filtered::<Entity, With<PreviewAttached>>();
        let panels: Vec<Entity> = panels.iter(app.world()).collect();
        let children = |app: &App, of: Entity| -> Vec<Entity> {
            app.world()
                .get::<Children>(of)
                .map(|c| c.iter().collect())
                .unwrap_or_default()
        };
        // The caption, then the grid: columns of cards.
        let cards: usize = panels
            .iter()
            .flat_map(|&panel| children(&app, panel).into_iter().skip(1))
            .flat_map(|grid| children(&app, grid))
            .map(|column| children(&app, column).len())
            .sum();
        (panels.len(), cards)
    };
    assert_eq!(shown(host), (1, 2), "the host's two attachments");
    assert_eq!(shown(bare), (0, 0), "nothing is attached to it");
}

/// A card a log line names opens the table's preview while the pointer
/// is on its link (#300), and closes it when the pointer leaves: the
/// overlay draws it from the link alone, with nothing on the table under
/// the pointer at all.
#[test]
fn a_log_link_opens_the_preview_of_the_card_it_names() {
    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    let mut app = bar_of(duel);
    app.update();
    let previews = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PreviewResize>>();
        q.iter(app.world()).count()
    };
    assert_eq!(previews(&mut app), 0, "a preview with nothing hovered");
    let span = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<Duel>().hovered_log = Some(super::super::LogHover {
        span,
        link: super::super::LogLink {
            card: Some(baylee_view::CardIdentity {
                index: baylee_core::ids::CardIndex::new(7),
                print: baylee_core::ids::PrintRef::new(3),
                face: 0,
            }),
            token: None,
        },
        at: Vec2::new(300.0, 200.0),
        card: None,
    });
    app.update();
    assert_eq!(previews(&mut app), 1, "the link opened no preview");
    app.world_mut().resource_mut::<Duel>().hovered_log = None;
    app.update();
    assert_eq!(previews(&mut app), 0, "the preview outlived the hover");
}

/// The rung a UI node is drawn at among the roots: its own
/// `GlobalZIndex`, else its nearest ancestor's, else the root's zero.
/// Bevy orders the roots and every node carrying one by it, and walks
/// everything else inside its parent.
fn drawn_at(app: &App, node: Entity) -> i32 {
    let mut at = Some(node);
    while let Some(node) = at {
        if let Some(z) = app.world().get::<GlobalZIndex>(node) {
            return z.0;
        }
        at = app.world().get::<ChildOf>(node).map(ChildOf::parent);
    }
    0
}

/// A card link in the end screen's log opens its preview over the end
/// screen, not behind it: the owner hovered one and saw the card through
/// the veil, under the sheet he was pointing at. And while the game is
/// going the preview keeps the overlay's own rung, under the ability
/// sheet and the other roots it was ordered against before.
#[test]
fn a_preview_opened_from_the_end_screen_s_log_stands_over_the_end_screen() {
    let preview_rung = |over: bool| {
        let mut duel = duel_with(over);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        let mut app = bar_of(duel);
        app.update();
        let span = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Duel>().hovered_log = Some(super::super::LogHover {
            span,
            link: super::super::LogLink {
                card: Some(baylee_view::CardIdentity {
                    index: baylee_core::ids::CardIndex::new(7),
                    print: baylee_core::ids::PrintRef::new(3),
                    face: 0,
                }),
                token: None,
            },
            at: Vec2::new(300.0, 200.0),
            card: None,
        });
        app.update();
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PreviewResize>>();
        let previews: Vec<Entity> = q.iter(app.world()).collect();
        assert_eq!(previews.len(), 1, "the link opened one preview");
        drawn_at(&app, previews[0])
    };
    assert!(
        preview_rung(true) > G_FINISH,
        "the preview is drawn under the end screen"
    );
    assert_eq!(preview_rung(false), 0, "and in play, with the overlay");
}

/// A hover on a card link, as `hover_log_links` writes it.
fn log_hover(span: Entity, at: Vec2, card: Option<Rect>) -> super::super::LogHover {
    super::super::LogHover {
        span,
        link: super::super::LogLink {
            card: Some(baylee_view::CardIdentity {
                index: baylee_core::ids::CardIndex::new(7),
                print: baylee_core::ids::PrintRef::new(3),
                face: 0,
            }),
            token: None,
        },
        at,
        card,
    }
}

/// The overlay's rebuild sweeps every child of the root it does not name, and
/// the reveal sheet is named: a pointer resting on a revealed card opens a
/// preview, which is itself a rebuild, and the sheet it took with it stood
/// up again as new cards under the pointer, so the reveal flickered while
/// the pointer rested (owner, 12.10.2026).
///
/// The loose node beside the sheet is the control: it is swept, which is
/// what shows the rebuild ran at all and the sheet came through it.
#[test]
fn the_reveal_sheet_survives_the_overlay_s_rebuild() {
    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    let mut app = bar_of(duel);
    app.update();
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<super::super::HudRoot>>()
        .single(app.world())
        .expect("the overlay's root");
    let sheet = app
        .world_mut()
        .spawn((crate::hud::revealed::RevealSheet, Node::default()))
        .id();
    let loose = app.world_mut().spawn(Node::default()).id();
    app.world_mut()
        .entity_mut(root)
        .add_children(&[sheet, loose]);
    let span = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<Duel>().hovered_log =
        Some(log_hover(span, Vec2::new(300.0, 200.0), None));
    app.update();
    assert!(
        app.world().get_entity(loose).is_err(),
        "the premise: the hover rebuilt the overlay and swept what it does not name"
    );
    assert!(
        app.world().get_entity(sheet).is_ok(),
        "the rebuild took the reveal sheet with it"
    );
    assert_eq!(
        app.world().get::<ChildOf>(sheet).map(ChildOf::parent),
        Some(root),
        "and the sheet still hangs off the overlay's root"
    );
}

/// The preview a revealed card opens stands beside that card's edge, not
/// beside the pointer: the pointer is on the card, so a preview opened at it
/// would cover the card it is of. The control is the same hover without the
/// card's box, which opens at the pointer, over the card.
///
/// The card is a 120 x 168 box in the left of the harness's 1200 x 800
/// window, so there is room to its right for the whole panel.
#[test]
fn a_revealed_card_s_preview_opens_beside_the_card_not_over_it() {
    let card = Rect::from_center_size(Vec2::new(300.0, 400.0), Vec2::new(120.0, 168.0));
    let left_of_the_preview = |box_of_the_card: Option<Rect>| {
        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        let mut app = bar_of(duel);
        app.update();
        let span = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Duel>().hovered_log =
            Some(log_hover(span, card.center(), box_of_the_card));
        app.update();
        let mut q = app
            .world_mut()
            .query_filtered::<&Node, With<super::super::PreviewBounds>>();
        let panels: Vec<&Node> = q.iter(app.world()).collect();
        assert_eq!(panels.len(), 1, "the link opened one preview panel");
        let (left, top) = (panels[0].left, panels[0].top);
        assert!(
            matches!(top, Val::Px(_)),
            "the panel is placed in pixels: {top:?}"
        );
        match left {
            Val::Px(x) => x,
            other => panic!("the panel's left is not in pixels: {other:?}"),
        }
    };
    let at_the_pointer = left_of_the_preview(None);
    assert!(
        at_the_pointer < card.max.x,
        "the control: beside the pointer the panel starts over the card ({at_the_pointer} vs {})",
        card.max.x
    );
    let beside_the_card = left_of_the_preview(Some(card));
    assert!(
        beside_the_card >= card.max.x,
        "the panel starts at {beside_the_card}, over the card that ends at {}",
        card.max.x
    );
}

/// The room the cards beside a preview leave on the roomier flank, and the
/// height the window leaves them, written out as the rule says it and not
/// as `beside_width` computes it.
fn beside_room(columns: f32, rows: f32, taken: Rect, window: Vec2) -> (f32, f32) {
    let flank = (window.x - taken.max.x).max(taken.min.x) - 8.0;
    let across = (flank - (columns - 1.0) * 4.0) / columns;
    let tall = (window.y - crate::hud::TOP_CLEAR - CAPTION_H - 8.0 - (rows - 1.0) * 4.0) / rows;
    (across, tall * 63.0 / 88.0)
}

/// Report 29: the cards beside a preview were a third of it, and the pointer
/// cannot reach them to ask for more. On a wide, tall window a full-size
/// preview gets its neighbours at a revealed card's width and no wider.
#[test]
fn the_cards_beside_a_wide_preview_stop_at_a_revealed_cards_width() {
    let window = Vec2::new(1920.0, 1080.0);
    let taken = Rect::new(700.0, 100.0, 1000.0, 600.0);
    let (across, height) = beside_room(1.0, 1.0, taken, window);
    assert!(
        across > 240.0 && height > 240.0,
        "the premise: neither the flank nor the height is what binds here"
    );
    assert!(
        (beside_width(384.0, (1, 1), taken, window) - crate::hud::revealed::WIDEST).abs() < 1e-3,
        "two thirds of 384 is more than a revealed card is, so the revealed card's width wins"
    );
}

/// A small preview keeps its neighbours smaller than the revealed cap: they
/// are drawn from the preview's own size, two thirds of it, not at 240.
#[test]
fn the_cards_beside_a_small_preview_are_two_thirds_of_it() {
    let window = Vec2::new(1920.0, 1080.0);
    let taken = Rect::new(700.0, 100.0, 850.0, 300.0);
    assert!(
        (beside_width(150.0, (1, 1), taken, window) - 100.0).abs() < 1e-3,
        "two thirds of 150"
    );
    assert!(
        (beside_width(210.0, (2, 2), taken, window) - 140.0).abs() < 1e-3,
        "two thirds of 210, with a flank and a window to spare for four of them"
    );
}

/// The flank beside the preview binds before the preview does: the more
/// columns, the narrower each, and the roomier of the two flanks is the one
/// that counts.
#[test]
fn a_narrow_flank_makes_the_cards_beside_a_preview_narrower() {
    let window = Vec2::new(600.0, 1080.0);
    // 200 either side: the preview is in the middle.
    let taken = Rect::new(200.0, 100.0, 400.0, 600.0);
    let one = beside_width(384.0, (1, 1), taken, window);
    assert!(
        (one - (200.0 - 8.0)).abs() < 1e-3,
        "one column takes the flank less the gaps: {one}"
    );
    let two = beside_width(384.0, (2, 1), taken, window);
    assert!(
        (two - (200.0 - 8.0 - 4.0) / 2.0).abs() < 1e-3,
        "two columns share it: {two}"
    );
    // Roomier on the left, and the left is the one used.
    let lopsided = Rect::new(500.0, 100.0, 580.0, 600.0);
    let left = beside_width(384.0, (1, 1), lopsided, window);
    assert!(
        (left - crate::hud::revealed::WIDEST).abs() < 1e-3,
        "500 to the left is room for a revealed card, and 20 on the right is not asked: {left}"
    );
}

/// The window's height binds too: two rows of cards must fit under the top
/// clearance with their caption, whatever the flank would allow.
#[test]
fn a_short_window_makes_the_cards_beside_a_preview_smaller() {
    let window = Vec2::new(1920.0, 380.0);
    let taken = Rect::new(700.0, 100.0, 1000.0, 360.0);
    let (across, height) = beside_room(1.0, 2.0, taken, window);
    assert!(
        height < across && height < 240.0,
        "the premise: it is the height that binds ({height} vs {across})"
    );
    let two_rows = beside_width(384.0, (1, 2), taken, window);
    assert!(
        (two_rows - height).abs() < 1e-3,
        "two rows share the height: {two_rows} vs {height}"
    );
    let one_row = beside_width(384.0, (1, 1), taken, window);
    assert!(
        one_row > two_rows,
        "one row has the height to itself ({one_row} vs {two_rows})"
    );
}

/// However little room there is, the cards keep the old floor: a card
/// smaller than that is a smudge.
#[test]
fn the_cards_beside_a_preview_are_never_below_the_floor() {
    let window = Vec2::new(300.0, 200.0);
    let taken = Rect::new(60.0, 20.0, 240.0, 190.0);
    assert!(
        (beside_width(384.0, (3, 2), taken, window) - 56.0).abs() < 1e-3,
        "no room at all"
    );
    assert!(
        (beside_width(30.0, (1, 1), taken, window) - 56.0).abs() < 1e-3,
        "a preview too small to take two thirds of"
    );
}

/// What the attached cards beside a hovered host measure, against the
/// preview they stand beside: `(preview width, widths of the attached
/// cards)`.
fn attached_beside_preview(host: ObjectId, view: &PlayerView) -> (f32, Vec<f32>) {
    let mut duel = duel_with(false);
    duel.view = Some(view.clone());
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    crate::rebuild_board(&mut duel);
    duel.hovered = Some(host);
    let mut app = bar_of(duel);
    app.update();
    let px_of = |val: Val| match val {
        Val::Px(w) => w,
        other => panic!("a width that is not in pixels: {other:?}"),
    };
    let preview: Vec<f32> = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Node, With<crate::flip::Flip>>();
        q.iter(app.world()).map(|node| px_of(node.width)).collect()
    };
    assert_eq!(
        preview.len(),
        1,
        "the hover drew one preview to compare against: {preview:?}"
    );
    let children = |app: &App, of: Entity| -> Vec<Entity> {
        app.world()
            .get::<Children>(of)
            .map(|c| c.iter().collect())
            .unwrap_or_default()
    };
    let mut panels = app
        .world_mut()
        .query_filtered::<Entity, With<PreviewAttached>>();
    let panels: Vec<Entity> = panels.iter(app.world()).collect();
    let widths: Vec<f32> = panels
        .iter()
        .flat_map(|&panel| children(&app, panel).into_iter().skip(1))
        .flat_map(|grid| children(&app, grid))
        .flat_map(|column| children(&app, column))
        .map(|card| {
            px_of(
                app.world()
                    .get::<Node>(card)
                    .expect("an attached card is a node")
                    .width,
            )
        })
        .collect();
    (preview[0], widths)
}

/// Report 29: "I'd like to enlarge the card under General Tazri". The cards
/// attached to a hovered host stood beside its preview at a third of its
/// width, a picture to recognise and not a card to read. They are now
/// two-thirds of it (at most a revealed card's width), and every one of them
/// the same size.
#[test]
fn the_cards_attached_to_a_host_are_drawn_wider_than_a_third_of_its_preview() {
    use baylee_client_core::test_support::{ViewBuilder, printed, token};
    let host = ObjectId::new(5, 0);
    let mut aura = printed(6, 0, "Pacifism", 3);
    aura.types = baylee_core::types::TypeSet::ENCHANTMENT;
    aura.attached_to = Some(host);
    let mut role = token(7, 0, "Monster Role", 0, 0);
    role.types = baylee_core::types::TypeSet::ENCHANTMENT;
    role.attached_to = Some(host);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [printed(5, 0, "Grizzly Bears", 2), aura, role])
        .build();
    let (preview, widths) = attached_beside_preview(host, &view);
    assert_eq!(widths.len(), 2, "the host's two attachments: {widths:?}");
    for width in &widths {
        assert!(
            *width > preview * 0.34 + 1.0,
            "an attached card is {width} wide beside a preview {preview} wide: \
             still the old third"
        );
        assert!(
            *width <= crate::hud::revealed::WIDEST + 1e-3,
            "and no wider than a revealed card: {width}"
        );
    }
    assert!(
        (widths[0] - widths[1]).abs() < 1e-3,
        "the attached cards are drawn at one size: {widths:?}"
    );
}

/// The preview the duel overlay builds for `name` hovered on the table with
/// its art arrived: the frame's reading, and whether it holds a back face.
fn preview_of(name: &str) -> (crate::flip::Reading, bool) {
    let (view, id) = hovering(name);
    let mut duel = duel_with(false);
    duel.view = Some(view);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    crate::rebuild_board(&mut duel);
    duel.hovered = id;
    let mut app = bar_of(duel);
    // The scan of print 1, face 0 at the size the preview asks for: without
    // it the preview draws a text face, which is upright by design.
    app.world_mut().resource_mut::<CardTextures>().mark_arrived(
        baylee_client_core::images::ImageKey::new(
            baylee_core::ids::PrintRef::new(1),
            0,
            baylee_client_core::images::ArtSize::Normal,
        ),
    );
    app.update();
    let mut frames = app.world_mut().query::<(Entity, &crate::flip::Flip)>();
    let found: Vec<(Entity, crate::flip::Reading)> = frames
        .iter(app.world())
        .map(|(frame, flip)| (frame, flip.reading))
        .collect();
    assert_eq!(found.len(), 1, "the hover drew one preview: {found:?}");
    let (frame, reading) = found[0];
    let has_back = app.world().get::<Children>(frame).is_some_and(|children| {
        children.iter().any(|child| {
            app.world().get::<crate::flip::Side>(child) == Some(&crate::flip::Side::Back)
        })
    });
    (reading, has_back)
}

/// Fire // Ice is printed sideways: its preview lies across its box, shows
/// both halves on the one scan, and has no back to turn to.
#[test]
fn a_split_card_previews_lying_across_with_no_back() {
    let (reading, has_back) = preview_of("Fire // Ice");
    assert!(reading.lies_across(), "{reading:?}");
    assert!(!reading.over, "shift has no back to turn a split card to");
    assert!(!has_back, "a split card shows no back");
}

/// An ordinary card previews upright and turns over to its printed back.
#[test]
fn an_ordinary_card_previews_upright_with_a_back() {
    let (reading, has_back) = preview_of("Lightning Bolt");
    assert_eq!(reading, crate::flip::Reading::UPRIGHT);
    assert!(has_back, "an ordinary card turns over to the printed back");
}

/// The reading is the pool's, face by face: Fire // Ice lies across on both
/// halves, a battle on its front only, and a card the pool does not know
/// (or no card at all) is upright.
#[test]
fn the_hovered_card_names_its_reading() {
    let identity = |name: &str, face: u8| {
        let index = baylee_cards::decks::by_name(name).expect("in the pool");
        baylee_view::CardIdentity {
            index,
            print: baylee_core::ids::PrintRef::new(1),
            face,
        }
    };
    for face in [0, 1] {
        let read = crate::hud::overlay::preview::reading_of(Some(identity("Fire // Ice", face)));
        assert!(read.lies_across() && !read.over, "face {face}: {read:?}");
    }
    let front = crate::hud::overlay::preview::reading_of(Some(identity("Invasion of Ikoria", 0)));
    assert!(front.lies_across() && front.over, "{front:?}");
    let back = crate::hud::overlay::preview::reading_of(Some(identity("Invasion of Ikoria", 1)));
    assert!(!back.lies_across() && back.over, "{back:?}");
    assert_eq!(
        crate::hud::overlay::preview::reading_of(None),
        crate::flip::Reading::UPRIGHT
    );
}
