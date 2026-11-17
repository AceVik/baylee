//! The chat's sheets (the beta.8 social design §11.8): the end-to-end
//! encryption notice (shown the first time the chat opens on a device, and
//! from every lock), a contact's safety number and devices, and the chat's
//! privacy settings (who may whisper, requests, sharing the table, history,
//! sounds, this account's devices, blocked players).

use super::zoom::{z, zpx};
use crate::ui_button::{Clicks, UiButton};
use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{ChatPhrase, ConvId, Intent, KeyHome, Social};
use baylee_protocol::social::{SocialSettings, WhispersFrom};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use super::window::ChatSurface;
use super::{ChatUi, SocialHost};
use crate::ambience::Feel;
use crate::hud::{UiFonts, glyph, icon_tf, palette, tf, tf_bold};
use crate::shellkit::tokens;

/// Which sheet stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sheet {
    /// The encryption notice, for a conversation (its members' safety
    /// numbers) or for none (the first-run notice).
    Encryption(Option<ConvId>),
    /// A contact's safety number.
    Safety(String),
    /// Settings → Chat & privacy.
    Privacy,
}

/// The sheet's root.
#[derive(Component)]
pub(crate) struct SheetRoot;

/// The scrollable body inside a sheet.
#[derive(Component)]
pub(crate) struct SheetList;

/// A press on a sheet.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SheetPress {
    Close,
    GotIt,
    Safety(String),
    Verify {
        tag: String,
        expected: Vec<([u8; 32], [u8; 32])>,
    },
    Unverify(String),
    WhispersFrom(WhispersFrom),
    Requests,
    ShareTable,
    History,
    Timestamps,
    ToastText,
    Sound(u8),
    RemoveDevice(String),
    Forget,
    Unblock(String),
}

/// Installs the sheets.
pub(crate) fn install(app: &mut App) {
    crate::ui_button::install(app);
    app.add_systems(
        Update,
        (first_run, presses, sync)
            .chain()
            .run_if(resource_exists::<UiFonts>),
    );
}

/// The first time the chat opens on a device, the notice stands first.
/// Opening the contacts drawer alone does not raise a blocking modal over it.
fn first_run(host: Res<SocialHost>, mut ui: ResMut<ChatUi>) {
    let Some(machine) = host.machine() else {
        return;
    };
    if machine.encryption_note().seen || ui.sheet.is_some() {
        return;
    }
    if ui.open && ui.tab.is_chat() {
        ui.sheet = Some(Sheet::Encryption(None));
    }
}

#[allow(clippy::too_many_lines)] // one arm per control
fn presses(
    mut clicks: Clicks<SheetPress>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut prefs: Option<ResMut<crate::prefs::Prefs>>,
    mut forget_armed: Local<bool>,
) {
    let mut clicked: Vec<SheetPress> = clicks.read();
    if ui.sheet.is_some() && keys.is_some_and(|k| k.just_pressed(KeyCode::Escape)) {
        clicked.push(SheetPress::Close);
        ui.swallow = true;
    }
    for press in clicked {
        let Some(machine) = host.machine() else {
            return;
        };
        let settings = machine.contacts().settings;
        let mut chat = machine.prefs().clone();
        if press != SheetPress::Forget {
            *forget_armed = false;
        }
        match press {
            SheetPress::Close => {
                if matches!(ui.sheet, Some(Sheet::Encryption(None))) {
                    host.act(Intent::NoticeSeen);
                    if let Some(chat) = host.machine().map(|m| m.prefs().clone()) {
                        super::window::save_prefs(&mut host, prefs.as_deref_mut(), chat);
                    }
                }
                ui.sheet = None;
            }
            SheetPress::GotIt => {
                host.act(Intent::NoticeSeen);
                if let Some(chat) = host.machine().map(|m| m.prefs().clone()) {
                    super::window::save_prefs(&mut host, prefs.as_deref_mut(), chat);
                }
                ui.sheet = None;
            }
            SheetPress::Safety(tag) => ui.sheet = Some(Sheet::Safety(tag)),
            SheetPress::Verify { tag, expected } => host.act(Intent::Verify { tag, expected }),
            SheetPress::Unverify(tag) => host.act(Intent::Unverify { tag }),
            SheetPress::WhispersFrom(from) => host.act(Intent::SetSettings(SocialSettings {
                whispers_from: from,
                ..settings
            })),
            SheetPress::Requests => host.act(Intent::SetSettings(SocialSettings {
                requests_open: !settings.requests_open,
                ..settings
            })),
            SheetPress::ShareTable => host.act(Intent::SetSettings(SocialSettings {
                share_table: !settings.share_table,
                ..settings
            })),
            SheetPress::History => {
                let days = baylee_client_core::social::ChatPrefs::HISTORY_DAYS;
                let at = days
                    .iter()
                    .position(|d| *d == chat.history_days)
                    .unwrap_or(2);
                chat.history_days = days[(at + 1) % days.len()];
                super::window::save_prefs(&mut host, prefs.as_deref_mut(), chat);
            }
            SheetPress::Timestamps => {
                chat.timestamps = !chat.timestamps;
                super::window::save_prefs(&mut host, prefs.as_deref_mut(), chat);
            }
            SheetPress::ToastText => {
                chat.toast_text = !chat.toast_text;
                super::window::save_prefs(&mut host, prefs.as_deref_mut(), chat);
            }
            SheetPress::Sound(n) => {
                let s = &mut chat.sounds;
                match n {
                    0 => s.whisper = !s.whisper,
                    1 => s.mention = !s.mention,
                    2 => s.group = !s.group,
                    3 => s.table = !s.table,
                    _ => s.friend_online = !s.friend_online,
                }
                super::window::save_prefs(&mut host, prefs.as_deref_mut(), chat);
            }
            SheetPress::RemoveDevice(id) => host.act(Intent::RemoveDevice { id }),
            SheetPress::Forget => {
                if *forget_armed {
                    let id = machine.device_id().to_string();
                    host.act(Intent::RemoveDevice { id });
                    *forget_armed = false;
                    ui.sheet = None;
                } else {
                    *forget_armed = true;
                    ui.forget_armed = true;
                }
            }
            SheetPress::Unblock(tag) => host.act(Intent::Unblock { tag }),
        }
    }
    if !*forget_armed && ui.forget_armed {
        ui.forget_armed = false;
    }
}

fn button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    press: SheetPress,
    lit: bool,
) -> Entity {
    let rest_bg = if lit {
        tokens::ACCENT.with_alpha(0.25)
    } else {
        tokens::CONTROL
    };
    let rest_border = if lit { tokens::ACCENT } else { tokens::BORDER };
    commands
        .spawn((
            UiButton,
            press,
            Node {
                padding: UiRect::axes(zpx(10.0), zpx(5.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(tokens::RADIUS_CONTROL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(rest_bg),
            BorderColor::all(rest_border),
            crate::shellkit::surfaces::elevation_control(),
            Feel::tinting_to(rest_bg, tokens::HOVER),
            crate::ambience::FeelBorder {
                rest: rest_border,
                hot: tokens::ACCENT,
                press: tokens::ACCENT,
            },
            crate::ambience::FeelShadow,
            children![(
                Text::new(label.to_string()),
                tf(fonts, z(12.0)),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

fn words(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: String,
    colour: Color,
    size: f32,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(fonts, z(size)),
            TextColor(colour),
            Pickable::IGNORE,
        ))
        .id()
}

fn line(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: zpx(8.0),
                row_gap: zpx(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

fn stretch(commands: &mut Commands, id: Entity) -> Entity {
    commands.entity(id).entry::<Node>().and_modify(|mut n| {
        n.flex_grow = 1.0;
        n.flex_basis = zpx(0.0);
        n.min_height = zpx(32.0);
        n.align_items = AlignItems::Center;
        n.justify_content = JustifyContent::Center;
    });
    id
}

fn switch(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    label: &str,
    on: bool,
    press: SheetPress,
) -> Entity {
    let row = line(commands);
    let name = commands
        .spawn((
            Text::new(label.to_string()),
            tf(fonts, z(12.5)),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let state = if on {
        ChatPhrase::OnLabel
    } else {
        ChatPhrase::OffLabel
    };
    let b = button(commands, fonts, state.text(lang), press, on);
    commands.entity(row).add_children(&[name, b]);
    row
}

/// The day a device was added, `DD.MM.`.
fn day(ms: u64) -> String {
    let date = i64::try_from(ms / 1000)
        .ok()
        .and_then(|s| time::OffsetDateTime::from_unix_timestamp(s).ok());
    date.map_or_else(String::new, |d| {
        format!("{:02}.{:02}.", d.day(), u8::from(d.month()))
    })
}

/// A safety number's block: the digits, the state, the button.
fn safety_block(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    tag: &str,
) -> Vec<Entity> {
    let person = machine.person(tag);
    let mut out = vec![words(
        commands,
        fonts,
        ChatPhrase::SafetyWith
            .fill(lang, &[&person.handle])
            .to_uppercase(),
        tokens::MUTED,
        11.0,
    )];
    let Some(safety) = machine.safety(tag) else {
        out.push(words(
            commands,
            fonts,
            ChatPhrase::NoKeysYet.text(lang).to_string(),
            tokens::MUTED,
            12.0,
        ));
        return out;
    };
    for chunk in safety.groups.chunks(4) {
        out.push(
            commands
                .spawn((
                    Text::new(chunk.join("  ")),
                    tf_bold(fonts, z(15.0)),
                    TextColor(tokens::INK),
                    Pickable::IGNORE,
                ))
                .id(),
        );
    }
    let state = if safety.verified {
        ChatPhrase::SafetyVerified
    } else {
        ChatPhrase::SafetyNotVerified
    };
    out.push(words(
        commands,
        fonts,
        format!(
            "{} · {}",
            state.text(lang),
            ChatPhrase::SafetyCompare.fill(lang, &[&person.handle])
        ),
        if safety.verified {
            palette::CANDLE
        } else {
            tokens::MUTED
        },
        12.0,
    ));
    let devices: Vec<String> = safety
        .devices
        .iter()
        .map(|(label, at)| format!("{label} · {}", day(*at)))
        .collect();
    out.push(words(
        commands,
        fonts,
        ChatPhrase::DevicesOf.fill(lang, &[&person.handle, &devices.join(", ")]),
        tokens::MUTED,
        11.5,
    ));
    let verify = if safety.verified {
        button(
            commands,
            fonts,
            ChatPhrase::ClearVerified.text(lang),
            SheetPress::Unverify(tag.to_string()),
            false,
        )
    } else {
        button(
            commands,
            fonts,
            ChatPhrase::MarkVerified.text(lang),
            SheetPress::Verify {
                tag: tag.to_string(),
                expected: safety.expected,
            },
            true,
        )
    };
    let verify_row = line(commands);
    let verify = stretch(commands, verify);
    commands.entity(verify_row).add_child(verify);
    out.push(verify_row);
    out
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a sheet per kind
fn sync(
    mut commands: Commands,
    host: Res<SocialHost>,
    ui: Res<ChatUi>,
    fonts: Res<UiFonts>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    roots: Query<Entity, With<SheetRoot>>,
    scrolls: Query<&bevy::ui::ScrollPosition, With<SheetList>>,
    mut drawn: Local<Option<(u64, String, bool)>>,
    mut last_sheet: Local<Option<Sheet>>,
    zoom: Res<super::zoom::Zoom>,
) {
    let _zoom = super::zoom::Scope::new(zoom.0);
    let Some((machine, sheet)) = host.machine().zip(ui.sheet.as_ref()) else {
        for root in &roots {
            commands.entity(root).try_despawn();
        }
        *drawn = None;
        *last_sheet = None;
        return;
    };
    let backup = host.vault.sync_status();
    let key = (
        machine.revision(),
        format!(
            "{sheet:?}-{}",
            backup.map_or("", |status| status.phrase().text(Lang::En))
        ),
        ui.forget_armed,
    );
    if drawn.as_ref() == Some(&key) && !roots.is_empty() {
        return;
    }
    let keep = if last_sheet.as_ref() == Some(sheet) {
        scrolls.iter().next().cloned().unwrap_or_default()
    } else {
        bevy::ui::ScrollPosition::default()
    };
    *last_sheet = Some(sheet.clone());
    *drawn = Some(key);
    for root in &roots {
        commands.entity(root).try_despawn();
    }
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let scrim = commands
        .spawn((
            SheetRoot,
            ChatSurface,
            RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: zpx(0.0),
                right: zpx(0.0),
                top: zpx(0.0),
                bottom: zpx(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            super::window::stacked(50),
        ))
        .id();
    let card = commands
        .spawn((
            crate::shellkit::Role::Opaque,
            Pickable::default(),
            ZIndex(1),
            Node {
                width: zpx(560.0),
                max_height: Val::Percent(88.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(tokens::RADIUS_PANEL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            super::rows::dialog_glass(),
            BorderColor::all(super::rows::GLASS_EDGE),
            super::rows::glass_shadow(),
        ))
        .id();
    let backdrop = commands
        .spawn((
            UiButton,
            SheetPress::Close,
            ZIndex(0),
            Node {
                position_type: PositionType::Absolute,
                left: zpx(0.0),
                right: zpx(0.0),
                top: zpx(0.0),
                bottom: zpx(0.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .id();
    commands.entity(scrim).add_children(&[card, backdrop]);
    let title = match sheet {
        Sheet::Encryption(_) | Sheet::Safety(_) => ChatPhrase::EncryptionTitle,
        Sheet::Privacy => ChatPhrase::PrivacyTitle,
    };
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(zpx(20.0), zpx(14.0)),
                border: UiRect::bottom(zpx(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            super::rows::head_glass(),
            BorderColor::all(super::rows::GLASS_RULE),
            ZIndex(1),
        ))
        .id();
    let name = commands
        .spawn((
            Text::new(format!("{} {}", glyph::LOCK, title.text(lang))),
            tf_bold(&fonts, z(16.0)),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let close = commands
        .spawn((
            UiButton,
            SheetPress::Close,
            Node {
                width: zpx(28.0),
                height: zpx(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(zpx(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::tinting_to(Color::NONE, tokens::HOVER),
            super::hint_bundle(baylee_client_core::i18n::Phrase::TransferClose.text(lang)),
            children![(
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(&fonts, z(11.0)),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands.entity(head).add_children(&[name, close]);
    let mut kids = Vec::new();
    if let Some(backup) = backup {
        kids.push(words(
            &mut commands,
            &fonts,
            backup.phrase().text(lang).to_string(),
            tokens::MUTED,
            11.5,
        ));
    }
    let note = machine.encryption_note();
    let home = match note.home {
        KeyHome::Password => ChatPhrase::KeysInPasswordVault,
        KeyHome::Central => ChatPhrase::KeysInCentralVault,
        KeyHome::Keychain => ChatPhrase::KeysInKeychain,
        KeyHome::File => ChatPhrase::KeysInFile,
        KeyHome::Memory | KeyHome::Unknown => ChatPhrase::KeysInMemory,
    };
    match sheet {
        Sheet::Encryption(conv) => {
            kids.push(words(
                &mut commands,
                &fonts,
                ChatPhrase::NoticeBody.fill(lang, &[&note.keep_days.to_string()]),
                tokens::INK,
                12.5,
            ));
            if !note.seen {
                let row = line(&mut commands);
                let got_it = button(
                    &mut commands,
                    &fonts,
                    ChatPhrase::GotIt.text(lang),
                    SheetPress::GotIt,
                    true,
                );
                let got_it = stretch(&mut commands, got_it);
                commands.entity(row).add_child(got_it);
                kids.push(row);
            }
            if let Some(conv) = conv {
                if let Some((people, devices)) = machine.encrypted_to(conv) {
                    kids.push(words(
                        &mut commands,
                        &fonts,
                        ChatPhrase::EncryptedTo
                            .fill(lang, &[&people.to_string(), &devices.to_string()]),
                        tokens::MUTED,
                        12.0,
                    ));
                }
                let me = machine.me().tag.clone();
                let partners: Vec<String> = match conv {
                    baylee_protocol::social::Conv::Dm { .. } => {
                        conv.partner(&me).map(str::to_string).into_iter().collect()
                    }
                    _ => machine.conversation(conv).map_or_default(|c| {
                        c.members
                            .iter()
                            .filter(|p| p.tag != me)
                            .map(|p| p.tag.clone())
                            .collect()
                    }),
                };
                if let [only] = partners.as_slice() {
                    let block = safety_block(&mut commands, &fonts, lang, machine, only);
                    kids.extend(block);
                } else if !partners.is_empty() {
                    let row = line(&mut commands);
                    for tag in partners {
                        let b = button(
                            &mut commands,
                            &fonts,
                            &machine.person(&tag).handle,
                            SheetPress::Safety(tag),
                            false,
                        );
                        let b = stretch(&mut commands, b);
                        commands.entity(row).add_child(b);
                    }
                    kids.push(row);
                }
            }
            kids.push(words(
                &mut commands,
                &fonts,
                home.text(lang).to_string(),
                tokens::MUTED,
                11.5,
            ));
        }
        Sheet::Safety(tag) => {
            let block = safety_block(&mut commands, &fonts, lang, machine, tag);
            kids.extend(block);
            kids.push(words(
                &mut commands,
                &fonts,
                home.text(lang).to_string(),
                tokens::MUTED,
                11.5,
            ));
        }
        Sheet::Privacy => {
            let social = machine.contacts().settings;
            let chat = machine.prefs();
            if !machine.me().guest {
                kids.push(words(
                    &mut commands,
                    &fonts,
                    ChatPhrase::WhoMayWhisper.text(lang).to_string(),
                    tokens::MUTED,
                    11.5,
                ));
                let row = line(&mut commands);
                let a = button(
                    &mut commands,
                    &fonts,
                    ChatPhrase::WhispersFriends.text(lang),
                    SheetPress::WhispersFrom(WhispersFrom::Friends),
                    social.whispers_from == WhispersFrom::Friends,
                );
                let a = stretch(&mut commands, a);
                let b = button(
                    &mut commands,
                    &fonts,
                    ChatPhrase::WhispersTables.text(lang),
                    SheetPress::WhispersFrom(WhispersFrom::FriendsAndTables),
                    social.whispers_from == WhispersFrom::FriendsAndTables,
                );
                let b = stretch(&mut commands, b);
                commands.entity(row).add_children(&[a, b]);
                kids.push(row);
                kids.push(switch(
                    &mut commands,
                    &fonts,
                    lang,
                    ChatPhrase::AcceptRequests.text(lang),
                    social.requests_open,
                    SheetPress::Requests,
                ));
                kids.push(switch(
                    &mut commands,
                    &fonts,
                    lang,
                    ChatPhrase::ShareTable.text(lang),
                    social.share_table,
                    SheetPress::ShareTable,
                ));
            }
            let history = if chat.history_days == 0 {
                ChatPhrase::HistoryOff.text(lang).to_string()
            } else {
                ChatPhrase::HistoryDays.fill(lang, &[&chat.history_days.to_string()])
            };
            let row = line(&mut commands);
            let name = words(
                &mut commands,
                &fonts,
                ChatPhrase::KeepHistory.text(lang).to_string(),
                tokens::INK,
                12.5,
            );
            commands.entity(name).insert(Node {
                flex_grow: 1.0,
                ..default()
            });
            let b = button(&mut commands, &fonts, &history, SheetPress::History, true);
            commands.entity(row).add_children(&[name, b]);
            kids.push(row);
            kids.push(switch(
                &mut commands,
                &fonts,
                lang,
                ChatPhrase::KindTimestamps.text(lang),
                chat.timestamps,
                SheetPress::Timestamps,
            ));
            kids.push(switch(
                &mut commands,
                &fonts,
                lang,
                ChatPhrase::ToastTextLabel.text(lang),
                chat.toast_text,
                SheetPress::ToastText,
            ));
            kids.push(words(
                &mut commands,
                &fonts,
                ChatPhrase::SoundsLabel.text(lang).to_string(),
                tokens::MUTED,
                11.5,
            ));
            let sounds = [
                (ChatPhrase::KindWhispers, chat.sounds.whisper),
                (ChatPhrase::SoundMention, chat.sounds.mention),
                (ChatPhrase::KindGroups, chat.sounds.group),
                (ChatPhrase::KindTable, chat.sounds.table),
                (ChatPhrase::SoundFriendOnline, chat.sounds.friend_online),
            ];
            let row = line(&mut commands);
            for (n, (phrase, on)) in sounds.into_iter().enumerate() {
                let b = button(
                    &mut commands,
                    &fonts,
                    phrase.text(lang),
                    SheetPress::Sound(u8::try_from(n).unwrap_or(0)),
                    on,
                );
                commands.entity(b).entry::<Node>().and_modify(|mut node| {
                    node.flex_grow = 1.0;
                    node.min_height = zpx(30.0);
                    node.align_items = AlignItems::Center;
                    node.justify_content = JustifyContent::Center;
                });
                commands.entity(row).add_child(b);
            }
            kids.push(row);
            kids.push(words(
                &mut commands,
                &fonts,
                ChatPhrase::YourDevices.text(lang).to_uppercase(),
                tokens::MUTED,
                11.0,
            ));
            for device in machine.own_devices() {
                let row = line(&mut commands);
                let id: String = device
                    .id
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(4)
                    .take(4)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ");
                let this = if device.this {
                    format!(" · {}", ChatPhrase::ThisDevice.text(lang))
                } else {
                    String::new()
                };
                let text = words(
                    &mut commands,
                    &fonts,
                    format!("{} · {} · {id}{this}", device.label, day(device.added_at)),
                    tokens::INK,
                    12.0,
                );
                commands.entity(text).insert(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                commands.entity(row).add_child(text);
                if !device.this && note.home != KeyHome::Central {
                    let b = button(
                        &mut commands,
                        &fonts,
                        ChatPhrase::RemoveButton.text(lang),
                        SheetPress::RemoveDevice(device.id.clone()),
                        false,
                    );
                    commands.entity(row).add_child(b);
                }
                kids.push(row);
            }
            if machine.encryption_note().home == KeyHome::Central {
                kids.push(words(
                    &mut commands,
                    &fonts,
                    ChatPhrase::CentralDeviceSignOut.text(lang).to_string(),
                    tokens::MUTED,
                    11.5,
                ));
            } else {
                let actions = line(&mut commands);
                let safety_btn = button(
                    &mut commands,
                    &fonts,
                    ChatPhrase::SafetyButton.text(lang),
                    SheetPress::Safety(machine.me().tag.clone()),
                    false,
                );
                let safety_btn = stretch(&mut commands, safety_btn);
                let forget = button(
                    &mut commands,
                    &fonts,
                    ChatPhrase::ForgetDevice.text(lang),
                    SheetPress::Forget,
                    ui.forget_armed,
                );
                let forget = stretch(&mut commands, forget);
                if ui.forget_armed {
                    let bg = palette::DANGER.with_alpha(0.22);
                    commands.entity(forget).insert((
                        BackgroundColor(bg),
                        BorderColor::all(palette::DANGER),
                        Feel::tinting_to(bg, tokens::HOVER),
                        crate::ambience::FeelBorder {
                            rest: palette::DANGER,
                            hot: palette::DANGER,
                            press: palette::DANGER,
                        },
                        super::hint_bundle(ChatPhrase::ForgetConfirm.text(lang)),
                    ));
                }
                commands.entity(actions).add_children(&[safety_btn, forget]);
                kids.push(actions);
            }
            let blocked = &machine.contacts().blocked;
            if !blocked.is_empty() {
                kids.push(words(
                    &mut commands,
                    &fonts,
                    ChatPhrase::BlockedPlayers.text(lang).to_uppercase(),
                    tokens::MUTED,
                    11.0,
                ));
                for person in blocked {
                    let row = line(&mut commands);
                    let name = words(
                        &mut commands,
                        &fonts,
                        person.handle.clone(),
                        tokens::INK,
                        12.0,
                    );
                    commands.entity(name).insert(Node {
                        flex_grow: 1.0,
                        ..default()
                    });
                    let b = button(
                        &mut commands,
                        &fonts,
                        ChatPhrase::UnblockButton.text(lang),
                        SheetPress::Unblock(person.tag.clone()),
                        false,
                    );
                    commands.entity(row).add_children(&[name, b]);
                    kids.push(row);
                }
            }
            kids.push(words(
                &mut commands,
                &fonts,
                home.text(lang).to_string(),
                tokens::MUTED,
                11.5,
            ));
        }
    }
    let list = commands
        .spawn((
            SheetList,
            crate::hud::Scrolls,
            keep,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: zpx(0.0),
                min_height: zpx(0.0),
                row_gap: zpx(10.0),
                padding: UiRect::all(zpx(20.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(list).add_children(&kids);
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
            n.right = zpx(4.0);
            n.top = zpx(8.0);
            n.bottom = zpx(8.0);
            n.width = zpx(5.0);
        });
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: zpx(0.0),
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
            zpx(tokens::RADIUS_PANEL),
        ))
        .id();
    commands.entity(card).add_children(&[head, body, rim]);
}

#[cfg(test)]
mod tests;
