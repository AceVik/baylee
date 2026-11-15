use super::head::text_button;
use super::*;
use crate::ui_button::testing;
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};
use bevy::ui::{InteractionDisabled, Pressed};

fn app() -> App {
    let mut app = testing::app();
    app.init_resource::<SocialHost>()
        .init_resource::<ChatUi>()
        .add_systems(Update, presses);
    app
}

fn control(app: &mut App, press: ChatPress) -> Entity {
    let entity = text_button(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "test control",
        press,
        &LOBBY_INKS,
    );
    app.world_mut().flush();
    entity
}

#[test]
fn nested_window_buttons_only_toggle_the_inner_action_once() {
    let mut app = app();
    let outer = control(&mut app, ChatPress::Friends);
    let inner = control(&mut app, ChatPress::Gear);
    app.world_mut().entity_mut(outer).add_child(inner);
    // A pickable icon inside the button must resolve to the inner control.
    let icon = app
        .world_mut()
        .spawn((Node::default(), ChildOf(inner)))
        .id();
    testing::click(&mut app, icon);
    let ui = app.world().resource::<ChatUi>();
    assert_eq!(ui.filters, Some(ui.tab));
    assert!(!ui.drawer, "the outer button did not also toggle");
    assert_eq!(app.world().resource::<testing::Activations>().0, [inner]);
    assert!(app.world().get::<Pressed>(inner).is_none());
    assert!(app.world().get::<Pressed>(outer).is_none());
    testing::click(&mut app, icon);
    assert!(app.world().resource::<ChatUi>().filters.is_none());
    assert!(!app.world().resource::<ChatUi>().drawer);
}

#[test]
fn a_disabled_window_button_blocks_its_descendants_and_recovers_on_remove() {
    let mut app = app();
    let gear = control(&mut app, ChatPress::Gear);
    let icon = app.world_mut().spawn((Node::default(), ChildOf(gear))).id();
    app.world_mut().entity_mut(gear).insert(InteractionDisabled);
    testing::click(&mut app, icon);
    assert!(app.world().resource::<ChatUi>().filters.is_none());
    assert!(app.world().resource::<testing::Activations>().0.is_empty());
    assert!(app.world().get::<Pressed>(gear).is_none());
    app.world_mut()
        .entity_mut(gear)
        .remove::<InteractionDisabled>();
    testing::click(&mut app, icon);
    let ui = app.world().resource::<ChatUi>();
    assert_eq!(ui.filters, Some(ui.tab));
    assert_eq!(app.world().resource::<testing::Activations>().0, [gear]);
}

#[test]
fn secondary_activate_is_ignored_but_touch_toggles_once_on_release() {
    let mut app = app();
    let gear = control(&mut app, ChatPress::Gear);
    for action in [
        PointerAction::Press(PointerButton::Secondary),
        PointerAction::Release(PointerButton::Secondary),
    ] {
        testing::frame(&mut app, PointerId::Mouse, Some(gear), &[action]);
        assert!(app.world().resource::<ChatUi>().filters.is_none());
    }
    assert_eq!(app.world().resource::<testing::Activations>().0, [gear]);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(gear),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().resource::<ChatUi>().filters.is_none());
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    let ui = app.world().resource::<ChatUi>();
    assert_eq!(ui.filters, Some(ui.tab));
    app.update();
    assert!(
        app.world().resource::<ChatUi>().filters.is_some(),
        "no delayed second toggle"
    );
    assert!(app.world().get::<Pressed>(gear).is_none());
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [gear, gear]
    );
}

#[test]
fn the_scrollbar_appears_only_when_the_list_overflows() {
    let mut app = App::new();
    app.add_systems(Update, show_scrollbars_only_on_overflow);
    let list = app
        .world_mut()
        .spawn((
            LogList,
            ComputedNode {
                size: Vec2::new(300.0, 100.0),
                content_size: Vec2::new(300.0, 100.0),
                ..default()
            },
        ))
        .id();
    let track = app
        .world_mut()
        .spawn((
            ChatScrollbar,
            Node::default(),
            Visibility::Hidden,
            ChildOf(list),
        ))
        .id();
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(track).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        app.world().get::<Node>(track).unwrap().display,
        Display::Flex
    );

    app.world_mut()
        .get_mut::<ComputedNode>(list)
        .unwrap()
        .content_size = Vec2::new(300.0, 101.0);
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(track).unwrap(),
        Visibility::Inherited
    );
    assert_eq!(
        app.world().get::<Node>(track).unwrap().display,
        Display::Flex
    );

    let tick = app.world().read_change_tick();
    app.update();
    let now = app.world().read_change_tick();
    assert!(
        !app.world()
            .entity(track)
            .get_ref::<Visibility>()
            .unwrap()
            .last_changed()
            .is_newer_than(tick, now),
        "resting scrollbar must not mark Visibility changed"
    );

    app.world_mut()
        .get_mut::<ComputedNode>(list)
        .unwrap()
        .content_size = Vec2::new(300.0, 80.0);
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(track).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn stored_window_sizes_are_clamped_to_the_window_and_zoom() {
    use crate::settings::ChatWindowSize;
    let stored = ChatWindowSize {
        width: 900.0,
        height: 700.0,
    };
    let size = lobby_panel_size(Some(stored), Vec2::new(800.0, 600.0), 2.0);
    assert_eq!(
        size,
        ChatWindowSize {
            width: 388.0,
            height: 288.0,
        }
    );
    let tiny = lobby_panel_size(
        Some(ChatWindowSize {
            width: 10.0,
            height: f32::NAN,
        }),
        Vec2::new(800.0, 600.0),
        1.0,
    );
    assert_eq!(tiny.width, 320.0);
    assert_eq!(tiny.height, 320.0);
    assert!(tiny.width.is_finite() && tiny.height.is_finite());
}

#[test]
fn the_composer_puts_the_card_and_player_hint_in_its_placeholder() {
    let me = baylee_client_core::social::Me {
        tag: "af03".into(),
        handle: "Alice#af03".into(),
        guest: false,
        gateway: "http://127.0.0.1:1".into(),
    };
    let (machine, _) = Social::start(
        me,
        None,
        None,
        baylee_client_core::social::ChatPrefs::default(),
        1_000,
        &mut crate::social::OsEntropy,
    );
    let mut app = App::new();
    let kids = composer::composer_parts(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        Lang::De,
        &machine,
        &ChatUi::default(),
        &[],
        0,
        Frame::Lobby,
    );
    app.world_mut().flush();
    let row = app.world().get::<Children>(kids[0]).expect("composer row");
    for entity in row {
        if let Some(text) = app.world().get::<Text>(*entity) {
            assert!(!text.0.contains("# Karte"), "a separate hint remains");
        }
    }
    let mut spans = app.world_mut().query::<&TextSpan>();
    let placeholder = spans
        .iter(app.world())
        .map(|span| span.0.as_str())
        .find(|text| text.contains("# Karte"))
        .expect("the placeholder carries the hint");
    assert!(placeholder.contains("@ Spieler"));
    let write = baylee_client_core::social::ChatPhrase::TypeWhisper.text(Lang::De);
    assert!(
        placeholder.contains(write.trim_end_matches('…')),
        "{placeholder:?}"
    );
    // The composer offers nothing that locks the chat again (the owner,
    // 12.11.2026: "Security Zeugs darf UX nicht verschlechtern").
    let mut vault = app.world_mut().query::<&crate::social::vault_ui::Press>();
    assert_eq!(vault.iter(app.world()).count(), 0);
}

/// The resize corner is a real target above the composer's shadowed surface.
#[test]
fn the_resize_corner_stays_above_the_composer() {
    let mut app = App::new();
    let ids = skeleton(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        Frame::Lobby,
        Lang::En,
    );
    let handle = resize::handle(&mut app.world_mut().commands());
    app.world_mut().flush();
    let composer = app.world().get::<ZIndex>(ids.composer).unwrap().0;
    let corner = app.world().get::<ZIndex>(handle).unwrap().0;
    assert!(corner > composer, "{corner} must exceed {composer}");
    assert_eq!(
        app.world().get::<Node>(ids.head).unwrap().padding.right,
        zpx(3.0 + resize::HANDLE),
        "the header reserves the resize corner beside Close"
    );
}

fn identity(index: u32) -> baylee_view::CardIdentity {
    baylee_view::CardIdentity {
        index: baylee_core::ids::CardIndex::new(index),
        print: baylee_core::ids::PrintRef::new(index as u16),
        face: 0,
    }
}

fn card_link(index: u32) -> rows::ChatCardLink {
    rows::ChatCardLink {
        card: index,
        face: 0,
        print: None,
    }
}

#[test]
fn chat_previews_use_only_hand_looking_at_and_public_identities() {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.hand = vec![baylee_view::HandObject {
        id: baylee_core::ids::ObjectId::new(50, 0),
        card: identity(7),
        name: "Mine".into(),
        mana_value: 1,
        colors: baylee_core::color::ColorSet::EMPTY,
        types: baylee_core::types::TypeSet::EMPTY,
        commander: false,
    }];
    view.looking_at = vec![baylee_client_core::test_support::printed(8, 0, "Shown", 8)];
    view.battlefield = vec![baylee_client_core::test_support::printed(9, 1, "Public", 9)];
    let shared = baylee_view::SharedHand {
        player: baylee_core::ids::PlayerId::new(1),
        public: false,
        cards: vec![baylee_view::HandObject {
            card: identity(10),
            ..view.hand[0].clone()
        }],
    };
    view.shared_hands = vec![shared.clone()];
    view.controlled_hands = vec![shared];
    assert_eq!(preview::in_view(&view, &card_link(7)), Some(identity(7)));
    assert_eq!(preview::in_view(&view, &card_link(8)), Some(identity(8)));
    assert_eq!(preview::in_view(&view, &card_link(9)), Some(identity(9)));
    assert_eq!(
        preview::in_view(&view, &card_link(10)),
        Some(identity(10)),
        "a hand the view explicitly shows is visible too"
    );
    assert_eq!(preview::in_view(&view, &card_link(11)), None);
}

#[test]
fn a_known_log_card_previews_with_its_printing_and_an_unknown_one_does_not() {
    let line = baylee_client_core::gamelog::LogLine {
        index: 0,
        turn: 1,
        times: 1,
        header: false,
        at: 0,
        text: "A known and an unknown card".into(),
        names: vec![
            baylee_client_core::gamelog::NameSpan {
                range: 2..7,
                id: baylee_core::ids::ObjectId::new(50, 0),
                card: Some(identity(11)),
                token: None,
            },
            baylee_client_core::gamelog::NameSpan {
                range: 20..27,
                id: baylee_core::ids::ObjectId::new(51, 0),
                card: None,
                token: None,
            },
        ],
        players: Vec::new(),
        ability: None,
        subject: None,
        crowned: false,
        phase: None,
        follows: false,
    };
    assert_eq!(
        preview::in_log(std::slice::from_ref(&line), &card_link(11)),
        Some(identity(11))
    );
    assert_eq!(preview::in_log(&[line], &card_link(12)), None);
}

#[test]
fn a_missing_picture_uses_small_art_or_the_text_face() {
    let mut images = Assets::<Image>::default();
    let mut textures = crate::textures::CardTextures::new(&mut images, 1 << 20);
    let card = identity(7);
    assert_eq!(preview::stopgap(&textures, card, false), None);
    let small =
        baylee_client_core::images::ImageKey::of(card, baylee_client_core::images::ArtSize::Small);
    textures.mark_arrived(small);
    assert_eq!(preview::stopgap(&textures, card, false), Some(small));
    let normal =
        baylee_client_core::images::ImageKey::of(card, baylee_client_core::images::ArtSize::Normal);
    textures.mark_arrived(normal);
    assert_eq!(preview::stopgap(&textures, card, false), Some(normal));
    assert_eq!(preview::stopgap(&textures, card, true), None);
}

#[test]
fn a_links_print_is_used_only_when_the_print_table_knows_it() {
    let mut statics = baylee_client_core::test_support::statics(8);
    statics.prints[4] = Some(baylee_view::PrintEntry {
        scryfall_id: "chosen-print".into(),
        lang: "EN".into(),
        finish: baylee_view::Finish::Normal,
    });
    let mut link = card_link(7);
    link.print = Some(baylee_client_core::social::BodyPrint {
        scryfall_id: "chosen-print".into(),
        lang: "en".into(),
    });
    assert_eq!(
        preview::with_requested_print(Some(&statics), &link, identity(7)),
        baylee_view::CardIdentity {
            print: baylee_core::ids::PrintRef::new(4),
            ..identity(7)
        }
    );
    link.print.as_mut().unwrap().scryfall_id = "not-in-the-table".into();
    assert_eq!(
        preview::with_requested_print(Some(&statics), &link, identity(7)),
        identity(7)
    );
}

/// Bevy's `RelativeCursorPosition` is centred: a press a quarter in from the
/// left edge is a quarter across, not the field's start.
#[test]
fn a_press_in_the_composer_lands_where_it_is_across_the_box() {
    let at = |x: f32| {
        composer::across(&bevy::ui::RelativeCursorPosition {
            cursor_over: true,
            normalized: Some(bevy::math::Vec2::new(x, 0.0)),
        })
    };
    assert!(at(-0.5).abs() < f32::EPSILON);
    assert!((at(-0.25) - 0.25).abs() < f32::EPSILON);
    assert!((at(0.0) - 0.5).abs() < f32::EPSILON);
    assert!((at(0.5) - 1.0).abs() < f32::EPSILON);
    assert!(
        (at(0.9) - 1.0).abs() < f32::EPSILON,
        "clamped past the edge"
    );
    assert!(composer::across(&bevy::ui::RelativeCursorPosition::default()).abs() < f32::EPSILON);
}

/// The window is glass: the room behind it comes through, so its ground is
/// less than the kit's panel. What comes through must not take the lines:
/// over the tavern fire's orange, blended in linear light as the UI is, the
/// quiet ink keeps WCAG's 4.5:1 at the head and at the foot. The flat 0.66
/// this replaced gave 2.5:1, which the last assertion holds on to.
#[test]
fn the_window_is_glass_and_its_quiet_ink_still_reads_over_the_fire() {
    let fire = Color::srgb(0.90, 0.50, 0.15).to_linear();
    let luminance = |c: LinearRgba| 0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue;
    let reads = |glass: Color| {
        let over = glass.to_linear();
        let share = over.alpha;
        let under = LinearRgba::rgb(
            over.red * share + fire.red * (1.0 - share),
            over.green * share + fire.green * (1.0 - share),
            over.blue * share + fire.blue * (1.0 - share),
        );
        (luminance(LOBBY_INKS.soft.to_linear()) + 0.05) / (luminance(under) + 0.05)
    };
    for glass in rows::window_glass() {
        assert!(reads(glass) >= 4.5, "{glass:?}: {}", reads(glass));
        assert!(glass.alpha() <= 0.92, "{glass:?}");
        assert!(glass.alpha() < crate::shellkit::tokens::PANEL.alpha());
    }
    assert!(reads(LOBBY_INKS.ground) < 3.0);

    let mut app = app();
    let fonts = UiFonts::default();
    let pill = chat_pill(&mut app.world_mut().commands(), &fonts, Lang::En, 2, false);
    app.world_mut().flush();
    assert_eq!(
        app.world().get::<BackgroundColor>(pill).unwrap().0,
        rows::PILL_GLASS
    );
    assert!(
        app.world()
            .get::<bevy::ui::BackgroundGradient>(pill)
            .is_none(),
        "chat_pill must not cover BackgroundColor with a gradient so Feel hover shows"
    );

    app.insert_resource(UiFonts::default())
        .init_resource::<Windows>()
        .init_resource::<crate::social::zoom::Zoom>()
        .init_resource::<LobbyChatSizing>()
        .insert_resource(State::new(DuelPhase::Playing))
        .insert_resource(SocialHost {
            machine: Some(
                Social::start(
                    baylee_client_core::social::Me {
                        tag: "AB12".into(),
                        handle: "Alice#AB12".into(),
                        guest: false,
                        gateway: "https://example.test".into(),
                    },
                    None,
                    None,
                    baylee_client_core::social::ChatPrefs::default(),
                    1_000,
                    &mut crate::social::OsEntropy,
                )
                .0,
            ),
            ..default()
        })
        .add_systems(Update, sync_lobby_window);
    app.world_mut().resource_mut::<ChatUi>().open = true;
    app.update();
    let chat_root = app
        .world_mut()
        .query_filtered::<Entity, With<LobbyChat>>()
        .single(app.world())
        .expect("LobbyChat spawned");
    assert_eq!(
        app.world().get::<BackgroundColor>(chat_root).unwrap().0,
        Color::NONE,
        "LobbyChat must not double-composite BackgroundColor under window_ground()"
    );
}

#[test]
fn head_parts_omits_friends_button_for_guest_sessions() {
    for guest in [false, true] {
        let mut app = app();
        let fonts = UiFonts::default();
        let _ = head::head_parts(
            &mut app.world_mut().commands(),
            &fonts,
            Lang::En,
            &[(ChatTab::Table, 0, false)],
            ChatTab::Table,
            Frame::Lobby,
            guest,
        );
        app.world_mut().flush();
        let mut q = app.world_mut().query::<&ChatPress>();
        let has_friends = q.iter(app.world()).any(|p| *p == ChatPress::Friends);
        assert_eq!(has_friends, !guest, "guest={guest}");
    }
}

#[test]
fn filter_dialog_distributes_reset_and_done_symmetrically() {
    use baylee_client_core::social::{ChatPrefs, Me};

    let (machine, _) = Social::start(
        Me {
            tag: "AB12".into(),
            handle: "Alice#AB12".into(),
            guest: false,
            gateway: "https://example.test".into(),
        },
        None,
        None,
        ChatPrefs::default(),
        1_000,
        &mut crate::social::OsEntropy,
    );
    let mut app = app();
    app.insert_resource(SocialHost {
        machine: Some(machine),
        ..default()
    })
    .insert_resource(ChatUi {
        filters: Some(ChatTab::Table),
        ..default()
    })
    .insert_resource(UiFonts::default())
    .init_resource::<super::super::zoom::Zoom>()
    .add_systems(Update, filter::filter_dialog);
    app.update();

    let mut buttons = app
        .world_mut()
        .query::<(Entity, &ChatPress, &Node, &ChildOf)>();
    let actions: Vec<(ChatPress, Node, Entity)> = buttons
        .iter(app.world())
        .filter(|(_, p, _, _)| matches!(p, ChatPress::FilterReset | ChatPress::FilterDone))
        .map(|(_, p, n, parent)| (p.clone(), n.clone(), parent.parent()))
        .collect();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0].2, actions[1].2, "shared horizontal footer row");
    assert_eq!(
        app.world().get::<Node>(actions[0].2).unwrap().width,
        Val::Percent(100.0)
    );
    for (_, node, _) in &actions {
        assert!((node.flex_grow - 1.0).abs() < f32::EPSILON);
        assert_eq!(node.flex_basis, Val::Px(0.0));
    }
}

#[test]
fn empty_composer_disables_send_with_hover_tooltip() {
    use baylee_client_core::social::{ChatPrefs, Me};

    let (mut machine, _) = Social::start(
        Me {
            tag: "AB12".into(),
            handle: "Alice#AB12".into(),
            guest: false,
            gateway: "https://example.test".into(),
        },
        None,
        None,
        ChatPrefs::default(),
        1_000,
        &mut crate::social::OsEntropy,
    );
    let mut empty_app = app();
    let fonts = UiFonts::default();
    let ui = ChatUi::default();
    let _ = composer::composer_parts(
        &mut empty_app.world_mut().commands(),
        &fonts,
        Lang::En,
        &machine,
        &ui,
        &[],
        0,
        Frame::Lobby,
    );
    empty_app.world_mut().flush();
    let has_send_press = empty_app
        .world_mut()
        .query::<&ChatPress>()
        .iter(empty_app.world())
        .any(|p| *p == ChatPress::Send);
    assert!(
        !has_send_press,
        "empty composer does not offer clickable Send"
    );
    let has_hint = empty_app
        .world_mut()
        .query::<(&crate::lobby::HoverHint, &crate::hud::Hint)>()
        .iter(empty_app.world())
        .any(|(lobby_h, hud_h)| !lobby_h.0.is_empty() && lobby_h.0 == hud_h.0);
    assert!(has_hint, "disabled Send exposes a hover tooltip");

    machine.composer_mut().typed("gg");
    let mut ready_app = app();
    let _ = composer::composer_parts(
        &mut ready_app.world_mut().commands(),
        &fonts,
        Lang::En,
        &machine,
        &ui,
        &[],
        0,
        Frame::Lobby,
    );
    ready_app.world_mut().flush();
    let has_send_press = ready_app
        .world_mut()
        .query::<(&ChatPress, &UiButton)>()
        .iter(ready_app.world())
        .any(|(p, _)| *p == ChatPress::Send);
    assert!(has_send_press, "non-empty composer offers clickable Send");
}
