//! The chat's toasts (the beta.8 social design §11.11): a whisper, a
//! mention, a friend request, an invite. Up to three stand at once; one
//! with buttons (a request, an invite) stays until answered, the others go
//! after six seconds. A press on a toast's text opens its conversation.
//!
//! The sounds the machine asks for are played here too, through the
//! interface's cue path.

use super::zoom::{z, zpx};
use crate::ui_button::{Clicks, UiButton};
use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{ChatCue, ChatToast, ToastAbout, ToastButton};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use super::window::ChatSurface;
use super::{SocialHost, UiAsk};
use crate::ambience::Feel;
use crate::hud::{UiFonts, glyph, icon_tf, tf, tf_bold};
use crate::shellkit::tokens;

/// How long a toast without buttons stands.
const STANDS: f32 = 15.0;

/// How many stand at once.
const AT_ONCE: usize = 3;

#[cfg(test)]
mod tests;

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
    crate::ui_button::install(app);
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
        if !host.withdrawn.is_empty() {
            host.withdrawn.clear();
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
    // Drained whether or not anything stands, so the list never grows.
    if !host.withdrawn.is_empty() {
        let gone = std::mem::take(&mut host.withdrawn);
        if withdraw_answered(&mut toasts.bypass_change_detection().standing, &gone) {
            toasts.set_changed();
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

/// Takes away every toast that asked about something in `gone`: a request
/// accepted or declined in the drawer or on another device, or taken back by
/// whoever sent it; an invite answered, withdrawn or lapsed. Its buttons
/// would answer nothing (feedback of 29.10.2026: the request's toast kept
/// asking after the drawer had accepted it). Whether any went.
fn withdraw_answered(standing: &mut Vec<(u64, ChatToast, f32)>, gone: &[ToastAbout]) -> bool {
    let before = standing.len();
    standing.retain(|(_, toast, _)| {
        toast
            .about
            .as_ref()
            .is_none_or(|about| !gone.contains(about))
    });
    standing.len() != before
}

fn presses(
    mut clicks: Clicks<ToastPress>,
    mut host: ResMut<SocialHost>,
    mut toasts: ResMut<Toasts>,
) {
    for press in clicks.read() {
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

/// Draws the column at the top-left corner of the screen: narrow, with title,
/// icon, gradient, shader sheen, and 15-second auto-dismiss.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a toast is drawn top to bottom
fn sync(
    mut commands: Commands,
    mut toasts: ResMut<Toasts>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    mut columns: Query<(Entity, &mut Node), With<ToastColumn>>,
    _ui: Res<super::ChatUi>,
    (_windows, _scale): (Query<&Window>, Option<Res<UiScale>>),
    zoom: Res<super::zoom::Zoom>,
    _slide: Res<super::drawer::DrawerSlide>,
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
        let top = if at_table { zpx(60.0) } else { zpx(62.0) };
        if node.top != top {
            node.top = top;
        }
        if node.bottom != Val::Auto {
            node.bottom = Val::Auto;
        }
        let left = zpx(12.0);
        if node.left != left {
            node.left = left;
        }
        if node.right != Val::Auto {
            node.right = Val::Auto;
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
                    left: zpx(12.0),
                    right: Val::Auto,
                    top: zpx(if at_table { 60.0 } else { 62.0 }),
                    bottom: Val::Auto,
                    width: zpx(290.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: zpx(8.0),
                    ..default()
                },
                super::window::stacked(40),
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
                    width: Val::Percent(100.0),
                    max_width: zpx(290.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: zpx(6.0),
                    padding: UiRect::axes(zpx(12.0), zpx(10.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(10.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.095, 0.135, 0.88)),
                BackgroundGradient::from(LinearGradient::new(
                    135.0_f32.to_radians(),
                    vec![
                        ColorStop::auto(Color::srgba(0.11, 0.20, 0.27, 0.92)),
                        ColorStop::auto(Color::srgba(0.035, 0.07, 0.10, 0.88)),
                    ],
                )),
                BorderColor::all(super::rows::WHISPER_INK.with_alpha(0.65)),
                BoxShadow::default(),
                crate::shellkit::sheen::Sheen,
            ))
            .id();
        let top = commands
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: zpx(8.0),
                ..default()
            },))
            .id();
        let badge = commands
            .spawn((
                Node {
                    width: zpx(22.0),
                    height: zpx(22.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(zpx(6.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(super::rows::WHISPER_INK.with_alpha(0.18)),
                Pickable::IGNORE,
                children![(
                    Text::new(glyph::CHAT.to_string()),
                    icon_tf(&fonts, z(11.0)),
                    TextColor(super::rows::WHISPER_INK),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        let title = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    min_width: zpx(0.0),
                    ..default()
                },
                Text::new(
                    baylee_client_core::social::ChatPhrase::ChatTitle
                        .text(lang)
                        .to_string(),
                ),
                tf_bold(&fonts, z(12.0)),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
        let close = commands
            .spawn((
                UiButton,
                ToastPress::Close(id),
                Node {
                    width: zpx(24.0),
                    height: zpx(24.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(zpx(5.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::new(Color::NONE),
                children![(
                    Text::new(glyph::CLOSE.to_string()),
                    icon_tf(&fonts, z(11.0)),
                    TextColor(tokens::MUTED),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(top).add_children(&[badge, title, close]);
        let text = commands
            .spawn((
                UiButton,
                ToastPress::Open(id),
                Node {
                    width: Val::Percent(100.0),
                    min_width: zpx(0.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                children![(
                    Text::new(words),
                    tf(&fonts, z(12.0)),
                    TextColor(tokens::INK),
                    TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                    Pickable::IGNORE,
                )],
            ))
            .id();
        commands.entity(card).add_children(&[top, text]);
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
                        UiButton,
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
