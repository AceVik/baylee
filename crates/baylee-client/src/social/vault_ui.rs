//! Native vault unlock and explicit migration. Passwords are masked, bounded,
//! zeroed when submitted/closed, and never included in render/debug state.

use baylee_client_core::{
    i18n::Lang,
    social::ChatPhrase,
    textbuf::{Dir, Step, TextBuffer},
};
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};
use zeroize::Zeroizing;

use super::{
    ChatUi, InfoState, SocialHost,
    vault_access::{self, Choice, Error},
};
use crate::{
    hud::{UiFonts, tf},
    shellkit::tokens,
    ui_button::{Clicks, UiButton},
};

#[derive(Component)]
struct Root;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Press {
    Open,
    Close,
    Field(usize),
    Action(Choice),
    Lock,
}

#[derive(Resource, Default)]
struct Form {
    password: TextBuffer,
    new_password: TextBuffer,
    again: TextBuffer,
    mismatch: bool,
    focus: usize,
    create_armed: bool,
    paste: Option<bevy::clipboard::ClipboardRead>,
    session: Option<(String, String)>,
}

impl Form {
    fn clear(&mut self) {
        self.password.clear();
        self.new_password.clear();
        self.again.clear();
        self.mismatch = false;
        self.paste = None;
        self.create_armed = false;
    }
    fn field(&mut self) -> &mut TextBuffer {
        match self.focus {
            1 => &mut self.new_password,
            2 => &mut self.again,
            _ => &mut self.password,
        }
    }
}
impl Drop for Form {
    fn drop(&mut self) {
        self.clear();
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Form>()
        .add_systems(PreUpdate, typing.after(bevy::input::InputSystems))
        .add_systems(
            Update,
            (presses, draw).chain().run_if(resource_exists::<UiFonts>),
        );
}

fn act(press: Press, form: &mut Form, host: &mut SocialHost, ui: &mut ChatUi) {
    match press {
        Press::Open => {
            ui.vault_open = true;
            form.focus = 0;
        }
        Press::Close => {
            ui.vault_open = false;
            ui.swallow = true;
            form.clear();
        }
        Press::Field(at) => {
            form.focus = at;
        }
        Press::Lock => {
            if host.vault.is_central() {
                // A still-authenticated account worker can restore these keys. Account
                // sign-out is the supported boundary; do not present a false local lock.
                return;
            }
            host.flush_save();
            host.socket.hang();
            host.socket = super::Socket::default();
            host.mailbox = super::Mailbox::default();
            host.table = None;
            host.machine = None;
            host.store = None;
            host.disk = None;
            host.save = None;
            host.toasts.clear();
            host.copy = None;
            host.vault.attempted = true;
            host.vault.error = None;
            ui.sheet = None;
            ui.typing = false;
            ui.vault_open = true;
            form.clear();
        }
        Press::Action(choice) => {
            let creating = choice != Choice::Unlock;
            let chosen = if creating && !form.new_password.is_empty() {
                form.new_password.text()
            } else {
                form.password.text()
            };
            if host.vault.busy() || chosen.is_empty() {
                return;
            }
            let confirm = if creating {
                chosen
            } else {
                form.new_password.text()
            };
            if confirm != form.again.text() {
                form.mismatch = true;
                return;
            }
            if choice == Choice::Create && !form.create_armed {
                form.create_armed = true;
                return;
            }
            let password = Zeroizing::new(chosen.to_string());
            let new = (!creating && !form.new_password.is_empty())
                .then(|| Zeroizing::new(form.new_password.text().to_string()));
            form.clear();
            vault_access::begin(host, choice, password, None, new);
        }
    }
}

fn presses(
    mut clicks: Clicks<Press>,
    mut form: ResMut<Form>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
) {
    for press in clicks.read() {
        act(press, &mut form, &mut host, &mut ui);
    }
}

#[allow(clippy::too_many_arguments)] // one password form and the native input/clipboard
fn typing(
    mut keys: MessageReader<KeyboardInput>,
    codes: Res<ButtonInput<KeyCode>>,
    mut form: ResMut<Form>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    if !ui.vault_open {
        keys.clear();
        return;
    }
    if let Some(result) = form
        .paste
        .as_mut()
        .and_then(bevy::clipboard::ClipboardRead::poll_result)
    {
        form.paste = None;
        if let Ok(text) = result {
            insert(&mut form, &Zeroizing::new(text));
        }
    }
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let command = codes.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        match &key.logical_key {
            Key::Escape => {
                act(Press::Close, &mut form, &mut host, &mut ui);
                break;
            }
            Key::Tab => form.focus = (form.focus + if shift { 7 } else { 1 }) % 8,
            Key::Enter => {
                let press = match form.focus {
                    4 => Press::Action(Choice::ImportFile),
                    5 => Press::Action(Choice::ImportKeychain),
                    6 => Press::Action(Choice::Create),
                    7 => Press::Close,
                    _ => Press::Action(Choice::Unlock),
                };
                act(press, &mut form, &mut host, &mut ui);
            }
            _ if form.focus > 2 || host.vault.busy() => {}
            Key::Backspace => form.field().delete_back(),
            Key::Delete => form.field().delete_forward(),
            Key::ArrowLeft => form.field().move_caret(Step::Char, Dir::Left, shift),
            Key::ArrowRight => form.field().move_caret(Step::Char, Dir::Right, shift),
            Key::Home => form.field().move_caret(Step::Line, Dir::Left, shift),
            Key::End => form.field().move_caret(Step::Line, Dir::Right, shift),
            Key::Character(c) if command && c.eq_ignore_ascii_case("a") => {
                form.field().select_all();
            }
            Key::Character(c) if command && c.eq_ignore_ascii_case("v") => {
                if let Some(clipboard) = clipboard.as_deref_mut() {
                    form.paste = Some(clipboard.fetch_text());
                }
            }
            _ if command => {}
            _ => {
                if let Some(text) = &key.text {
                    insert(&mut form, text);
                }
            }
        }
    }
}

fn insert(form: &mut Form, text: &str) {
    if form.focus > 2 {
        return;
    }
    let text = Zeroizing::new(text.chars().filter(|c| !c.is_control()).collect::<String>());
    if form.field().text().len() + text.len()
        <= baylee_client_core::social::vault::password::MAX_PASSWORD
    {
        form.field().replace_selection(&text);
    }
}

fn button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    press: Press,
    focus: bool,
) -> Entity {
    commands
        .spawn((
            UiButton,
            press,
            Node {
                min_height: px(44.0),
                padding: UiRect::all(px(10.0)),
                border: UiRect::all(px(1.0)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(if focus {
                tokens::ACCENT
            } else {
                tokens::BORDER
            }),
            children![(
                Text::new(label),
                tf(fonts, 14.0),
                TextColor(tokens::INK),
                Pickable::IGNORE
            )],
        ))
        .id()
}

fn words(commands: &mut Commands, fonts: &UiFonts, text: impl Into<String>) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(fonts, 14.0),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id()
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one small native sheet, two masked fields and explicit choices
fn draw(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut form: ResMut<Form>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    roots: Query<Entity, With<Root>>,
    mut drawn: Local<String>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    windows: Query<&Window>,
    captions: Query<
        (&ComputedNode, &UiGlobalTransform),
        With<crate::lobby::memorial::MemorialCaption>,
    >,
) {
    let at_table = phase.is_some_and(|p| *p.get() != crate::DuelPhase::Closed);
    let window_h = windows.iter().next().map_or(720.0, Window::height);
    let left = super::window::pill_left(&captions, window_h);
    let session = host
        .session
        .as_ref()
        .map(|s| (s.gateway.clone(), s.token.clone()));
    if form.session != session {
        form.clear();
        ui.vault_open = false;
        form.session = session;
    }
    let visible = host.session.as_ref().is_some_and(|s| !s.guest)
        && matches!(
            host.info,
            InfoState::Known(Some((baylee_protocol::social::SOCIAL_VERSION, _)))
        );
    if host.machine.is_some() && ui.vault_open {
        form.clear();
        ui.vault_open = false;
    }
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    // Only masks/counts and public UI state enter the render cache.
    let key = format!(
        "{visible}:{}:{}:{:?}:{}:{}:{}:{}:{lang:?}",
        host.active(),
        ui.vault_open,
        host.vault.error,
        host.vault.busy(),
        form.password.text().chars().count(),
        form.new_password.text().chars().count(),
        form.focus
    );
    let key = format!(
        "{key}:{}:{}:{}:{}:{}",
        form.create_armed,
        form.again.text().chars().count(),
        form.mismatch,
        ui.open,
        ui.minimised
    );
    let key = format!(
        "{key}:{:?}:{at_table}:{left:?}",
        host.session.as_ref().map(|s| (&s.gateway, &s.handle))
    );
    if *drawn == key && !roots.is_empty() {
        return;
    }
    *drawn = key;
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !visible || (host.active() && host.vault.is_central()) {
        return;
    }
    // Unlocked, the chat is locked from its composer, which carries the
    // lock; nothing floats. A "Lock chat" pill stood free at 42 % of the
    // width over the lobby and over the hand at a table whenever the window
    // was shut (observed-faults, beta.8b pictures; played again 29.10.2026).
    if host.active() {
        return;
    }
    if !ui.vault_open {
        let root = button(
            &mut commands,
            &fonts,
            ChatPhrase::VaultOpen.text(lang),
            Press::Open,
            false,
        );
        // Locked, the way back in stands where the chat's pill would: the
        // lobby's bottom-left corner, clear of the memorial's caption, and
        // at a table under the arrangement pill, off the hand.
        let place = if at_table {
            Node {
                position_type: PositionType::Absolute,
                top: px(60.0),
                left: px(16.0),
                min_height: px(44.0),
                padding: UiRect::all(px(10.0)),
                ..default()
            }
        } else {
            Node {
                position_type: PositionType::Absolute,
                bottom: px(12.0),
                left: left.unwrap_or(px(12.0)),
                min_height: px(44.0),
                padding: UiRect::all(px(10.0)),
                ..default()
            }
        };
        commands
            .entity(root)
            .insert((Root, place, super::window::stacked(51)));
        return;
    }
    let root = commands
        .spawn((
            Root,
            super::window::ChatSurface,
            bevy::ui::RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                top: px(0.0),
                bottom: px(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            super::window::stacked(70),
        ))
        .id();
    let card = commands
        .spawn((
            Node {
                width: px(600.0),
                max_width: Val::Percent(95.0),
                max_height: Val::Percent(90.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10.0),
                padding: UiRect::all(px(20.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            crate::hud::Scrolls,
            bevy::ui::ScrollPosition::default(),
        ))
        .id();
    commands.entity(root).add_child(card);
    let mut children = vec![
        words(&mut commands, &fonts, ChatPhrase::VaultTitle.text(lang)),
        words(&mut commands, &fonts, ChatPhrase::VaultExplain.text(lang)),
    ];
    if let Some(session) = host.session.as_ref() {
        children.push(words(
            &mut commands,
            &fonts,
            format!("{} · {}", session.handle, session.gateway),
        ));
    }
    if let Some(error) = host.vault.error {
        children.push(words(&mut commands, &fonts, error_words(error).text(lang)));
    }
    if form.mismatch {
        children.push(words(
            &mut commands,
            &fonts,
            ChatPhrase::VaultPasswordMismatch.text(lang),
        ));
    }
    if host.vault.busy() {
        children.push(words(
            &mut commands,
            &fonts,
            ChatPhrase::VaultWorking.text(lang),
        ));
    }
    for (at, (phrase, value)) in [
        (ChatPhrase::VaultPassword, &form.password),
        (ChatPhrase::VaultNewPassword, &form.new_password),
        (ChatPhrase::VaultPasswordAgain, &form.again),
    ]
    .into_iter()
    .enumerate()
    {
        children.push(button(
            &mut commands,
            &fonts,
            &format!(
                "{}: {}",
                phrase.text(lang),
                "•".repeat(value.text().chars().count())
            ),
            Press::Field(at),
            form.focus == at,
        ));
    }
    for (at, (phrase, choice)) in [
        (ChatPhrase::VaultUnlock, Choice::Unlock),
        (ChatPhrase::VaultImportFile, Choice::ImportFile),
        (ChatPhrase::VaultImportKeychain, Choice::ImportKeychain),
        (
            if form.create_armed {
                ChatPhrase::VaultConfirmCreate
            } else {
                ChatPhrase::VaultCreate
            },
            Choice::Create,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        children.push(button(
            &mut commands,
            &fonts,
            phrase.text(lang),
            Press::Action(choice),
            form.focus == at + 3,
        ));
    }
    children.push(button(
        &mut commands,
        &fonts,
        ChatPhrase::VaultLater.text(lang),
        Press::Close,
        form.focus == 7,
    ));
    commands.entity(card).add_children(&children);
}

fn error_words(error: Error) -> ChatPhrase {
    match error {
        Error::Authentication => ChatPhrase::VaultWrongPassword,
        Error::Missing => ChatPhrase::VaultMissing,
        Error::Busy => ChatPhrase::VaultBusy,
        Error::Exists => ChatPhrase::VaultExists,
        Error::Format | Error::Entropy | Error::Io | Error::Scope => ChatPhrase::VaultFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_and_preferences_survive_lock_and_edits_while_locked() {
        use baylee_client_core::social::{ChatPrefs, Intent, Me, Social};
        let me = Me {
            tag: "AB12".into(),
            handle: "Alice#AB12".into(),
            guest: false,
            gateway: "https://example.test".into(),
        };
        let (machine, _) = Social::start(
            me.clone(),
            None,
            None,
            ChatPrefs::default(),
            super::super::now_ms(),
            &mut super::super::OsEntropy,
        );
        let mut host = SocialHost {
            machine: Some(machine),
            ..Default::default()
        };
        let mut prefs = crate::prefs::Prefs::default();
        host.act(Intent::NoticeSeen);
        let seen = host.machine().unwrap().prefs().clone();
        super::super::window::save_prefs(&mut host, Some(&mut prefs), seen);
        act(
            Press::Lock,
            &mut Form::default(),
            &mut host,
            &mut ChatUi::default(),
        );
        assert!(host.machine().is_none());
        assert!(host.vault.prefs.notice_seen);
        let mut edited = prefs.all().chat.clone();
        edited.timestamps = false;
        super::super::window::save_prefs(&mut host, Some(&mut prefs), edited);
        assert!(!host.vault.prefs.timestamps);
        // A fetched account update also arrives while chat is locked.
        prefs.edit().chat.toast_text = false;
        host.sync_prefs(&prefs.all().chat);
        let (reopened, _) = Social::start(
            me,
            None,
            None,
            host.vault.prefs.clone(),
            super::super::now_ms(),
            &mut super::super::OsEntropy,
        );
        assert_eq!(reopened.prefs(), &prefs.all().chat);
        assert!(reopened.prefs().notice_seen);
    }

    #[test]
    fn close_erases_all_password_fields_and_releases_keyboard() {
        let mut form = Form::default();
        form.password.set("old", 3, None);
        form.new_password.set("new", 3, None);
        form.again.set("new", 3, None);
        let mut ui = ChatUi {
            vault_open: true,
            ..Default::default()
        };
        let mut host = SocialHost::default();
        act(Press::Close, &mut form, &mut host, &mut ui);
        assert!(form.password.is_empty() && form.new_password.is_empty() && form.again.is_empty());
        assert!(!ui.vault_open);
        assert!(ui.swallow);
    }

    #[test]
    fn mismatched_new_password_does_not_start_a_job() {
        let mut form = Form::default();
        form.password.set("old", 3, None);
        form.new_password.set("new", 3, None);
        form.again.set("typo", 4, None);
        let mut host = SocialHost::default();
        act(
            Press::Action(Choice::Unlock),
            &mut form,
            &mut host,
            &mut ChatUi::default(),
        );
        assert!(form.mismatch);
        assert!(!host.vault.busy());
    }

    #[test]
    fn create_and_import_require_confirmation_of_the_actual_wrapping_password() {
        for choice in [Choice::Create, Choice::ImportFile, Choice::ImportKeychain] {
            let mut form = Form::default();
            form.password.set("password", 8, None);
            let mut host = SocialHost::default();
            act(
                Press::Action(choice),
                &mut form,
                &mut host,
                &mut ChatUi::default(),
            );
            assert!(form.mismatch && !form.create_armed && !host.vault.busy());
            form.again.set("password", 8, None);
            form.new_password.set("different", 9, None);
            act(
                Press::Action(choice),
                &mut form,
                &mut host,
                &mut ChatUi::default(),
            );
            assert!(form.mismatch && !form.create_armed && !host.vault.busy());
        }
    }

    #[test]
    fn new_device_creation_requires_two_deliberate_presses() {
        let mut form = Form::default();
        form.password.set("password", 8, None);
        form.again.set("password", 8, None);
        let mut host = SocialHost::default();
        act(
            Press::Action(Choice::Create),
            &mut form,
            &mut host,
            &mut ChatUi::default(),
        );
        assert!(form.create_armed);
        assert!(!host.vault.busy());
        assert_eq!(form.password.text(), "password");
        act(Press::Close, &mut form, &mut host, &mut ChatUi::default());
        assert!(!form.create_armed);
    }
}
