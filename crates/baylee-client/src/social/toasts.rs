//! The chat's toasts (the beta.8 social design §11.11): a whisper, a
//! mention, a friend request, an invite. Up to three stand at once; one
//! with buttons (a request, an invite) stays until answered, the others go
//! after six seconds. A press on a toast's text opens its conversation.
//!
//! The sounds the machine asks for are played here too, through the
//! interface's cue path.

use super::zoom::{z, zpx};
use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{ChatCue, ChatToast, ToastButton};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use super::window::{ChatSurface, G_PANEL};
use super::{SocialHost, UiAsk};
use crate::ambience::Feel;
use crate::hud::{UiFonts, glyph, icon_tf, tf, tf_bold};
use crate::shellkit::tokens;

/// How long a toast without buttons stands.
const STANDS: f32 = 6.0;

/// How many stand at once.
const AT_ONCE: usize = 3;

/// The toasts' column.
#[derive(Component)]
pub(crate) struct ToastColumn;

/// A press on a toast.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ToastPress {
    /// One of its buttons: the toast's index and the button's.
    Button(u64, usize),
    /// Its text: open its conversation.
    Open(u64),
    /// Its cross.
    Close(u64),
}

/// The toasts standing, each with a number and its age.
#[derive(Resource, Default)]
pub(crate) struct Toasts {
    next: u64,
    standing: Vec<(u64, ChatToast, f32)>,
    drawn: Vec<u64>,
}

/// Installs the toasts.
pub(crate) fn install(app: &mut App) {
    app.init_resource::<Toasts>().add_systems(
        Update,
        (take, presses, sync, sounds)
            .chain()
            .run_if(resource_exists::<UiFonts>),
    );
}

/// Takes the machine's toasts, and ages the standing ones.
fn take(time: Res<Time>, mut host: ResMut<SocialHost>, mut toasts: ResMut<Toasts>) {
    if !host.active() {
        if !toasts.standing.is_empty() {
            toasts.standing.clear();
        }
        return;
    }
    while let Some(toast) = host.toasts.pop_front() {
        let toasts = &mut *toasts;
        toasts.next += 1;
        toasts.standing.push((toasts.next, toast, 0.0));
        while toasts.standing.len() > 6 {
            toasts.standing.remove(0);
        }
    }
    if toasts.standing.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let toasts = toasts.bypass_change_detection();
    for (_, toast, age) in &mut toasts.standing {
        if !toast.until_answered {
            *age += dt;
        }
    }
    toasts.standing.retain(|(_, _, age)| *age < STANDS);
}

fn presses(
    pressed: Query<(Entity, &Interaction, &ToastPress), Changed<Interaction>>,
    mut held: Local<std::collections::HashSet<Entity>>,
    mut host: ResMut<SocialHost>,
    mut toasts: ResMut<Toasts>,
) {
    let clicked: Vec<ToastPress> = super::released(&pressed, &mut held);
    for press in clicked {
        match press {
            ToastPress::Button(id, index) => {
                let intent = toasts
                    .standing
                    .iter()
                    .find(|(n, _, _)| *n == id)
                    .and_then(|(_, t, _)| t.actions.get(index).map(|(_, i)| i.clone()));
                toasts.standing.retain(|(n, _, _)| *n != id);
                if let Some(intent) = intent {
                    host.act(intent);
                }
            }
            ToastPress::Open(id) => {
                let conv = toasts
                    .standing
                    .iter()
                    .find(|(n, _, _)| *n == id)
                    .and_then(|(_, t, _)| t.conv.clone());
                if let Some(conv) = conv {
                    host.asks.push(UiAsk::Open(conv));
                    toasts.standing.retain(|(n, _, _)| *n != id);
                }
            }
            ToastPress::Close(id) => toasts.standing.retain(|(n, _, _)| *n != id),
        }
    }
}

fn button_words(button: ToastButton, lang: Lang) -> &'static str {
    use baylee_client_core::social::ChatPhrase;
    match button {
        ToastButton::Accept => ChatPhrase::Accept.text(lang),
        ToastButton::Decline => ChatPhrase::Decline.text(lang),
        ToastButton::Join => ChatPhrase::JoinButton.text(lang),
        ToastButton::Watch => ChatPhrase::WatchButton.text(lang),
        ToastButton::NotNow => ChatPhrase::NotNow.text(lang),
    }
}

/// Draws the column: bottom-right above the pills in the lobby, under the
/// report corner at the table.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a toast is drawn top to bottom
fn sync(
    mut commands: Commands,
    mut toasts: ResMut<Toasts>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    mut columns: Query<(Entity, &mut Node), With<ToastColumn>>,
    ui: Res<super::ChatUi>,
    windows: Query<&Window>,
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let shown: Vec<u64> = toasts
        .standing
        .iter()
        .rev()
        .take(AT_ONCE)
        .map(|(n, _, _)| *n)
        .collect();
    let column = if let Some((column, mut node)) = columns.iter_mut().next() {
        // In the lobby, above the chat panel while it stands.
        let window_h = windows.iter().next().map_or(720.0, Window::height);
        let lobby_bottom = if ui.open && !ui.minimised {
            super::window::lobby_panel_height(window_h) + 16.0
        } else {
            64.0
        };
        let (top, bottom) = if at_table {
            (zpx(60.0), Val::Auto)
        } else {
            (Val::Auto, Val::Px(lobby_bottom))
        };
        if node.top != top {
            node.top = top;
        }
        if node.bottom != bottom {
            node.bottom = bottom;
        }
        column
    } else {
        toasts.drawn.clear();
        commands
            .spawn((
                ToastColumn,
                ChatSurface,
                RelativeCursorPosition::default(),
                Node {
                    position_type: PositionType::Absolute,
                    right: zpx(12.0),
                    bottom: zpx(64.0),
                    width: zpx(340.0),
                    flex_direction: FlexDirection::ColumnReverse,
                    row_gap: zpx(8.0),
                    ..default()
                },
                GlobalZIndex(G_PANEL + 40),
                Pickable::IGNORE,
            ))
            .id()
    };
    if toasts.drawn == shown {
        return;
    }
    toasts.drawn.clone_from(&shown);
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    commands.entity(column).despawn_children();
    for id in shown {
        let Some((_, toast, _)) = toasts.standing.iter().find(|(n, _, _)| *n == id) else {
            continue;
        };
        let args: Vec<&str> = toast.args.iter().map(String::as_str).collect();
        let words = toast.phrase.fill(lang, &args);
        let card = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: zpx(6.0),
                    padding: UiRect::all(zpx(10.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(8.0)),
                    ..default()
                },
                BackgroundColor(super::rows::LOBBY_GROUND),
                BorderColor::all(super::rows::WHISPER_INK.with_alpha(0.6)),
                BoxShadow::default(),
            ))
            .id();
        let top = commands
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(8.0),
                ..default()
            },))
            .id();
        let text = commands
            .spawn((
                Button,
                ToastPress::Open(id),
                Node {
                    flex_grow: 1.0,
                    min_width: zpx(0.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                children![(
                    Text::new(words),
                    tf(&fonts, z(12.5)),
                    TextColor(tokens::INK),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        let close = commands
            .spawn((
                Button,
                ToastPress::Close(id),
                Node {
                    width: zpx(20.0),
                    height: zpx(20.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(glyph::CLOSE.to_string()),
                    icon_tf(&fonts, z(9.0)),
                    TextColor(tokens::MUTED),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(top).add_children(&[text, close]);
        commands.entity(card).add_child(top);
        if !toast.actions.is_empty() {
            let buttons = commands
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: zpx(8.0),
                    ..default()
                },))
                .id();
            for (index, (button, _)) in toast.actions.iter().enumerate() {
                let b = commands
                    .spawn((
                        Button,
                        ToastPress::Button(id, index),
                        Node {
                            padding: UiRect::axes(zpx(12.0), zpx(4.0)),
                            border: UiRect::all(zpx(1.0)),
                            border_radius: BorderRadius::all(zpx(5.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(tokens::ACCENT),
                        Feel::new(Color::NONE),
                        children![(
                            Text::new(button_words(*button, lang)),
                            tf_bold(&fonts, z(11.5)),
                            TextColor(tokens::INK),
                            Pickable::IGNORE,
                        )],
                    ))
                    .id();
                commands.entity(buttons).add_child(b);
            }
            commands.entity(card).add_child(buttons);
        }
        commands.entity(column).add_child(card);
    }
    let _ = &mut toasts;
}

/// The chat's voices, made on first use.
#[derive(Resource, Default)]
pub(crate) struct ChatVoices(Vec<(crate::sound::ChatSound, Handle<AudioSource>)>);

/// Plays the cues the machine asked for, at the interface's level and this
/// device's effects volume: synthesised (`docs/legal.md` §5: no samples).
fn sounds(
    mut commands: Commands,
    mut host: ResMut<SocialHost>,
    mut voices: Local<ChatVoices>,
    sources: Option<ResMut<Assets<AudioSource>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if host.sounds.is_empty() {
        return;
    }
    let cues = std::mem::take(&mut host.sounds);
    let Some(mut sources) = sources else {
        return;
    };
    let level = prefs.map_or_else(baylee_client_core::cue::Loudness::default, |p| {
        p.all().sound
    });
    let mix = settings.as_ref().map_or(1.0, |s| {
        s.audio.effects_gain(crate::quality::focused(&windows))
    });
    if !level.audible() || mix <= 0.0 {
        return;
    }
    for cue in cues {
        let kind = match cue {
            ChatCue::Whisper => crate::sound::ChatSound::Whisper,
            ChatCue::Mention => crate::sound::ChatSound::Mention,
            ChatCue::FriendRequest => crate::sound::ChatSound::FriendRequest,
            ChatCue::GroupLine => crate::sound::ChatSound::GroupLine,
        };
        let handle = if let Some((_, h)) = voices.0.iter().find(|(k, _)| *k == kind) {
            h.clone()
        } else {
            let h = sources.add(crate::sound::chat_voice(kind));
            voices.0.push((kind, h.clone()));
            h
        };
        commands.spawn((
            bevy::audio::AudioPlayer::new(handle),
            bevy::audio::PlaybackSettings::DESPAWN
                .with_volume(bevy::audio::Volume::Linear(level.gain() * mix)),
        ));
    }
}
