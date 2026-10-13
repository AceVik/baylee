//! The chat's sheets (the beta.8 social design §11.8): the end-to-end
//! encryption notice (shown the first time the chat opens on a device, and
//! from every lock), a contact's safety number and devices, and the chat's
//! privacy settings (who may whisper, requests, sharing the table, history,
//! sounds, this account's devices, blocked players).

use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{ChatPhrase, ConvId, Intent, KeyHome, Social};
use baylee_protocol::social::{SocialSettings, WhispersFrom};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use super::window::{ChatSurface, G_PANEL};
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

/// A press on a sheet.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SheetPress {
    Close,
    GotIt,
    Safety(String),
    Verify(String, bool),
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
    app.add_systems(
        Update,
        (first_run, presses, sync)
            .chain()
            .run_if(resource_exists::<UiFonts>),
    );
}

/// The first time the chat opens on a device, the notice stands first.
fn first_run(host: Res<SocialHost>, mut ui: ResMut<ChatUi>) {
    let Some(machine) = host.machine() else {
        return;
    };
    if machine.encryption_note().seen || ui.sheet.is_some() {
        return;
    }
    if (ui.open && ui.tab.is_chat()) || ui.drawer {
        ui.sheet = Some(Sheet::Encryption(None));
    }
}

#[allow(clippy::too_many_lines)] // one arm per control
fn presses(
    pressed: Query<(Entity, &Interaction, &SheetPress), Changed<Interaction>>,
    mut held: Local<std::collections::HashSet<Entity>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    mut prefs: Option<ResMut<crate::prefs::Prefs>>,
    mut forget_armed: Local<bool>,
) {
    let clicked: Vec<SheetPress> = super::released(&pressed, &mut held);
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
            SheetPress::Close => ui.sheet = None,
            SheetPress::GotIt => {
                host.act(Intent::NoticeSeen);
                if let Some(chat) = host.machine().map(|m| m.prefs().clone()) {
                    super::window::save_prefs(&mut host, prefs.as_deref_mut(), chat);
                }
                ui.sheet = None;
            }
            SheetPress::Safety(tag) => ui.sheet = Some(Sheet::Safety(tag)),
            SheetPress::Verify(tag, on) => host.act(if on {
                Intent::Verify { tag }
            } else {
                Intent::Unverify { tag }
            }),
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
    commands
        .spawn((
            Button,
            press,
            Node {
                padding: UiRect::axes(px(10), px(4)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if lit {
                tokens::ACCENT.with_alpha(0.25)
            } else {
                Color::NONE
            }),
            BorderColor::all(if lit { tokens::ACCENT } else { tokens::BORDER }),
            Feel::new(Color::NONE),
            children![(
                Text::new(label.to_string()),
                tf(fonts, 12.0),
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
            tf(fonts, size),
            TextColor(colour),
            Pickable::IGNORE,
        ))
        .id()
}

fn line(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(8),
                row_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
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
            tf(fonts, 12.5),
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
                    tf_bold(fonts, 15.0),
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
            SheetPress::Verify(tag.to_string(), false),
            false,
        )
    } else {
        button(
            commands,
            fonts,
            ChatPhrase::MarkVerified.text(lang),
            SheetPress::Verify(tag.to_string(), true),
            true,
        )
    };
    out.push(verify);
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
    mut drawn: Local<Option<(u64, String, bool)>>,
) {
    let Some((machine, sheet)) = host.machine().zip(ui.sheet.as_ref()) else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = None;
        return;
    };
    let key = (machine.revision(), format!("{sheet:?}"), ui.forget_armed);
    if drawn.as_ref() == Some(&key) && !roots.is_empty() {
        return;
    }
    *drawn = Some(key);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let scrim = commands
        .spawn((
            SheetRoot,
            ChatSurface,
            RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            GlobalZIndex(G_PANEL + 50),
        ))
        .id();
    let card = commands
        .spawn((
            Node {
                width: px(560),
                max_height: Val::Percent(88.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(20)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(super::rows::LOBBY_GROUND),
            BorderColor::all(tokens::BORDER),
            crate::hud::Scrolls,
            bevy::ui::ScrollPosition::default(),
        ))
        .id();
    commands.entity(scrim).add_child(card);
    let title = match sheet {
        Sheet::Encryption(_) | Sheet::Safety(_) => ChatPhrase::EncryptionTitle,
        Sheet::Privacy => ChatPhrase::PrivacyTitle,
    };
    let head = line(&mut commands);
    let name = commands
        .spawn((
            Text::new(format!("{} {}", glyph::LOCK, title.text(lang))),
            tf_bold(&fonts, 16.0),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(name).insert(TextFont {
        ..tf_bold(&fonts, 16.0)
    });
    let close = commands
        .spawn((
            Button,
            SheetPress::Close,
            Node {
                width: px(26),
                height: px(26),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::new(Color::NONE),
            children![(
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(&fonts, 11.0),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands.entity(head).add_children(&[name, close]);
    let mut kids = vec![head];
    let note = machine.encryption_note();
    let home = match note.home {
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
                kids.push(button(
                    &mut commands,
                    &fonts,
                    ChatPhrase::GotIt.text(lang),
                    SheetPress::GotIt,
                    true,
                ));
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
                    _ => machine
                        .conversation(conv)
                        .map(|c| {
                            c.members
                                .iter()
                                .filter(|p| p.tag != me)
                                .map(|p| p.tag.clone())
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                if let [only] = partners.as_slice() {
                    let block = safety_block(&mut commands, &fonts, lang, machine, only);
                    kids.extend(block);
                } else {
                    let row = line(&mut commands);
                    for tag in partners {
                        let b = button(
                            &mut commands,
                            &fonts,
                            &machine.person(&tag).handle,
                            SheetPress::Safety(tag),
                            false,
                        );
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
                let b = button(
                    &mut commands,
                    &fonts,
                    ChatPhrase::WhispersTables.text(lang),
                    SheetPress::WhispersFrom(WhispersFrom::FriendsAndTables),
                    social.whispers_from == WhispersFrom::FriendsAndTables,
                );
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
                if !device.this {
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
            let forget = button(
                &mut commands,
                &fonts,
                ChatPhrase::ForgetDevice.text(lang),
                SheetPress::Forget,
                ui.forget_armed,
            );
            kids.push(forget);
            if ui.forget_armed {
                kids.push(words(
                    &mut commands,
                    &fonts,
                    ChatPhrase::ForgetConfirm.text(lang).to_string(),
                    palette::DANGER,
                    12.0,
                ));
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
    commands.entity(card).add_children(&kids);
}
