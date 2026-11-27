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
#[derive(Component)]
pub(crate) struct VaultList;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Press {
    Open,
    Close,
    Field(usize),
    Action(Choice),
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
                    3 => Press::Action(Choice::ImportFile),
                    4 => Press::Action(Choice::ImportKeychain),
                    5 => Press::Action(Choice::Create),
                    6 => Press::Close,
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

fn action_disabled_reason(choice: Choice, form: &Form, host: &SocialHost) -> Option<ChatPhrase> {
    if host.vault.busy() {
        return Some(ChatPhrase::VaultWorking);
    }
    let creating = matches!(
        choice,
        Choice::Create | Choice::ImportFile | Choice::ImportKeychain
    );
    let chosen = if creating && !form.new_password.is_empty() {
        form.new_password.text()
    } else {
        form.password.text()
    };
    if chosen.is_empty() {
        return Some(ChatPhrase::VaultPassword);
    }
    let confirm = if creating {
        chosen
    } else {
        form.new_password.text()
    };
    if confirm != form.again.text() {
        return Some(if form.again.is_empty() {
            ChatPhrase::VaultPasswordAgain
        } else {
            ChatPhrase::VaultPasswordMismatch
        });
    }
    None
}

fn button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    press: Press,
    focus: bool,
) -> Entity {
    styled_button(commands, fonts, label, press, focus, false, None)
}

fn styled_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    press: Press,
    focus: bool,
    stretch: bool,
    disabled_hint: Option<String>,
) -> Entity {
    let enabled = disabled_hint.is_none();
    let rest_border = if focus {
        tokens::ACCENT
    } else if enabled {
        super::rows::GLASS_RULE
    } else {
        tokens::BORDER.with_alpha(0.45)
    };
    let rest_bg = if enabled {
        tokens::CONTROL
    } else {
        tokens::CONTROL.with_alpha(0.45)
    };
    let mut ent = commands.spawn((
        Node {
            min_height: px(42.0),
            flex_grow: if stretch { 1.0 } else { 0.0 },
            flex_basis: if stretch { px(0.0) } else { Val::Auto },
            padding: UiRect::axes(px(14.0), px(9.0)),
            border: UiRect::all(px(1.0)),
            border_radius: BorderRadius::all(px(tokens::RADIUS_CONTROL)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(rest_bg),
        BorderColor::all(rest_border),
        children![(
            Text::new(label),
            tf(fonts, 13.5),
            TextColor(if enabled { tokens::INK } else { tokens::MUTED }),
            Pickable::IGNORE
        )],
    ));
    if let Some(reason) = disabled_hint {
        ent.insert((
            crate::shellkit::surfaces::elevation_control_disabled(),
            super::hint_bundle(reason),
        ));
    } else {
        ent.insert((
            UiButton,
            press,
            crate::shellkit::surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(tokens::CONTROL, tokens::HOVER),
            crate::ambience::FeelBorder {
                rest: rest_border,
                hot: tokens::ACCENT,
                press: tokens::ACCENT,
            },
            crate::ambience::FeelShadow,
        ));
    }
    ent.id()
}

fn button_row(commands: &mut Commands, items: &[Entity]) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                column_gap: px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(items)
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

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::type_complexity
)] // one small native sheet, two masked fields and explicit choices
fn draw(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    host: Res<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut form: ResMut<Form>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    roots: Query<Entity, With<Root>>,
    (scrolls, mut input_focus, mut was_open): (
        Query<&bevy::ui::ScrollPosition, With<VaultList>>,
        Option<ResMut<bevy::input_focus::InputFocus>>,
        Local<bool>,
    ),
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
    let sheet_showing = visible && !host.vault.is_central() && !host.active() && ui.vault_open;
    if !sheet_showing && *was_open {
        *was_open = false;
        if let Some(focus) = input_focus.as_mut()
            && focus.get().is_some()
        {
            focus.clear();
        }
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
        "{key}:{:?}:{at_table}:{left:?}:{:?}:{:?}",
        host.session.as_ref().map(|s| (&s.gateway, &s.handle)),
        action_disabled_reason(Choice::Unlock, &form, &host),
        action_disabled_reason(Choice::Create, &form, &host),
    );
    if *drawn == key && !roots.is_empty() {
        return;
    }
    let keep = if sheet_showing && *was_open {
        scrolls.iter().next().cloned().unwrap_or_default()
    } else {
        bevy::ui::ScrollPosition::default()
    };
    *drawn = key;
    for root in &roots {
        commands.entity(root).try_despawn();
    }
    if !visible || host.vault.is_central() {
        return;
    }
    // Unlocked, nothing here is drawn, and nothing locks the chat again
    // before the account signs out or the client quits: a "Lock chat"
    // control asked for the password a second time and gave the player
    // nothing for it (the owner, 12.11.2026).
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
    *was_open = true;
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
            crate::shellkit::Role::Opaque,
            Node {
                width: px(600.0),
                max_width: Val::Percent(95.0),
                max_height: Val::Percent(90.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(tokens::RADIUS_PANEL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::shellkit::glass::ground(crate::shellkit::glass::Density::Reading),
            crate::shellkit::glass::rim(),
            crate::shellkit::surfaces::sheet_shadow(),
        ))
        .id();
    commands.entity(root).add_child(card);
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(20.0), px(14.0)),
                border: UiRect::bottom(px(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            super::rows::head_glass(),
            BorderColor::all(super::rows::GLASS_RULE),
            ZIndex(1),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(ChatPhrase::VaultTitle.text(lang)),
            crate::hud::tf_bold(&fonts, 16.0),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let close_x = commands
        .spawn((
            UiButton,
            Press::Close,
            Node {
                width: px(28.0),
                height: px(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::ambience::Feel::tinting_to(Color::NONE, tokens::HOVER),
            super::hint_bundle(baylee_client_core::i18n::Phrase::TransferClose.text(lang)),
            children![(
                Text::new(crate::hud::glyph::CLOSE.to_string()),
                crate::hud::icon_tf(&fonts, 11.0),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands.entity(head).add_children(&[title, close_x]);
    let mut children = vec![words(
        &mut commands,
        &fonts,
        ChatPhrase::VaultExplain.text(lang),
    )];
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
        let field_btn = button(
            &mut commands,
            &fonts,
            &format!(
                "{}: {}",
                phrase.text(lang),
                "•".repeat(value.text().chars().count())
            ),
            Press::Field(at),
            form.focus == at,
        );
        commands
            .entity(field_btn)
            .entry::<Node>()
            .and_modify(|mut n| {
                n.width = Val::Percent(100.0);
                n.justify_content = JustifyContent::FlexStart;
            });
        children.push(field_btn);
    }
    let reason_text = |choice: Choice| {
        action_disabled_reason(choice, &form, &host).map(|p| p.text(lang).to_string())
    };
    let import_file = styled_button(
        &mut commands,
        &fonts,
        ChatPhrase::VaultImportFile.text(lang),
        Press::Action(Choice::ImportFile),
        form.focus == 3,
        true,
        reason_text(Choice::ImportFile),
    );
    let import_keychain = styled_button(
        &mut commands,
        &fonts,
        ChatPhrase::VaultImportKeychain.text(lang),
        Press::Action(Choice::ImportKeychain),
        form.focus == 4,
        true,
        reason_text(Choice::ImportKeychain),
    );
    children.push(button_row(&mut commands, &[import_file, import_keychain]));

    let create_phrase = if form.create_armed {
        ChatPhrase::VaultConfirmCreate
    } else {
        ChatPhrase::VaultCreate
    };
    let create_btn = styled_button(
        &mut commands,
        &fonts,
        create_phrase.text(lang),
        Press::Action(Choice::Create),
        form.focus == 5,
        true,
        reason_text(Choice::Create),
    );
    children.push(button_row(&mut commands, &[create_btn]));

    let later = styled_button(
        &mut commands,
        &fonts,
        ChatPhrase::VaultLater.text(lang),
        Press::Close,
        form.focus == 6,
        true,
        None,
    );
    let unlock = styled_button(
        &mut commands,
        &fonts,
        ChatPhrase::VaultUnlock.text(lang),
        Press::Action(Choice::Unlock),
        form.focus == 7,
        true,
        reason_text(Choice::Unlock),
    );
    children.push(button_row(&mut commands, &[later, unlock]));

    let want_focus = match form.focus {
        3 => Some(import_file),
        4 => Some(import_keychain),
        5 => Some(create_btn),
        6 => Some(later),
        7 => Some(unlock),
        _ => None,
    };
    if let Some(focus) = input_focus.as_mut()
        && focus.get() != want_focus
    {
        match want_focus {
            Some(entity) => focus.set(entity, bevy::input_focus::FocusCause::Navigated),
            None => focus.clear(),
        }
    }

    let list = commands
        .spawn((
            VaultList,
            crate::hud::Scrolls,
            keep,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0.0),
                min_height: px(0.0),
                row_gap: px(10.0),
                padding: UiRect::all(px(20.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(list).add_children(&children);
    let track = crate::hud::logpanel::scrollbar(
        &mut commands,
        list,
        (
            super::rows::GLASS_EDGE.with_alpha(0.32),
            tokens::ACCENT.with_alpha(0.84),
        ),
        crate::hud::logpanel::at_rest,
    );
    commands
        .entity(track)
        .insert((
            crate::hud::logpanel::OnDemandScrollbar(list),
            Visibility::Hidden,
        ))
        .entry::<Node>()
        .and_modify(|mut n| {
            n.position_type = PositionType::Absolute;
            n.right = px(4.0);
            n.top = px(8.0);
            n.bottom = px(8.0);
            n.width = px(5.0);
        });
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: px(0.0),
                ..default()
            },
            ZIndex(1),
            Pickable::IGNORE,
        ))
        .add_children(&[list, track])
        .id();
    let rim = commands
        .spawn(super::glass::layer(
            super::glass::Glass::Panel,
            px(tokens::RADIUS_PANEL),
        ))
        .id();
    commands.entity(card).add_children(&[head, body, rim]);
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
    fn notice_and_preferences_survive_edits_while_locked() {
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
        // Locked is where a resumed session without its password stands:
        // no machine, and the preferences it saved kept beside the vault.
        host.machine = None;
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

    #[test]
    fn vault_rows_are_symmetric_and_disabled_actions_expose_hover_reasons() {
        let mut form = Form::default();
        let host = SocialHost::default();
        assert_eq!(
            action_disabled_reason(Choice::Unlock, &form, &host),
            Some(ChatPhrase::VaultPassword)
        );
        form.password.set("secret", 6, None);
        assert_eq!(action_disabled_reason(Choice::Unlock, &form, &host), None);
        assert_eq!(
            action_disabled_reason(Choice::Create, &form, &host),
            Some(ChatPhrase::VaultPasswordAgain)
        );
        form.again.set("typo", 4, None);
        assert_eq!(
            action_disabled_reason(Choice::Create, &form, &host),
            Some(ChatPhrase::VaultPasswordMismatch)
        );
        form.again.set("secret", 6, None);
        assert_eq!(action_disabled_reason(Choice::Create, &form, &host), None);

        let mut app = App::new();
        let fonts = UiFonts::default();
        let (row, disabled, enabled) = {
            let mut commands = app.world_mut().commands();
            let a = styled_button(
                &mut commands,
                &fonts,
                "Unlock",
                Press::Action(Choice::Unlock),
                false,
                true,
                Some("Password required".into()),
            );
            let b = styled_button(
                &mut commands,
                &fonts,
                "Later",
                Press::Close,
                false,
                true,
                None,
            );
            let r = button_row(&mut commands, &[a, b]);
            (r, a, b)
        };
        app.world_mut().flush();
        assert_eq!(
            app.world().get::<Node>(row).unwrap().width,
            Val::Percent(100.0)
        );
        for btn in [disabled, enabled] {
            let n = app.world().get::<Node>(btn).unwrap();
            assert!((n.flex_grow - 1.0).abs() < f32::EPSILON);
            assert_eq!(n.flex_basis, px(0.0));
        }
        assert!(app.world().get::<UiButton>(disabled).is_none());
        assert_eq!(
            app.world()
                .get::<crate::hud::Hint>(disabled)
                .map(|h| h.0.as_str()),
            Some("Password required")
        );
        assert!(app.world().get::<UiButton>(enabled).is_some());
    }

    fn open_vault_app() -> App {
        let mut app = App::new();
        app.init_resource::<UiFonts>()
            .init_resource::<bevy::input_focus::InputFocus>()
            .init_resource::<Form>()
            .insert_resource(SocialHost {
                session: Some(super::super::Session {
                    gateway: "https://example.test".into(),
                    token: "tok".into(),
                    handle: "Alice#AB12".into(),
                    tag: "AB12".into(),
                    guest: false,
                }),
                info: InfoState::Known(Some((baylee_protocol::social::SOCIAL_VERSION, 30))),
                ..Default::default()
            })
            .init_resource::<ChatUi>()
            .add_systems(Update, draw);
        // First update latches `form.session` to the new session.
        app.update();
        app.world_mut().resource_mut::<ChatUi>().vault_open = true;
        app
    }

    #[test]
    fn vault_rebuilds_on_equal_length_confirmation_and_preserves_scroll() {
        let mut app = open_vault_app();
        {
            let mut form = app.world_mut().resource_mut::<Form>();
            form.password.set("secret", 6, None);
            form.again.set("typos!", 6, None);
        }
        app.update();

        let root = app
            .world_mut()
            .query_filtered::<Entity, With<Root>>()
            .single(app.world())
            .unwrap();
        let card = app.world().get::<Children>(root).unwrap()[0];
        assert_eq!(
            app.world().get::<BackgroundColor>(card).unwrap().0,
            Color::NONE
        );

        let create_enabled = |app: &mut App| {
            app.world_mut()
                .query::<(&Press, Has<UiButton>)>()
                .iter(app.world())
                .any(|(p, b)| *p == Press::Action(Choice::Create) && b)
        };
        assert!(!create_enabled(&mut app));

        let list = app
            .world_mut()
            .query_filtered::<Entity, With<VaultList>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .get_mut::<bevy::ui::ScrollPosition>(list)
            .unwrap()
            .y = 128.0;

        // Fixing `again` to an equal-length match (still 6 chars) must bust the cache.
        app.world_mut()
            .resource_mut::<Form>()
            .again
            .set("secret", 6, None);
        app.update();
        assert!(create_enabled(&mut app));

        let rebuilt_list = app
            .world_mut()
            .query_filtered::<Entity, With<VaultList>>()
            .single(app.world())
            .unwrap();
        assert_ne!(rebuilt_list, list);
        assert!(
            (app.world()
                .get::<bevy::ui::ScrollPosition>(rebuilt_list)
                .unwrap()
                .y
                - 128.0)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn vault_tab_order_syncs_visual_input_focus_with_zero_resting_churn() {
        use bevy::input_focus::InputFocus;

        let mut app = open_vault_app();
        // While empty, Tab onto ImportFile focuses the disabled button with its Hint.
        app.world_mut().resource_mut::<Form>().focus = 3;
        app.update();
        let disabled_focus = app
            .world()
            .resource::<InputFocus>()
            .get()
            .expect("disabled button focused");
        assert!(app.world().get::<Press>(disabled_focus).is_none());
        assert!(
            app.world()
                .get::<crate::hud::Hint>(disabled_focus)
                .is_some()
        );

        // With matching passwords, visual Tab order 3..=7 focuses each action in order.
        {
            let mut form = app.world_mut().resource_mut::<Form>();
            form.password.set("secret", 6, None);
            form.new_password.set("secret", 6, None);
            form.again.set("secret", 6, None);
        }
        let expected = [
            (3, Press::Action(Choice::ImportFile)),
            (4, Press::Action(Choice::ImportKeychain)),
            (5, Press::Action(Choice::Create)),
            (6, Press::Close),
            (7, Press::Action(Choice::Unlock)),
        ];
        for (idx, expected_press) in expected {
            app.world_mut().resource_mut::<Form>().focus = idx;
            app.update();
            let focused = app
                .world()
                .resource::<InputFocus>()
                .get()
                .expect("button focused");
            assert_eq!(
                app.world().get::<Press>(focused).copied(),
                Some(expected_press)
            );
        }

        let tick = app.world().read_change_tick();
        app.update();
        let now = app.world().read_change_tick();
        assert!(
            !app.world()
                .resource_ref::<InputFocus>()
                .last_changed()
                .is_newer_than(tick, now),
            "resting open VaultUi must not mutate InputFocus"
        );

        app.world_mut().resource_mut::<ChatUi>().vault_open = false;
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), None);
        let tick = app.world().read_change_tick();
        app.update();
        let now = app.world().read_change_tick();
        assert!(
            !app.world()
                .resource_ref::<InputFocus>()
                .last_changed()
                .is_newer_than(tick, now),
            "closed VaultUi must not mutate InputFocus at rest"
        );
    }
}
