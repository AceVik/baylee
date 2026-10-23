//! The window's head: the tab row and its buttons, the Group tab's bar,
//! and the pills and text buttons they are built of.

#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;

/// The tab row's contents: the tabs left, the head's buttons right.
pub(super) fn head_parts(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    counts: &[(ChatTab, u32, bool)],
    active: ChatTab,
    frame: Frame,
) -> Vec<Entity> {
    let inks = frame.inks();
    let mut out = Vec::new();
    for (tab, unread, urgent) in counts {
        let on = *tab == active;
        let ink = if on || *unread > 0 {
            inks.ink
        } else {
            inks.soft
        };
        let label = commands
            .spawn((
                Text::new(tab_name(*tab, lang)),
                tf(fonts, z(12.5)),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        let button = commands
            .spawn((
                UiButton,
                ChatPress::Tab(*tab),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: zpx(4.0),
                    padding: UiRect::new(zpx(8.0), zpx(8.0), zpx(0.0), zpx(0.0)),
                    border: UiRect::bottom(zpx(2.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(if on { inks.accent } else { Color::NONE }),
                Feel::new(Color::NONE),
            ))
            .add_child(label)
            .id();
        if *unread > 0 {
            let pill = count_pill(commands, fonts, *unread, *urgent, &inks);
            commands.entity(button).add_child(pill);
        }
        out.push(button);
    }
    let spacer = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    out.push(spacer);
    let tool = |commands: &mut Commands, glyph: char, press: ChatPress| {
        commands
            .spawn((
                UiButton,
                press,
                Node {
                    width: zpx(HEAD_BTN),
                    height: zpx(HEAD_BTN),
                    align_self: AlignSelf::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::left(zpx(4.0)),
                    border_radius: BorderRadius::all(zpx(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(glyph.to_string()),
                    icon_tf(fonts, z(10.0)),
                    TextColor(inks.soft),
                    Pickable::IGNORE,
                )],
            ))
            .id()
    };
    // The lock is the line under the tabs (owner, 13.10.), not a tool here.
    out.push(tool(commands, glyph::FRIENDS, ChatPress::Friends));
    out.push(tool(commands, glyph::GEAR, ChatPress::Gear));
    if frame == Frame::Lobby {
        out.push(tool(commands, glyph::MINUS, ChatPress::Minimise));
    }
    out.push(tool(commands, glyph::CLOSE, ChatPress::Close));
    out
}

/// An unread count's pill: `CANDLE` ground, rose for a whisper or a
/// mention, `99+` at most (§3.2).
pub(crate) fn count_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    count: u32,
    urgent: bool,
    inks: &Inks,
) -> Entity {
    let text = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };
    let ground = if urgent {
        rows::WHISPER_INK
    } else {
        inks.accent
    };
    commands
        .spawn((
            Node {
                min_width: zpx(14.0),
                height: zpx(14.0),
                padding: UiRect::horizontal(zpx(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(zpx(7.0)),
                ..default()
            },
            BackgroundColor(ground),
            Pickable::IGNORE,
            children![(
                Text::new(text),
                tf_bold(fonts, z(9.0)),
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// The Group tab's bar: the shown group's name with its dropdown, and its
/// member count.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
pub(super) fn group_bar(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    groups: &[(String, String, u32)],
    shown: Option<&str>,
    menu: bool,
    inks: &Inks,
) -> Vec<Entity> {
    let Some(shown) = shown else {
        let label = commands
            .spawn((
                Text::new(ChatPhrase::NoGroupYet.text(lang)),
                tf(fonts, z(11.5)),
                TextColor(inks.soft),
                Pickable::IGNORE,
            ))
            .id();
        let new = text_button(
            commands,
            fonts,
            ChatPhrase::NewGroupButton.text(lang),
            ChatPress::NewGroup,
            inks,
        );
        return vec![label, new];
    };
    let name = groups
        .iter()
        .find(|(id, _, _)| id == shown)
        .map_or_else(String::new, |(_, n, _)| n.clone());
    let group = machine.contacts().group(shown);
    let members = group.map_or(0, |g| g.members.len());
    let online = group.map_or(0, |g| {
        g.members
            .iter()
            .filter(|m| {
                m.person.tag == machine.me().tag
                    || machine.contacts().friend(&m.person.tag).is_some_and(|f| {
                        f.presence.status != baylee_protocol::social::Shown::Offline
                    })
            })
            .count()
    });
    let picker = commands
        .spawn((
            UiButton,
            ChatPress::GroupMenu,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(6.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![
                (
                    Text::new(name),
                    tf_bold(fonts, z(11.5)),
                    TextColor(rows::GROUP_INK),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(glyph::CHEVRON_DOWN.to_string()),
                    icon_tf(fonts, z(8.0)),
                    TextColor(inks.soft),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id();
    if menu {
        let list = commands
            .spawn((
                ChatSurface,
                Node {
                    position_type: PositionType::Absolute,
                    top: zpx(SUB_H),
                    left: zpx(0.0),
                    min_width: zpx(220.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(zpx(4.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(6.0)),
                    ..default()
                },
                BackgroundColor(inks.ground),
                BorderColor::all(inks.line),
                stacked(20),
            ))
            .id();
        for (id, name, unread) in groups {
            let tick = if id == shown { "\u{2713} " } else { "   " };
            let label = if *unread > 0 {
                format!("{tick}{name}  ({unread})")
            } else {
                format!("{tick}{name}")
            };
            let row = text_button(
                commands,
                fonts,
                &label,
                ChatPress::ShowGroup(id.clone()),
                inks,
            );
            commands.entity(list).add_child(row);
        }
        let new = text_button(
            commands,
            fonts,
            ChatPhrase::NewGroupButton.text(lang),
            ChatPress::NewGroup,
            inks,
        );
        commands.entity(list).add_child(new);
        commands.entity(picker).add_child(list);
    }
    let count = commands
        .spawn((
            Text::new(
                ChatPhrase::GroupCount.fill(lang, &[&members.to_string(), &online.to_string()]),
            ),
            tf(fonts, z(11.0)),
            TextColor(inks.soft),
            Pickable::IGNORE,
        ))
        .id();
    vec![picker, count]
}

/// A plain text button.
pub(crate) fn text_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    press: ChatPress,
    inks: &Inks,
) -> Entity {
    commands
        .spawn((
            UiButton,
            press,
            Node {
                padding: UiRect::axes(zpx(8.0), zpx(3.0)),
                border_radius: BorderRadius::all(zpx(4.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(label.to_string()),
                tf(fonts, z(11.5)),
                TextColor(inks.ink),
                Pickable::IGNORE,
            )],
        ))
        .id()
}
