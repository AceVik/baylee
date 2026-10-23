//! A tab's filter dialog.

use super::head::text_button;
#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;

/// The filter dialog's root.
#[derive(Component)]
pub(crate) struct FilterDialog;

/// The tab's filter dialog (§3.5): a small sheet over the panel, a box per
/// kind of line and the timestamps, Reset and Done.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
pub(super) fn filter_dialog(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<DuelPhase>>>,
    roots: Query<Entity, With<FilterDialog>>,
    mut drawn: Local<Option<(ChatTab, String)>>,
    zoom: Res<super::super::zoom::Zoom>,
) {
    let _zoom = super::super::zoom::Scope::new(zoom.0);
    let Some((machine, tab)) = host.machine().zip(ui.filters) else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = None;
        return;
    };
    let filter = machine.prefs().filter(tab);
    let key = (tab, format!("{filter:?}"));
    if drawn.as_ref() == Some(&key) && !roots.is_empty() {
        return;
    }
    *drawn = Some(key);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let at_table =
        phase.is_some_and(|p| matches!(*p.get(), DuelPhase::Playing | DuelPhase::Finished));
    let inks = if at_table { TABLE_INKS } else { LOBBY_INKS };
    let root = commands
        .spawn((
            FilterDialog,
            ChatSurface,
            RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: if at_table { Val::Auto } else { zpx(40.0) },
                right: if at_table { zpx(40.0) } else { Val::Auto },
                bottom: zpx(if at_table { 300.0 } else { 120.0 }),
                width: zpx(340.0),
                flex_direction: FlexDirection::Column,
                row_gap: zpx(6.0),
                padding: UiRect::all(zpx(14.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(8.0)),
                ..default()
            },
            BackgroundColor(inks.ground),
            BorderColor::all(inks.accent.with_alpha(0.6)),
            stacked(45),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(ChatPhrase::FilterTitle.fill(lang, &[tab_name(tab, lang)])),
            tf_bold(&fonts, z(13.0)),
            TextColor(inks.ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(title);
    let groups_on = filter.groups != baylee_client_core::social::GroupPick::None;
    let mut boxes = vec![
        (FilterKind::Global, ChatPhrase::KindGlobal, filter.global),
        (FilterKind::Table, ChatPhrase::KindTable, filter.table),
        (FilterKind::Team, ChatPhrase::KindTeam, filter.team),
        (
            FilterKind::Whispers,
            ChatPhrase::KindWhispers,
            filter.whispers,
        ),
        (FilterKind::Groups, ChatPhrase::KindGroups, groups_on),
        (FilterKind::Notices, ChatPhrase::KindNotices, filter.notices),
    ];
    if at_table {
        boxes.insert(0, (FilterKind::Log, ChatPhrase::KindLog, filter.log));
    } else {
        boxes.push((
            FilterKind::Presence,
            ChatPhrase::KindPresence,
            filter.friends_presence,
        ));
    }
    boxes.push((
        FilterKind::Timestamps,
        ChatPhrase::KindTimestamps,
        filter.timestamps,
    ));
    for (kind, phrase, on) in boxes {
        let mark = if on { "\u{2611}" } else { "\u{2610}" };
        let row = commands
            .spawn((
                UiButton,
                ChatPress::Filter(kind),
                Node {
                    padding: UiRect::axes(zpx(4.0), zpx(3.0)),
                    border_radius: BorderRadius::all(zpx(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(format!("{mark}  {}", phrase.text(lang))),
                    tf(&fonts, z(12.5)),
                    TextColor(if on { inks.ink } else { inks.soft }),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(root).add_child(row);
    }
    let note = commands
        .spawn((
            Text::new(ChatPhrase::FilterNote.text(lang)),
            tf_italic(&fonts, z(11.0)),
            TextColor(inks.soft),
            Pickable::IGNORE,
        ))
        .id();
    let buttons = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: zpx(8.0),
            ..default()
        })
        .id();
    let reset = text_button(
        &mut commands,
        &fonts,
        ChatPhrase::Reset.text(lang),
        ChatPress::FilterReset,
        &inks,
    );
    let done = text_button(
        &mut commands,
        &fonts,
        ChatPhrase::Done.text(lang),
        ChatPress::FilterDone,
        &inks,
    );
    commands.entity(buttons).add_children(&[reset, done]);
    commands.entity(root).add_children(&[note, buttons]);
}
