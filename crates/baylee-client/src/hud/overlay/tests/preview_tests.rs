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
