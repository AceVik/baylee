//! The retained report footer and its sending state.

#[allow(clippy::wildcard_imports)]
use super::*;

/// The mutable entities of one compact footer button.
struct FootButton {
    root: Entity,
    face: Entity,
    icon: Option<Entity>,
    words: Entity,
}

/// A compact button with the kit's full-size hit area.
fn footer_button(
    commands: &mut Commands,
    kit: Kit,
    icon: Option<char>,
    text: &str,
    weight: Weight,
    keys: Option<&str>,
    action: impl Bundle,
) -> FootButton {
    let ground = match weight {
        Weight::Primary => tokens::PRIMARY,
        _ => tokens::CONTROL,
    };
    let rim = match weight {
        Weight::Primary => tokens::PRIMARY_EDGE,
        _ => tokens::BORDER,
    };
    let face = commands
        .spawn((
            Role::Button,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: px_fixed(28.0),
                padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(rim),
        ))
        .id();
    controls::attach_live_feel(commands, face, ground, weight);
    let icon = icon.map(|glyph| {
        commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(kit.fonts, kit.m.small * 0.9),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id()
    });
    if let Some(icon) = icon {
        commands.entity(face).add_child(icon);
    }
    let words = commands
        .spawn((
            Text::new(text),
            tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(words);
    if let Some(cap) = keys.and_then(|keys| controls::key_cap(commands, kit, keys)) {
        commands.entity(face).add_child(cap);
    }
    let root = controls::hit(commands, kit, face, action);
    commands
        .entity(root)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_grow = 1.0;
            node.flex_shrink = 1.0;
            node.flex_basis = px_fixed(0.0);
            node.min_width = kit.m.px(96.0);
        });
    FootButton {
        root,
        face,
        icon,
        words,
    }
}

/// Send's key cap: the chord as the shell's keymap writes one ("Cmd+Enter",
/// "Ctrl+Enter"). It said "⌘↵" and "Ctrl ↵" before, and Alegreya Sans has
/// neither glyph, so the cap stood empty beside the word (feedback of
/// 28.10.2026, the buttons under the report's text).
pub(crate) fn send_keys(mac: bool) -> String {
    use baylee_client_core::shellkeys::{Learnt, ShellChord};
    ShellChord::code("Enter")
        .cmd()
        .display(mac, &Learnt::default())
}

/// Why Send is unavailable, if it is.
fn send_reason(
    desk: &ReportDesk,
    route: &Route,
    consent: &baylee_client_core::bugreport::Consent,
    lang: Lang,
) -> Option<String> {
    match route {
        Route::Nowhere => Some(Phrase::ReportNowhere.text(lang).to_string()),
        _ if desk.form.status == Status::Sending => {
            Some(Phrase::ReportSending.text(lang).to_string())
        }
        _ if desk.preparing(consent) => Some(Phrase::ReportPreparing.text(lang).to_string()),
        _ if desk.form.over_limit() => {
            Some(Phrase::ReportTextTooLong.fill(lang, &[&MAX_TEXT_CHARS.to_string()]))
        }
        _ if desk.form.text.text().trim().is_empty() => {
            Some(Phrase::ReportTextHint.text(lang).to_string())
        }
        _ => None,
    }
}

/// The retained footer: a visible disclosure, Copy as text where a report
/// can go nowhere, Close and Send. Only its contents change after this.
pub(super) fn form_foot(
    commands: &mut Commands,
    desk: &ReportDesk,
    route: &Route,
    kit: Kit,
    lang: Lang,
) -> Vec<Entity> {
    let what = footer_button(
        commands,
        kit,
        Some(if desk.form.preview {
            '\u{f078}'
        } else {
            '\u{f054}'
        }),
        if desk.form.preview {
            Phrase::ReportPreviewHide.text(lang)
        } else {
            Phrase::ReportPreviewShow.text(lang)
        },
        Weight::Secondary,
        None,
        (DeskPress::Preview, Stop::new(REPORT.name, "what-is-sent")),
    );
    commands.entity(what.words).insert(PreviewWords);
    if let Some(icon) = what.icon {
        commands.entity(icon).insert(PreviewMark);
    }
    let mut footer = vec![what.root];
    if matches!(route, Route::Nowhere) {
        let copy = footer_button(
            commands,
            kit,
            None,
            Phrase::CopyAsText.text(lang),
            Weight::Secondary,
            None,
            (DeskPress::CopyText, Stop::new(REPORT.name, "copy")),
        );
        footer.push(copy.root);
    }
    let close = footer_button(
        commands,
        kit,
        None,
        Phrase::ReportClose.text(lang),
        Weight::Secondary,
        Some("Esc"),
        (DeskPress::Close, Stop::new(REPORT.name, "close")),
    );
    footer.push(close.root);
    let send_keys = send_keys(crate::shellkit::keys::mac());
    let send = footer_button(
        commands,
        kit,
        Some(crate::hud::glyph::SEND),
        Phrase::ReportSend.text(lang),
        Weight::Primary,
        Some(&send_keys),
        (DeskPress::Send, Stop::new(REPORT.name, "send")),
    );
    commands
        .entity(send.root)
        .insert(SendButton { reason: None });
    commands.entity(send.face).insert(SendFace);
    commands.entity(send.words).insert(SendWords);
    if let Some(icon) = send.icon {
        commands.entity(icon).insert(SendMark);
    }
    let spinner = crate::card_loading::spinner(commands, kit.m.small);
    commands
        .entity(spinner)
        .insert((SendSpinner, Visibility::Hidden));
    commands.entity(send.face).add_child(spinner);
    footer.push(send.root);
    footer
}

/// Locks every report-changing control while its one request is in flight.
#[allow(clippy::too_many_arguments)]
pub(in crate::report) fn lock_while_sending(
    mut commands: Commands,
    desk: Res<ReportDesk>,
    settings: Option<Res<ClientSettings>>,
    fonts: Option<Res<UiFonts>>,
    input: Option<Res<InputClass>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    ui: Option<Res<UiScale>>,
    mut controls: Query<(Entity, &DeskPress, Has<Disabled>)>,
) {
    let sending = desk.open && desk.form.status == Status::Sending;
    let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let kit = fonts.as_deref().map(|f| {
        let window = windows.single().map_or(Vec2::new(1280.0, 800.0), |w| {
            Vec2::new(w.width(), w.height())
        });
        kit_for(
            f,
            window,
            input.as_deref().copied().unwrap_or(InputClass::Pointer),
            lang,
            ui.as_deref().map_or(1.0, |ui| ui.0),
        )
    });
    for (entity, press, disabled) in &mut controls {
        if matches!(press, DeskPress::Send) {
            continue; // sync_footer owns Send, including its preparation/validation state.
        }
        let lock = sending && press.locked_while_sending();
        match (lock, disabled) {
            (true, false) => {
                commands.entity(entity).try_insert(Disabled);
                if let Some(kit) = kit {
                    controls::attach_tooltip(
                        &mut commands,
                        entity,
                        kit,
                        Phrase::ReportSending.text(lang),
                    );
                }
            }
            (false, true) => {
                commands.entity(entity).try_remove::<(Disabled, Tooltip)>();
            }
            _ => {}
        }
    }
}

/// Patches the retained footer after typing, a response, or a disclosure.
#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::too_many_lines
)] // a Bevy system and the footer's separate parts
pub(in crate::report) fn sync_footer(
    mut commands: Commands,
    desk: Res<ReportDesk>,
    settings: Res<ClientSettings>,
    lobby: Option<Res<LobbyState>>,
    fonts: Option<Res<UiFonts>>,
    input: Option<Res<InputClass>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    ui: Option<Res<UiScale>>,
    mut send: Query<(Entity, &mut SendButton, Has<Disabled>)>,
    mut faces: Query<(&mut BackgroundColor, &mut BorderColor), With<SendFace>>,
    mut words: Query<(&mut Text, &mut TextColor), With<SendWords>>,
    mut icons: Query<
        (&mut Visibility, &mut TextColor),
        (With<SendMark>, Without<SendSpinner>, Without<SendWords>),
    >,
    mut spinners: Query<&mut Visibility, (With<SendSpinner>, Without<SendMark>)>,
    mut preview_words: Query<
        &mut Text,
        (With<PreviewWords>, Without<SendWords>, Without<PreviewMark>),
    >,
    mut preview_marks: Query<
        &mut Text,
        (With<PreviewMark>, Without<SendWords>, Without<PreviewWords>),
    >,
) {
    if !desk.open || desk.form.confirming {
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    let route = crate::report::route(lobby.as_deref(), &settings);
    let reason = send_reason(&desk, &route, &settings.reports, lang);
    let sending = desk.form.status == Status::Sending;
    let window = windows.single().map_or(Vec2::new(1280.0, 800.0), |w| {
        Vec2::new(w.width(), w.height())
    });
    let kit = kit_for(
        &fonts,
        window,
        input.as_deref().copied().unwrap_or(InputClass::Pointer),
        lang,
        ui.as_deref().map_or(1.0, |ui| ui.0),
    );
    for (entity, mut button, disabled) in &mut send {
        if button.reason != reason {
            button.reason.clone_from(&reason);
            if let Some(reason) = &reason {
                controls::attach_tooltip(&mut commands, entity, kit, reason);
            } else {
                commands.entity(entity).try_remove::<Tooltip>();
            }
        }
        match (reason.is_some(), disabled) {
            (true, false) => {
                commands.entity(entity).try_insert(Disabled);
            }
            (false, true) => {
                commands.entity(entity).try_remove::<Disabled>();
            }
            _ => {}
        }
    }
    let (ground, rim, ink) = if reason.is_some() {
        (
            tokens::CONTROL.with_alpha(0.6),
            tokens::BORDER,
            tokens::DISABLED,
        )
    } else {
        (tokens::PRIMARY, tokens::PRIMARY_EDGE, tokens::INK)
    };
    for (mut ground_now, mut rim_now) in &mut faces {
        if ground_now.0 != ground {
            ground_now.0 = ground;
        }
        if *rim_now != BorderColor::all(rim) {
            *rim_now = BorderColor::all(rim);
        }
    }
    let send_words = if sending {
        Phrase::ReportSending.text(lang)
    } else {
        Phrase::ReportSend.text(lang)
    };
    for (mut text, mut colour) in &mut words {
        if text.0 != send_words {
            text.0 = send_words.to_string();
        }
        if colour.0 != ink {
            colour.0 = ink;
        }
    }
    for (mut shown, mut colour) in &mut icons {
        shown.set_if_neq(if sending {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
        if colour.0 != ink {
            colour.0 = ink;
        }
    }
    for mut shown in &mut spinners {
        shown.set_if_neq(if sending {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    let disclosure = if desk.form.preview {
        Phrase::ReportPreviewHide.text(lang)
    } else {
        Phrase::ReportPreviewShow.text(lang)
    };
    for mut text in &mut preview_words {
        if text.0 != disclosure {
            text.0 = disclosure.to_string();
        }
    }
    let chevron = if desk.form.preview {
        "\u{f078}"
    } else {
        "\u{f054}"
    };
    for mut text in &mut preview_marks {
        if text.0 != chevron {
            text.0 = chevron.to_string();
        }
    }
}

/// Kept for the report plugin's system tuple; disabled Send reasons now live
/// in the floating [`Tooltip`] managed by [`sync_footer`].
pub(in crate::report) fn sync_touch_reason() {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn the_send_buttons_disabled_state_belongs_to_the_footer() {
        let mut world = World::new();
        world.insert_resource(ReportDesk::default());
        let send = world.spawn((DeskPress::Send, Disabled)).id();
        world.run_system_once(lock_while_sending).unwrap();
        assert!(
            world.get::<Disabled>(send).is_some(),
            "the generic lock unlocked Send"
        );
    }
    #[test]
    fn sending_waits_for_a_selected_screenshot_but_not_an_excluded_one() {
        let desk = ReportDesk {
            shooting: true,
            ..default()
        };
        let mut consent = baylee_client_core::bugreport::Consent::nothing();
        assert!(!desk.preparing(&consent));
        consent.set(Category::Screenshot, true);
        assert!(desk.preparing(&consent));
    }
}
