use super::{Action, Book, Screen, api, seasonal};
use crate::{
    hud::UiFonts,
    shellkit::{
        InputClass,
        controls::{self, Kit, Live, Weight},
        focus::{Activated, Stop, TabOrder},
        surfaces::{self, SheetHeader, SheetWidth},
        tokens,
    },
    ui_button::Clicks,
};
use baylee_client_core::{Lang, Phrase};
use bevy::prelude::*;

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "advent",
    stops: &[
        "close", "calendar", "gallery", "door", "move", "hint", "reset", "submit", "claim", "item",
        "deck", "prev", "next", "retry",
    ],
    modal: true,
};
#[derive(Component)]
pub(super) struct Root;
#[derive(Component)]
pub(super) struct Body;
#[derive(Component)]
pub(super) struct Entry {
    parent: Entity,
    gallery: bool,
}
#[derive(Component)]
pub(super) struct EntryMark;

pub(in crate::lobby) fn entry(commands: &mut Commands, parent: Entity, kit: Kit) {
    let e = seasonal::plaque(commands, kit);
    let holder = commands
        .spawn((
            Entry {
                parent: e,
                gallery: false,
            },
            Visibility::Hidden,
            Node {
                display: Display::None,
                width: percent(100),
                ..default()
            },
        ))
        .id();
    commands.entity(holder).add_child(e);
    commands.entity(parent).add_child(holder);
    let e = button(
        commands,
        kit,
        Phrase::AdventGallery.text(kit.lang()),
        Action::Gallery,
        "gallery",
        0,
    );
    commands
        .entity(e)
        .insert(Stop::new(super::super::orders::PLAY.name, "advent"));
    let holder = commands
        .spawn((
            Entry {
                parent: e,
                gallery: true,
            },
            Visibility::Hidden,
            Node {
                display: Display::None,
                width: percent(100),
                ..default()
            },
        ))
        .id();
    commands.entity(holder).add_child(e);
    commands.entity(parent).add_child(holder);
}
pub(super) fn entries(
    mut commands: Commands,
    book: Res<Book>,
    mut entry: Query<(
        Entity,
        &Entry,
        &mut Node,
        &mut Visibility,
        Option<&EntryMark>,
    )>,
) {
    for (holder, e, mut node, mut visibility, mark) in &mut entry {
        let show = book.owner.is_some()
            && if e.gallery {
                !book.released() && !book.inventory.is_empty()
            } else {
                book.released()
            };
        let display = if show { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let visible = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != visible {
            *visibility = visible;
        }
        if show && !e.gallery && mark.is_none() {
            commands.entity(e.parent).insert(seasonal::ButtonGlow);
            commands.entity(holder).insert(EntryMark);
        }
    }
}

pub(super) fn actions(
    mut book: ResMut<Book>,
    mut clicks: Clicks<Action>,
    mut activated: MessageReader<Activated>,
    controls: Query<&Action>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    appearance: Option<Res<crate::lobby::deck_appearance::Page>>,
    mut equip: MessageWriter<crate::lobby::deck_appearance::EquipReward>,
) {
    let b = book.bypass_change_detection();
    if appearance.is_some_and(|p| p.holds_keys()) {
        return;
    }
    if keys.as_ref().is_none_or(|k| !k.pressed(KeyCode::Escape)) {
        b.just_closed = false;
    }
    let starting = b.revision;
    let actions = clicks.read().into_iter().chain(
        activated
            .read()
            .filter(|press| press.by_key)
            .filter_map(|k| controls.get(k.entity).ok().copied()),
    );
    for action in actions {
        b.error = None;
        match action {
            Action::Open | Action::Calendar if b.released() => {
                b.screen = Screen::Calendar;
                b.load_hero();
            }
            Action::Open | Action::Calendar => continue,
            Action::Close => b.screen = Screen::Closed,
            Action::Gallery => {
                b.screen = Screen::Gallery;
                b.inventory_page = 0;
                b.request_inventory();
            }
            Action::Item(id) => {
                b.screen = Screen::Item(id);
                b.load_owned(id);
            }
            Action::Door(n) => b.open_door(n),
            Action::Deck => {
                if let Screen::Item(item) = b.screen
                    && b.inventory.iter().any(|r| r.id == item)
                {
                    equip.write(crate::lobby::deck_appearance::EquipReward { item });
                }
            }
            Action::Prev => b.inventory_page = b.inventory_page.saturating_sub(1),
            Action::Next => b.inventory_page = b.inventory_page.saturating_add(1),
            Action::Retry => b.retry(),
            Action::Hint(_)
            | Action::Reset
            | Action::Choose(_)
            | Action::Submit
            | Action::Claim => super::puzzle::act(b, action),
        }
        b.touch();
    }
    if b.visible() && keys.is_some_and(|k| k.just_pressed(KeyCode::Escape)) {
        b.screen = Screen::Closed;
        b.just_closed = true;
        b.touch();
    }
    if b.revision != starting {
        book.set_changed();
    }
}

#[derive(Default)]
pub(super) struct Paint {
    shell: Option<(Lang, crate::shellkit::ShellMetrics, bool)>,
    body: Option<(Screen, u64)>,
    navigation: Option<(Entity, Option<bool>)>,
}
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    mut commands: Commands,
    book: Res<Book>,
    lobby: Res<super::LobbyState>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    windows: Query<&Window>,
    input: Option<Res<InputClass>>,
    mut paint: Local<Paint>,
    roots: Query<Entity, With<Root>>,
    bodies: Query<Entity, With<Body>>,
) {
    let lang = lobby.lobby.lang();
    if !book.visible() {
        if paint.shell.take().is_some() {
            for root in &roots {
                commands.entity(root).try_despawn();
            }
            paint.body = None;
            paint.navigation = None;
        }
        return;
    }
    let (w, h) = windows
        .iter()
        .next()
        .map_or((1024., 720.), |w| (w.width(), w.height()));
    let metrics = super::super::header::kit_metrics(
        w,
        h,
        settings.as_deref().map_or_default(|s| s.text_size),
        input.as_deref().copied().unwrap_or_default(),
    );
    let kit = Kit {
        fonts: &fonts,
        m: metrics,
        german: lang == Lang::De,
    };
    let cd = (book.screen, book.revision);
    let shell = (lang, metrics, book.released());
    if paint.shell.as_ref() == Some(&shell) && paint.body == Some(cd) {
        return;
    }
    let body = if paint.shell.as_ref() == Some(&shell)
        && let Some(e) = bodies.iter().next()
    {
        commands.entity(e).despawn_children();
        e
    } else {
        for root in &roots {
            commands.entity(root).try_despawn();
        }
        let (body, navigation) = sheet(&mut commands, kit, book.released());
        paint.navigation = Some((navigation, None));
        paint.shell = Some(shell);
        body
    };
    let in_gallery = matches!(book.screen, Screen::Item(_) | Screen::Gallery);
    if let Some((navigation, selected)) = paint.navigation.as_mut()
        && *selected != Some(in_gallery)
    {
        commands.entity(*navigation).despawn_children();
        let tabs = section_tabs(&mut commands, kit, book.released(), in_gallery);
        commands.entity(*navigation).add_child(tabs);
        *selected = Some(in_gallery);
    }
    match book.screen {
        Screen::Calendar => {
            super::seasonal::mosaic(&mut commands, body, kit, &book, (h - 205.).max(190.) * 1.5);
        }
        Screen::Door(n) => super::puzzle::draw(&mut commands, body, kit, &book, n),
        Screen::Gallery | Screen::Item(_) => gallery(&mut commands, body, kit, &book),
        Screen::Closed => {}
    }
    if book.error.is_some()
        || matches!(book.screen, Screen::Gallery) && book.inventory_error.is_some()
    {
        label(
            &mut commands,
            body,
            kit,
            Phrase::AdventUnavailable.text(lang),
            tokens::INK,
        );
        let e = button(
            &mut commands,
            kit,
            Phrase::AdventRetry.text(lang),
            Action::Retry,
            "retry",
            0,
        );
        commands.entity(body).add_child(e);
    }
    paint.body = Some(cd);
}

fn sheet(commands: &mut Commands, kit: Kit, released: bool) -> (Entity, Entity) {
    let lang = kit.lang();
    let body = commands
        .spawn((
            Body,
            Node {
                flex_direction: FlexDirection::Column,
                width: percent(100),
                row_gap: kit.m.px(8.),
                ..default()
            },
        ))
        .id();
    let navigation = commands
        .spawn(Node {
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .id();
    let sheet = surfaces::sheet_box_fixed(
        commands,
        kit,
        SheetWidth::Large,
        SheetHeader::new(
            if released {
                Phrase::AdventCalendar
            } else {
                Phrase::AdventGallery
            }
            .text(lang),
        )
        .icon_colored(crate::lobby::parts::STAR, tokens::GOLD)
        .close((Action::Close, Stop::new(ORDER.name, "close"))),
        &[body],
        &[navigation],
    );
    let root = surfaces::scrim(
        commands,
        kit,
        crate::social::window::G_PANEL + 86,
        (),
        sheet,
    );
    commands
        .entity(root)
        .insert((Root, DespawnOnExit(crate::DuelPhase::Closed)));
    (body, navigation)
}
fn section_tabs(commands: &mut Commands, kit: Kit, released: bool, in_gallery: bool) -> Entity {
    let lang = kit.lang();
    let tabs = [
        (Phrase::AdventDoors.text(lang), None),
        (Phrase::AdventGallery.text(lang), None),
    ];
    let start = usize::from(!released);
    let selected = usize::from(in_gallery).saturating_sub(start);
    controls::tabs(commands, kit, &tabs[start..], selected, move |index| {
        let (action, stop) = if index + start == 0 {
            (Action::Calendar, "calendar")
        } else {
            (Action::Gallery, "gallery")
        };
        (
            action,
            Stop::new(ORDER.name, stop),
            crate::shellkit::focus::Current(index == selected),
        )
    })
}

pub(super) fn button(
    commands: &mut Commands,
    kit: Kit,
    words: &str,
    action: Action,
    stop: &'static str,
    index: u8,
) -> Entity {
    controls::button(
        commands,
        kit,
        words,
        match action {
            Action::Claim | Action::Deck => Weight::Gold,
            Action::Submit => Weight::Primary,
            Action::Reset | Action::Hint(_) => Weight::Ghost,
            _ => Weight::Secondary,
        },
        Live::Yes,
        None,
        (action, Stop::item(ORDER.name, stop, index)),
    )
}
pub(super) fn label(commands: &mut Commands, parent: Entity, kit: Kit, text: &str, color: Color) {
    let e = commands
        .spawn((
            Text::new(text),
            crate::hud::tf(kit.fonts, kit.m.text),
            TextColor(color),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(e);
}
pub(super) fn tile_action(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    text: &str,
    action: Action,
    stop: &'static str,
    index: u8,
) {
    let e = button(commands, kit, text, action, stop, index);
    commands.entity(parent).add_child(e);
}
fn gallery(commands: &mut Commands, parent: Entity, kit: Kit, book: &Book) {
    let lang = kit.lang();
    if let Screen::Item(id) = book.screen {
        item_view(commands, parent, kit, book, id);
        return;
    }
    label(
        commands,
        parent,
        kit,
        Phrase::AdventGallery.text(lang),
        tokens::GOLD,
    );
    if book.inventory.is_empty() {
        label(
            commands,
            parent,
            kit,
            if book.inventory_loaded {
                Phrase::AdventEmpty
            } else {
                Phrase::AdventLoading
            }
            .text(lang),
            tokens::MUTED,
        );
        return;
    }
    let page = book.inventory_page.min((book.inventory.len() - 1) / 6);
    for (i, item) in book.inventory.iter().skip(page * 6).take(6).enumerate() {
        let title = if lang == Lang::De {
            &item.title.de
        } else {
            &item.title.en
        };
        let kind = kind_phrase(item.kind);
        let meta = format!(
            "{} · {} · {}",
            kind.text(lang),
            item.year,
            Phrase::AdventDoor.fill(lang, &[&item.door.to_string()])
        );
        let row = surfaces::list_row(
            commands,
            kit,
            title,
            &meta,
            &[],
            (
                Action::Item(item.id),
                Stop::item(ORDER.name, "item", i as u8),
            ),
        );
        commands
            .entity(row)
            .insert(crate::ambience::Feel::tinting_to(
                Color::NONE,
                tokens::HOVER,
            ));
        commands.entity(parent).add_child(row);
    }
    pages(
        commands,
        parent,
        kit,
        page,
        book.inventory.len().div_ceil(6),
    );
}
fn kind_phrase(kind: api::Kind) -> Phrase {
    match kind {
        api::Kind::Sleeve => Phrase::DeckAppearanceSleeve,
        api::Kind::Playmat => Phrase::DeckAppearanceMat,
    }
}
fn item_title(commands: &mut Commands, parent: Entity, kit: Kit, title: &str, kind: api::Kind) {
    let header = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            column_gap: kit.m.px(10.),
            row_gap: kit.m.px(4.),
            ..default()
        })
        .id();
    let name = commands
        .spawn((
            Text::new(title),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let tag = crate::lobby::parts::badge(
        commands,
        kit,
        kind_phrase(kind).text(kit.lang()),
        tokens::GOLD,
    );
    commands.entity(header).add_children(&[name, tag]);
    commands.entity(parent).add_child(header);
}
fn item_view(commands: &mut Commands, parent: Entity, kit: Kit, book: &Book, id: api::Id) {
    let lang = kit.lang();
    let Some(reward) = book.inventory.iter().find(|r| r.id == id) else {
        label(
            commands,
            parent,
            kit,
            Phrase::AdventLoading.text(lang),
            tokens::MUTED,
        );
        return;
    };
    let title = if lang == Lang::De {
        &reward.title.de
    } else {
        &reward.title.en
    };
    item_title(commands, parent, kit, title, reward.kind);
    let art = commands
        .spawn((
            Node {
                width: percent(100),
                height: kit.m.px(255.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(tokens::PANEL),
        ))
        .id();
    commands.entity(parent).add_child(art);
    if let Some(image) = book.image(id) {
        let e = commands
            .spawn((
                Node {
                    height: percent(100),
                    max_width: percent(100),
                    aspect_ratio: Some(
                        f32::from(reward.asset.width) / f32::from(reward.asset.height.max(1)),
                    ),
                    ..default()
                },
                crate::event_cosmetics::CosmeticPreview::new(
                    image.clone(),
                    reward.kind,
                    reward.asset.tier,
                ),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(art).add_child(e);
    } else {
        label(
            commands,
            art,
            kit,
            Phrase::AdventImageLoading.text(lang),
            tokens::MUTED,
        );
        if book.image_error(id).is_some() {
            tile_action(
                commands,
                parent,
                kit,
                Phrase::AdventRetry.text(lang),
                Action::Retry,
                "retry",
                0,
            );
        }
    }
    tile_action(
        commands,
        parent,
        kit,
        Phrase::AdventApplyDeck.text(lang),
        Action::Deck,
        "deck",
        0,
    );
}
pub(super) fn pages(commands: &mut Commands, parent: Entity, kit: Kit, page: usize, count: usize) {
    if count <= 1 {
        return;
    }
    let group = commands
        .spawn(Node {
            width: percent(100),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(group);
    if page > 0 {
        tile_action(commands, group, kit, "‹", Action::Prev, "prev", 0);
    }
    label(
        commands,
        group,
        kit,
        &format!("{} / {}", page + 1, count),
        tokens::MUTED,
    );
    if page + 1 < count {
        tile_action(commands, group, kit, "›", Action::Next, "next", 0);
    }
}
