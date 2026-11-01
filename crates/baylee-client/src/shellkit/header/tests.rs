//! Native desktop header overflow, measured by Bevy with the shipped fonts.

use super::*;
use crate::hud::UiFonts;
use crate::shellkit::focus::Stop;
use crate::shellkit::{ShellMetrics, TextSize, Viewport};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::replay::words::ReplayPhrase;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Nav(u8),
    Gateway,
    Gear,
    Chat,
    Friends,
    Bell,
    Account,
    Back,
}

impl Action {
    fn stop(self) -> Stop {
        let id = match self {
            Self::Nav(n) => return Stop::item("header-qa", "nav", n),
            Self::Gateway => "gateway",
            Self::Gear => "gear",
            Self::Chat => "chat",
            Self::Friends => "friends",
            Self::Bell => "bell",
            Self::Account => "account",
            Self::Back => "back",
        };
        Stop::new("header-qa", id)
    }
}

const ACTIONS: [Action; 10] = [
    Action::Nav(0),
    Action::Nav(1),
    Action::Nav(2),
    Action::Nav(3),
    Action::Gateway,
    Action::Gear,
    Action::Chat,
    Action::Friends,
    Action::Bell,
    Action::Account,
];

const HANDLE: &str = "QaMica#0001";

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
        bevy::input::InputPlugin,
        bevy::picking::DefaultPickingPlugins,
    ));
    app.init_asset::<Image>()
        .init_asset::<bevy::image::TextureAtlasLayout>();
    let mut store = app.world_mut().resource_mut::<Assets<Font>>();
    let mut load = |file: &str| {
        let path = format!("{}/assets/fonts/{file}", env!("CARGO_MANIFEST_DIR"));
        store.add(Font::from_bytes(std::fs::read(path).expect(file)))
    };
    let fonts = UiFonts {
        text: load("AlegreyaSans-Regular.ttf"),
        medium: load("AlegreyaSans-Medium.ttf"),
        bold: load("AlegreyaSans-Bold.ttf"),
        icons: load("fa-solid-900.ttf"),
        ..default()
    };
    (app, fonts)
}

/// The lobby's three tools, including chat's word and both social counts.
fn tool(
    commands: &mut Commands,
    kit: Kit,
    action: Action,
    glyph: char,
    word: Option<&str>,
    count: bool,
) -> Entity {
    let face = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: kit.m.px(6.0),
            ..default()
        })
        .id();
    let icon = commands
        .spawn((Text::new(glyph.to_string()), icon_tf(kit.fonts, kit.m.text)))
        .id();
    commands.entity(face).add_child(icon);
    if let Some(word) = word {
        let words = label(commands, kit, word, kit.m.text, tokens::INK);
        commands.entity(face).add_child(words);
    }
    if count {
        let number = commands
            .spawn((Text::new("0"), tf_bold(kit.fonts, kit.m.text)))
            .id();
        commands.entity(face).add_child(number);
    }
    hit(commands, kit, face, (action, action.stop()))
}

fn draw(
    app: &mut App,
    fonts: &UiFonts,
    width: f32,
    step: TextSize,
    lang: Lang,
    gateway: &str,
) -> Entity {
    let kit = Kit {
        fonts,
        m: ShellMetrics::of(Viewport::desktop(width, 800.0), step),
        german: lang == Lang::De,
    };
    let nav = [
        Phrase::ShellPlay.text(lang),
        Phrase::ShellDecks.text(lang),
        ReplayPhrase::NavReplays.text(lang),
        Phrase::Settings.text(lang),
    ];
    for label in nav {
        assert!(
            label.chars().count() <= tokens::NAV_BUDGET.of(lang == Lang::De),
            "{lang:?}: {label}"
        );
    }
    let mut commands = app.world_mut().commands();
    let root = commands
        .spawn(Node {
            width: px_fixed(width),
            height: px_fixed(800.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let tools = [
        tool(&mut commands, kit, Action::Gear, '\u{f013}', None, false),
        tool(
            &mut commands,
            kit,
            Action::Chat,
            crate::hud::glyph::CHAT,
            Some(Phrase::ShellChat.text(lang)),
            true,
        ),
        tool(
            &mut commands,
            kit,
            Action::Friends,
            crate::hud::glyph::FRIENDS,
            None,
            true,
        ),
    ];
    let bar = header(
        &mut commands,
        kit,
        &HeaderLook {
            brand: "Baylee",
            build: "0.1.0-beta.8",
            nav: &nav,
            active: Some(3),
            reach: Reach::Up,
            gateway,
            counts: if lang == Lang::De {
                "0 Tische · 2 online"
            } else {
                "0 tables · 2 online"
            },
            unread: 0,
            handle: Some(HANDLE),
            return_pill: None,
            tools: &tools,
        },
        HeaderActions {
            nav: |i| {
                let action = Action::Nav(u8::try_from(i).expect("four tabs"));
                (action, action.stop())
            },
            gateway: (Action::Gateway, Action::Gateway.stop()),
            bell: (Action::Bell, Action::Bell.stop()),
            account: (Action::Account, Action::Account.stop()),
            back: (Action::Back, Action::Back.stop()),
        },
    );
    commands.entity(root).add_child(bar);
    app.world_mut().flush();
    app.update();
    app.update();
    root
}

pub(crate) fn bounds(world: &World, entity: Entity) -> Rect {
    let computed = world.get::<ComputedNode>(entity).expect("laid out");
    let (_, _, centre) = world
        .get::<UiGlobalTransform>(entity)
        .expect("UI transform")
        .to_scale_angle_translation();
    Rect::from_center_size(centre, computed.size())
}

fn assert_controls_fit(app: &mut App, root: Entity, width: f32, step: TextSize) {
    let viewport = bounds(app.world(), root);
    assert!((viewport.width() - width).abs() < 0.5, "{viewport:?}");
    let m = ShellMetrics::of(Viewport::desktop(width, 800.0), step);
    let mut controls = app.world_mut().query::<(Entity, &Action, &Stop, &Role)>();
    let drawn: Vec<_> = controls.iter(app.world()).collect();
    assert_eq!(drawn.len(), ACTIONS.len());
    let mut right = viewport.min.x + m.body;
    for action in ACTIONS {
        let &(entity, _, stop, role) = drawn
            .iter()
            .find(|(_, got, _, _)| **got == action)
            .expect("every action still exists");
        assert_eq!(
            *stop,
            action.stop(),
            "{action:?}: focus stays on its action"
        );
        assert_eq!(*role, Role::Hit);
        let rect = bounds(app.world(), entity);
        assert!(
            rect.width() >= m.hit - 0.5 && rect.height() >= m.hit - 0.5,
            "{action:?}: {rect:?}"
        );
        assert!(
            rect.min.x >= right - 0.5,
            "{action:?} overlaps the preceding action: {rect:?}"
        );
        assert!(
            rect.max.x <= viewport.max.x - m.body + 0.5,
            "{action:?} overflows {width}: {rect:?}"
        );
        assert!(
            rect.min.y >= viewport.min.y && rect.max.y <= viewport.min.y + m.header + 0.5,
            "{action:?}: {rect:?}"
        );
        right = rect.max.x;
    }
    let mut labels = app
        .world_mut()
        .query::<(&Text, &bevy::text::TextLayoutInfo)>();
    let (_, layout) = labels
        .iter(app.world())
        .find(|(text, _)| text.0 == HANDLE)
        .expect("the full account handle stays drawn");
    assert!(
        !layout.glyphs.is_empty(),
        "the test must measure real glyphs"
    );
}

/// The reported 13-pixel account overflow: four tabs, full handle, all
/// social tools, gateway status and a localhost URL, at default text size.
#[test]
fn signed_in_desktop_header_keeps_every_control_inside_1280() {
    let (mut app, fonts) = layout_app();
    let root = draw(
        &mut app,
        &fonts,
        1280.0,
        TextSize::M,
        Lang::En,
        "http://127.0.0.1:21888",
    );
    assert_controls_fit(&mut app, root, 1280.0, TextSize::M);
}

/// Address length cannot move the account off screen. Localisation and
/// text size still leave all ten actions and their focus stops reachable.
#[test]
fn long_gateway_labels_yield_to_actions_across_desktop_sizes() {
    let (mut app, fonts) = layout_app();
    let gateway = format!("https://localhost:21888/{}", "gateway-address/".repeat(16));
    for lang in [Lang::En, Lang::De] {
        for step in TextSize::ALL {
            let width = 1280.0 * step.factor();
            let root = draw(&mut app, &fonts, width, step, lang, &gateway);
            assert_controls_fit(&mut app, root, width, step);
            app.world_mut().entity_mut(root).despawn();
        }
    }
}

/// The gateway's status sits beside its name on a single line and stays
/// entirely readable at the reported viewport; its full name is retained
/// even while clipped.
#[test]
fn desktop_gateway_stacks_status_and_bounds_the_address() {
    let (mut app, fonts) = layout_app();
    let gateway = format!("https://localhost:21888/{}", "gateway-address/".repeat(16));
    draw(&mut app, &fonts, 1280.0, TextSize::M, Lang::En, &gateway);
    let mut labels = app.world_mut().query::<(Entity, &Text, &ChildOf)>();
    let (name, parent) = labels
        .iter(app.world())
        .find(|(_, text, _)| text.0 == gateway)
        .map(|(entity, _, parent)| (entity, parent.parent()))
        .expect("the complete gateway address");
    let (counts, _) = labels
        .iter(app.world())
        .find(|(_, text, _)| text.0 == "0 tables · 2 online")
        .map(|(entity, _, parent)| (entity, parent.parent()))
        .filter(|(_, p)| *p == parent)
        .expect("status shares the address row");
    let row = app.world().get::<Node>(parent).expect("gateway row");
    assert_eq!(row.flex_direction, FlexDirection::Row);
    assert_eq!(row.overflow, Overflow::clip_x());
    let name_rect = bounds(app.world(), name);
    let counts_rect = bounds(app.world(), counts);
    assert!(name_rect.max.x <= counts_rect.min.x + 0.5);
    let laid = app
        .world()
        .get::<bevy::text::TextLayoutInfo>(counts)
        .expect("status glyphs");
    assert!(
        laid.size.x <= bounds(app.world(), parent).width() + 0.5,
        "status stays readable"
    );
    let address = app
        .world()
        .get::<bevy::text::TextLayoutInfo>(name)
        .expect("address glyphs");
    assert!(
        address.size.x > bounds(app.world(), name).width(),
        "a long address is actually clipped"
    );
}

/// A resize returns space to the gateway without rebuilding or replacing
/// the account, navigation or their focus bindings.
#[test]
fn desktop_header_gives_the_gateway_space_back_on_resize() {
    let (mut app, fonts) = layout_app();
    let gateway = format!("https://localhost:21888/{}", "gateway-address/".repeat(16));
    let root = draw(&mut app, &fonts, 1280.0, TextSize::M, Lang::En, &gateway);
    let mut controls = app.world_mut().query::<(Entity, &Action)>();
    let before: Vec<_> = controls.iter(app.world()).map(|(e, a)| (e, *a)).collect();
    let gateway = before
        .iter()
        .find(|(_, a)| *a == Action::Gateway)
        .expect("gateway")
        .0;
    let narrow = bounds(app.world(), gateway).width();
    app.world_mut()
        .get_mut::<Node>(root)
        .expect("viewport")
        .width = px_fixed(1920.0);
    app.update();
    app.update();
    assert_controls_fit(&mut app, root, 1920.0, TextSize::M);
    let after: Vec<_> = controls.iter(app.world()).map(|(e, a)| (e, *a)).collect();
    assert_eq!(after, before, "resizing retains the same action entities");
    assert!(
        bounds(app.world(), gateway).width() > narrow + 500.0,
        "the gateway reclaims the extra width"
    );
}

#[test]
fn narrow_desktop_with_large_german_text_wraps_without_losing_actions() {
    let (mut app, fonts) = layout_app();
    let root = draw(&mut app, &fonts, 960.0, TextSize::Xl, Lang::De, "Local QA");
    let viewport = bounds(app.world(), root);
    let world = app.world_mut();
    let entities: Vec<_> = world
        .query::<(Entity, &Action)>()
        .iter(world)
        .map(|(e, a)| (e, *a))
        .collect();
    assert_eq!(entities.len(), ACTIONS.len());
    let rects: Vec<_> = entities
        .iter()
        .map(|(e, a)| (*a, bounds(app.world(), *e)))
        .collect();
    for (index, (action, rect)) in rects.iter().enumerate() {
        assert!(
            rect.width() >= 35.0 && rect.height() >= 35.0,
            "usable {action:?}: {rect:?}"
        );
        assert!(
            rect.min.x >= viewport.min.x && rect.max.x <= viewport.max.x + 0.5,
            "{action:?}: {rect:?}"
        );
        assert!(
            rect.min.y >= viewport.min.y && rect.max.y <= viewport.min.y + 160.0,
            "{action:?}: {rect:?}"
        );
        for (other, next) in &rects[index + 1..] {
            let overlap = rect.intersect(*next);
            assert!(
                overlap.width() <= 0.5 || overlap.height() <= 0.5,
                "{action:?} overlaps {other:?}"
            );
        }
    }
}

/// Exercise the real builder through its normal retained sections, with the
/// shipped fonts: contextual and shared header actions must remain reachable,
/// the wide account keeps its handle, and no version stands beside the brand.
#[test]
fn builder_header_keeps_contextual_actions_inside_the_shared_frame() {
    use crate::buildui::{Env, Layout};
    use crate::lobby::{
        BuildPress, FrontPress, HeaderPress, LibraryPress, LobbyState, Press, Scrolled,
    };

    let (mut app, fonts) = layout_app();
    let scrolled = Scrolled::default();
    for lang in [Lang::En, Lang::De] {
        for width in [1280.0, 1600.0, 1920.0] {
            let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
            state.lobby.set_lang(lang);
            state.chat_header = Some(true);
            state.lobby.set_me(baylee_client_core::lobby::strips::Me {
                handle: HANDLE.to_string(),
                guest: false,
            });
            for ch in "A long deck name that must yield to every navigation action".chars() {
                state.lobby.builder_mut().type_name(ch);
            }
            let m = ShellMetrics::of(Viewport::desktop(width, 800.0), TextSize::M);
            let env = Env {
                kit: Kit {
                    fonts: &fonts,
                    m,
                    german: lang == Lang::De,
                },
                state: &state,
                scrolled: &scrolled,
                layout: Layout::of(m.frame, width),
                save_keys: Some("Cmd+S"),
            };
            let mut commands = app.world_mut().commands();
            let root = commands
                .spawn(Node {
                    width: px_fixed(width),
                    height: px_fixed(800.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .id();
            let _retained = crate::buildui::builder(&mut commands, root, &env, None, None);
            app.world_mut().flush();
            app.update();
            app.update();
            let header = app
                .world_mut()
                .query::<(Entity, &HeaderSurface)>()
                .iter(app.world())
                .find(|(_, surface)| !surface.0)
                .unwrap()
                .0;
            let frame = bounds(app.world(), header);
            let mut texts = app.world_mut().query::<&Text>();
            let drawn: Vec<_> = texts
                .iter(app.world())
                .map(|text| text.0.as_str())
                .collect();
            assert!(drawn.contains(&HANDLE), "the wide account keeps its handle");
            assert!(
                drawn.iter().all(|text| *text != baylee_build::VERSION),
                "no version beside the builder's brand"
            );
            let mut previous = frame.min.x;
            for action in [
                Press::Build(BuildPress::CloseBuilder),
                Press::Build(BuildPress::OpenImport),
                Press::Build(BuildPress::OpenExport),
                Press::Library(LibraryPress::BrowseHistory),
                Press::Build(BuildPress::SaveDeck),
                Press::Build(BuildPress::ToggleHeaderMenu),
                Press::Header(HeaderPress::Gateway),
                Press::Front(FrontPress::FrontMenu),
                Press::Header(HeaderPress::Chat),
                Press::Header(HeaderPress::Friends),
                Press::Header(HeaderPress::Bell),
                Press::Header(HeaderPress::Account),
            ] {
                let entity = app
                    .world_mut()
                    .query::<(Entity, &Press)>()
                    .iter(app.world())
                    .find(|(_, p)| **p == action)
                    .expect("contextual action is still available")
                    .0;
                let rect = bounds(app.world(), entity);
                assert!(
                    rect.min.x >= previous - 0.5,
                    "{lang:?} {width} {action:?}: overlap {rect:?}"
                );
                assert!(
                    rect.max.x <= frame.max.x + 0.5,
                    "{lang:?} {width} {action:?}: outside {rect:?}"
                );
                assert!(rect.height() >= m.hit - 0.5, "hit target stays usable");
                previous = rect.max.x;
            }
            app.world_mut().entity_mut(root).despawn();
        }
    }
}
