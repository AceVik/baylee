//! The front door's faces (`DESIGN-v5` §3): the gateway picker, and at a
//! chosen gateway the three account faces — sign in, create an account,
//! play as a guest.
//!
//! Each face is built from its `TabOrder` (`front::keys`): every control it
//! draws carries the stop of that name, and the walker visits them in the
//! table's order. The beta key opens under the button chosen (the create
//! and guest faces), never on the sign-in face; Sign in is the one primary
//! (the blue face) right under the fields.

use super::super::field::{FieldStops, text_field_with};
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::super::*;
use super::FrontPress;
use super::keys;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};
use crate::tour::TourAnchor;
use baylee_client_core::tour::Anchor;

/// The lobby's own field metrics, read off the kit's (the text step and
/// the size class): the fields are the lobby's editor, sized by the shell.
pub(crate) fn field_metrics(kit: Kit) -> Metrics {
    Metrics {
        frame: kit.m.frame,
        text: kit.m.text,
        head: kit.m.head,
        small: kit.m.small,
        tap: kit.m.control,
        pad: kit.m.pad,
        gap: kit.m.gap,
    }
}

/// Whether the face sets two things to a row: only a phone, whose form has
/// to end above the keyboard line (§2.7). Everywhere else one input stands
/// on each line (owner, 09.10.2026: the create and guest faces looked
/// squeezed), and the sign-in face's two doors share a row
/// ([`doors_side_by_side`]).
fn two_columns(kit: Kit) -> bool {
    kit.m.frame == Frame::Phone
}

/// The sign-in face's two doors side by side, as before, on every class
/// but the smallest window.
fn doors_side_by_side(kit: Kit) -> bool {
    kit.m.frame != Frame::Compact
}

/// A column of rows at the face's gap.
fn column(commands: &mut Commands, kit: Kit, gap: f32) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// Two cells side by side, each half the row; one above the other on the
/// smallest window.
fn pair(commands: &mut Commands, kit: Kit, left: Entity, right: Option<Entity>) -> Entity {
    pair_if(commands, kit, two_columns(kit), left, right)
}

/// [`pair`], side by side when `side` says so.
fn pair_if(
    commands: &mut Commands,
    kit: Kit,
    side: bool,
    left: Entity,
    right: Option<Entity>,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(10.0),
                flex_direction: if side {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                align_items: AlignItems::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for cell in std::iter::once(left).chain(right) {
        commands
            .entity(cell)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_basis = px_fixed(0.0);
                node.flex_grow = 1.0;
                node.min_width = px_fixed(0.0);
                node.width = Val::Percent(100.0);
            });
        commands.entity(row).add_child(cell);
    }
    row
}

/// A hairline across the card.
pub(super) fn rule(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: px_fixed(1.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id()
}

/// A lobby text field with its caption, its stops, and help under it.
#[allow(clippy::too_many_arguments)] // one field: what it is, what it shows, how it is reached
fn field(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    which: Field,
    caption: Phrase,
    placeholder: Option<&str>,
    help: Option<&str>,
    eye: Option<&'static str>,
) -> Entity {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let look = FieldLook {
        buffer: lobby.buffer(which),
        focused: lobby.caret_in(which),
        mask: eye.map(|_| Masked {
            field: Some(which),
            shown: lobby.showing(which),
        }),
        press: Press::Shared(SharedPress::Focus(which)),
        lead: None,
        hint: placeholder,
        tail: None,
    };
    let stops = keys::field_stop(state, which).map(|field| FieldStops {
        field,
        typed: Some(which),
        eye: eye.map(|id| keys::stop(state, id)),
    });
    let boxed = text_field_with(
        commands,
        kit.fonts,
        field_metrics(kit),
        caption.text(lang),
        &look,
        stops,
    );
    let mut lines: Vec<Entity> = Vec::new();
    // Caps Lock, inferred from a letter's case against Shift (S4-6), under
    // the password that is being typed.
    if eye.is_some() && state.caps_lock && lobby.caret_in(which) {
        let warn = commands
            .spawn((
                Text::new(format!("\u{26a0} {}", Phrase::FrontCapsLock.text(lang))),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(tokens::GOLD),
                Pickable::IGNORE,
            ))
            .id();
        lines.push(warn);
    }
    if let Some(help) = help {
        lines.push(surfaces::prose(commands, kit, help, true));
    }
    if lines.is_empty() {
        return boxed;
    }
    let cell = column(commands, kit, 4.0);
    commands.entity(cell).add_child(boxed);
    commands.entity(cell).add_children(&lines);
    cell
}

/// Help under a field; a phone, short of height, keeps the form above
/// the keyboard and says it nowhere (the hints stay on the wider classes).
fn help<'t>(kit: Kit, text: &'t str) -> Option<&'t str> {
    (kit.m.frame != Frame::Phone).then_some(text)
}

/// Tracks the active status banner's entrance progress across UI rebuilds so
/// that background state updates never restart its animation or cause flicker.
#[derive(Resource, Default)]
pub(crate) struct StatusBannerMemory {
    text: String,
    progress: f32,
}

/// Smooth entrance motion (`0.0..=1.0`) for an inline status or error banner.
#[derive(Component)]
pub(crate) struct StatusBannerMotion {
    base_bg: Color,
    base_border: Color,
}

/// Animates inline status/refusal banners smoothly into view.
pub(crate) fn animate_status_banners(
    time: Res<Time>,
    state: Res<LobbyState>,
    mut memory: ResMut<StatusBannerMemory>,
    mut banners: Query<(
        &StatusBannerMotion,
        &mut UiTransform,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let said = state.lobby.status();
    if said.is_empty() {
        if !memory.text.is_empty() {
            memory.text.clear();
            memory.progress = 0.0;
        }
        return;
    }
    if memory.text != said {
        memory.text = said.to_string();
        memory.progress = 0.0;
    }
    let dt = time.delta_secs().min(0.05);
    if memory.progress < 1.0 {
        memory.progress = (memory.progress + dt / 0.14).min(1.0);
    }
    let ease = 1.0 - (1.0 - memory.progress).powi(3);
    for (motion, mut transform, mut bg, mut border) in &mut banners {
        transform.translation = Val2::new(px_fixed(0.0), px_fixed((1.0 - ease) * -6.0));
        transform.scale = Vec2::new(0.98 + 0.02 * ease, 0.92 + 0.08 * ease);
        bg.0 = motion.base_bg.with_alpha(motion.base_bg.alpha() * ease);
        *border = BorderColor::all(
            motion
                .base_border
                .with_alpha(motion.base_border.alpha() * ease),
        );
    }
}

/// The status line (a refusal, verbatim; a note), only while it says
/// something: no reserved hole under the button (A12).
fn status(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Option<Entity> {
    let said = state.lobby.status();
    if said.is_empty() {
        return None;
    }
    let is_refusal = state.lobby.tone() == Tone::Refusal;
    let ink = if is_refusal {
        Color::srgba(0.98, 0.56, 0.56, 1.0)
    } else {
        tokens::MUTED
    };
    let bg = if is_refusal {
        Color::srgba(0.32, 0.08, 0.10, 0.52)
    } else {
        Color::srgba(0.06, 0.11, 0.17, 0.55)
    };
    let border = if is_refusal {
        tokens::DANGER.with_alpha(0.42)
    } else {
        tokens::BORDER.with_alpha(0.45)
    };
    let banner = commands
        .spawn((
            StatusBannerMotion {
                base_bg: bg,
                base_border: border,
            },
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(11.0), kit.m.px(7.0)),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                ..default()
            },
            UiTransform::default(),
            BackgroundColor(bg),
            BorderColor::all(border),
            Pickable::IGNORE,
        ))
        .id();
    let said_owned = said.to_string();
    commands.queue(move |world: &mut World| {
        let ease = world
            .get_resource::<StatusBannerMemory>()
            .map_or(1.0, |mem| {
                if mem.text == said_owned {
                    1.0 - (1.0 - mem.progress).powi(3)
                } else {
                    0.0
                }
            });
        if ease < 1.0
            && let Ok(mut ent) = world.get_entity_mut(banner)
        {
            if let Some(mut transform) = ent.get_mut::<UiTransform>() {
                transform.translation = Val2::new(px_fixed(0.0), px_fixed((1.0 - ease) * -6.0));
                transform.scale = Vec2::new(0.98 + 0.02 * ease, 0.92 + 0.08 * ease);
            }
            if let Some(mut bg_col) = ent.get_mut::<BackgroundColor>() {
                bg_col.0 = bg.with_alpha(bg.alpha() * ease);
            }
            if let Some(mut border_col) = ent.get_mut::<BorderColor>() {
                *border_col = BorderColor::all(border.with_alpha(border.alpha() * ease));
            }
        }
    });
    let glyph = if is_refusal { "\u{f06a}" } else { "\u{f05a}" };
    let icon = commands
        .spawn((
            Text::new(glyph),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.86),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(said),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 0.88),
            TextColor(ink),
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(banner).add_children(&[icon, words]);
    Some(banner)
}

/// An icon-only control in the kit's hit area: `‹`, the back chevron.
fn glyph_button(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    hint: &str,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_width: px_fixed(kit.m.control * 0.8),
                min_height: px_fixed(kit.m.control * 0.8),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            super::super::hint::HoverHint(hint.to_string()),
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(mark);
    let wrapper = controls::hit(commands, kit, face, action);
    commands.entity(face).insert(Pickable::default());
    wrapper
}

/// The gateway's badge for a closed beta.
fn beta_badge(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let badge = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(2.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::GOLD.with_alpha(0.18)),
            Pickable::IGNORE,
        ))
        .id();
    let words = controls::label(
        commands,
        kit,
        Phrase::FrontClosedBeta.text(lang),
        kit.m.small,
        tokens::GOLD,
    );
    commands.entity(badge).add_child(words);
    badge
}

/// A reach dot.
fn dot(commands: &mut Commands, kit: Kit, colour: Color) -> Entity {
    commands
        .spawn((
            Node {
                width: kit.m.px(8.0),
                height: kit.m.px(8.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(colour),
            Pickable::IGNORE,
        ))
        .id()
}

fn is_central_login_face(state: &LobbyState) -> bool {
    let leaving_central = !state.lobby.gateway_chosen() && state.left_central_bound;
    state.central_pending()
        || leaving_central
        || (!state.lobby.gateway_chosen() && state.central_ready())
}

#[allow(clippy::too_many_lines)] // one head: the way back, the name over its address, the version
/// The sign-in face's head: on Central, the `Baylee Authentifizierung` title
/// lockup and the top-right utility bar (`Music`, `About`, `Language`,
/// `Settings`); at a direct gateway, `‹` back to the gateways, the reach dot,
/// the gateway's name + version, and the top-right utility bar.
fn gateway_head(commands: &mut Commands, state: &LobbyState, kit: Kit, _music_on: bool) -> Entity {
    let leaving_central = !state.lobby.gateway_chosen() && state.left_central_bound;
    if is_central_login_face(state) {
        let lang = state.lobby.lang();
        let auth_title = match lang {
            Lang::De => "Baylee Authentifizierung",
            Lang::En => "Baylee Authentication",
        };
        let title = super::door::card_title_lockup(commands, kit, '\u{f3ed}', auth_title);
        commands
            .entity(title)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_grow = 1.0;
                node.min_width = px_fixed(0.0);
            });
        let head = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    column_gap: kit.m.px(10.0),
                    row_gap: kit.m.px(6.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        if state.central_bound() || leaving_central {
            // A gateway asked for Central: `‹` goes back to the device's own.
            let back = glyph_button(
                commands,
                kit,
                '\u{f053}',
                Phrase::ChooseGateway.text(lang),
                (
                    Press::Front(if state.lobby.busy() {
                        FrontPress::Nothing
                    } else {
                        FrontPress::LeaveGateway
                    }),
                    keys::stop(state, "back"),
                ),
            );
            commands.entity(head).add_children(&[back, title]);
        } else {
            commands.entity(head).add_child(title);
        }
        return head;
    }
    let lang = state.lobby.lang();
    let words =
        super::super::gateway::row_words(&state.gateway, state.probes.get(&state.gateway), lang);
    let phone = kit.m.frame == Frame::Phone;
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(10.0),
                row_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
            TourAnchor(Anchor::FrontGatewayRow),
        ))
        .id();
    let busy = state.lobby.busy();
    let back = glyph_button(
        commands,
        kit,
        '\u{f053}',
        Phrase::ChooseGateway.text(lang),
        (
            Press::Front(if busy {
                FrontPress::Nothing
            } else {
                FrontPress::LeaveGateway
            }),
            keys::stop(state, "back"),
        ),
    );
    let names = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let line = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let reach = dot(commands, kit, words.reach);
    let name = commands
        .spawn((
            Text::new(words.title.clone()),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.06),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_children(&[reach, name]);
    if state.lobby.registration() == Registration::Invite {
        let badge = beta_badge(commands, kit, lang);
        commands.entity(line).add_child(badge);
    }
    commands.entity(names).add_child(line);
    if let Some(address) = words.address.as_ref().filter(|_| !phone) {
        let at = controls::label(commands, kit, address, kit.m.small, tokens::MUTED);
        commands.entity(names).add_child(at);
    }
    let verdict = match words.warning {
        Some(warning) => warning.explain(lang),
        None if matches!(state.probes.get(&state.gateway), Some(Probe::Known(_))) => {
            Phrase::FrontCompatible.text(lang).to_string()
        }
        None => String::new(),
    };
    let right = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                flex_shrink: 0.0,
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(4.0),
                ..default()
            },
            super::super::hint::HoverHint(words.version_in_full.clone()),
        ))
        .id();
    let version = commands
        .spawn((
            Text::new(if verdict.is_empty() {
                words.version.clone()
            } else {
                format!("{} \u{b7} {verdict}", words.version)
            }),
            crate::hud::tf(kit.fonts, kit.m.small * 0.90),
            TextColor(words.ink),
            TextLayout::justify(Justify::Right),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(right).add_child(version);
    commands.entity(head).add_children(&[back, names, right]);
    head
}

/// A second face's head: `‹ Back`, the face's title, and the gateway chip.
fn face_head(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    title: &str,
    _music_on: bool,
) -> Entity {
    let lang = state.lobby.lang();
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let back = controls::button(
        commands,
        kit,
        &format!("\u{2039} {}", Phrase::Back.text(lang)),
        Weight::Secondary,
        Live::Yes,
        Some("Esc"),
        (
            Press::Front(FrontPress::BackToSignIn),
            keys::stop(state, "back"),
        ),
    );
    // The title whole, never cut: it wraps before it is clipped.
    let words = commands
        .spawn((
            Text::new(title),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Node {
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let gateway =
        super::super::gateway::row_words(&state.gateway, state.probes.get(&state.gateway), lang);
    // Pushed to the right end of its line, which is the head's first when
    // there is room and a line of its own when there is not.
    let chip = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                flex_shrink: 1.0,
                flex_wrap: FlexWrap::Wrap,
                max_width: Val::Percent(100.0),
                margin: UiRect::left(Val::Auto),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let reach = dot(commands, kit, gateway.reach);
    let name = commands
        .spawn((
            Text::new(gateway.title.clone()),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            Node {
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(chip).add_children(&[reach, name]);
    if state.lobby.registration() == Registration::Invite {
        let badge = beta_badge(commands, kit, lang);
        commands.entity(chip).add_child(badge);
    }
    commands.entity(head).add_children(&[back, words, chip]);
    head
}

/// The gateway did not answer: a red line naming it, and Retry (§2.5).
fn unreachable_line(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Option<Entity> {
    if !matches!(state.probes.get(&state.gateway), Some(Probe::Silent)) {
        return None;
    }
    let lang = state.lobby.lang();
    let title = super::super::gateway::title_of(state, &state.gateway);
    let retry = controls::button(
        commands,
        kit,
        Phrase::FrontRetry.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (
            Press::Front(FrontPress::RetryGateway),
            keys::stop(state, "retry"),
        ),
    );
    Some(crate::shellkit::states::error_line(
        commands,
        kit,
        &Phrase::FrontUnreachable.fill(lang, &[&title]),
        retry,
    ))
}

/// The face's primary: the blue face, as wide as the card, with Enter's
/// cap under a pointer.
fn submit(commands: &mut Commands, state: &LobbyState, kit: Kit, text: &str) -> Entity {
    let lobby = &state.lobby;
    // A busy form says why in the status line under it already; a gateway
    // not chosen explicitly signs nothing in (#187).
    let live = if lobby.busy() || (!state.gateway_selected && !is_central_login_face(state)) {
        Live::No("")
    } else {
        Live::Yes
    };
    let button = controls::wide_button(
        commands,
        kit,
        text,
        Weight::Primary,
        live,
        Some("Enter"),
        (
            Press::Front(FrontPress::Submit),
            keys::stop(state, "submit"),
        ),
    );
    commands.entity(button).insert(super::AccountSubmit);
    button
}

/// The sign-in face: the gateway/Central header, username and password,
/// Sign in; under a rule, Register + Play offline (or Create account +
/// Play as guest + Play offline).
#[allow(clippy::too_many_lines)] // one face, read top to bottom
pub(super) fn sign_in(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let head = gateway_head(commands, state, kit, music_on);
    let line = rule(commands);
    commands.entity(card).add_children(&[head, line]);
    if state.central_ready() && lobby.gateway_chosen() {
        if let Some(said) = status(commands, state, kit) {
            commands.entity(card).add_child(said);
        }
        if let Some(index) = state.gateways.iter().position(|g| g == &state.gateway) {
            let retry = controls::wide_button(
                commands,
                kit,
                Phrase::SignIn.text(lang),
                Weight::Primary,
                Live::Yes,
                None,
                (
                    Press::Front(FrontPress::SelectGateway(index)),
                    keys::stop(state, "submit"),
                ),
            );
            commands.entity(card).add_child(retry);
        }
        return;
    }
    if let Some(gone) = unreachable_line(commands, state, kit) {
        commands.entity(card).add_child(gone);
    }
    let username = field(
        commands,
        state,
        kit,
        Field::Username,
        Phrase::Username,
        None,
        None,
        None,
    );
    let password = field(
        commands,
        state,
        kit,
        Field::Password,
        Phrase::Password,
        None,
        None,
        Some("eye"),
    );
    // A phone sets the two side by side, so the form ends above the
    // keyboard (§2.7); a wider card stacks them, as every sign-in does.
    if kit.m.frame == Frame::Phone {
        let both = pair(commands, kit, username, Some(password));
        commands.entity(card).add_child(both);
    } else {
        commands.entity(card).add_children(&[username, password]);
    }
    let go = submit(commands, state, kit, Phrase::SignIn.text(lang));
    commands.entity(card).add_child(go);
    if let Some(said) = status(commands, state, kit) {
        commands.entity(card).add_child(said);
    }
    let create = lobby.registration_enabled().then(|| {
        controls::wide_button(
            commands,
            kit,
            Phrase::CreateAccount.text(lang),
            Weight::Secondary,
            if lobby.busy() {
                Live::No("")
            } else {
                Live::Yes
            },
            None,
            (
                Press::Front(FrontPress::ToggleRegistering),
                keys::stop(state, "create"),
            ),
        )
    });
    let guest = (lobby.guest_offered() && (!is_central_login_face(state) || state.central_bound()))
        .then(|| {
            let text = lobby.kept_guest().map_or_else(
                || Phrase::FrontGuestTitle.text(lang).to_string(),
                |kept| Phrase::ContinueAsGuest.fill(lang, &[&kept.handle]),
            );
            controls::wide_button(
                commands,
                kit,
                &text,
                Weight::Secondary,
                if lobby.busy() || !(state.gateway_selected || state.central_bound()) {
                    Live::No("")
                } else {
                    Live::Yes
                },
                None,
                (
                    Press::Front(FrontPress::GuestFace),
                    keys::stop(state, "guest"),
                    super::super::hint::HoverHint(Phrase::GuestNotice.text(lang).to_string()),
                ),
            )
        });
    // Central's sign-in has no sign-up of its own yet: an account is made on
    // the website (baylee-central `docs/web-accounts.md`).
    let register = is_central_login_face(state).then(|| {
        super::door::register_cta(
            commands,
            kit,
            Phrase::CentralRegister.text(lang),
            (
                Press::Front(FrontPress::OpenCentralRegister),
                keys::stop(state, "register"),
                super::super::hint::HoverHint(Phrase::CentralRegisterHint.text(lang).to_string()),
            ),
        )
    });
    let offline = super::door::offline_cta(
        commands,
        kit,
        Phrase::PlayOffline.text(lang),
        (
            Press::Front(FrontPress::PlayOffline),
            keys::stop(state, "offline"),
        ),
    );
    let mut doors: Vec<Entity> = create.into_iter().chain(guest).chain(register).collect();
    doors.push(offline);
    let line = rule(commands);
    let row = pair_if(
        commands,
        kit,
        doors_side_by_side(kit),
        doors[0],
        doors.get(1).copied(),
    );
    commands.entity(row).insert(TourAnchor(Anchor::FrontGuest));
    commands.entity(card).add_children(&[line, row]);
    if let Some(&third) = doors.get(2) {
        commands.entity(card).add_child(third);
    }
    let new_one =
        lobby.registration_enabled() || (lobby.guest_offered() && lobby.kept_guest().is_none());
    if lobby.registration() == Registration::Invite && new_one {
        let hint = surfaces::prose(commands, kit, Phrase::FrontKeyHint.text(lang), true);
        commands
            .entity(hint)
            .insert(TextLayout::justify(Justify::Center));
        commands.entity(card).add_child(hint);
    }
}

/// The create-account face: username and display name, the password twice,
/// the key on a closed beta, Create account.
pub(super) fn create(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let head = face_head(
        commands,
        state,
        kit,
        Phrase::CreateAccount.text(lang),
        music_on,
    );
    let line = rule(commands);
    commands.entity(card).add_children(&[head, line]);
    let username = field(
        commands,
        state,
        kit,
        Field::Username,
        Phrase::Username,
        None,
        help(kit, Phrase::FrontUsernameHint.text(lang)),
        None,
    );
    let display = field(
        commands,
        state,
        kit,
        Field::DisplayName,
        Phrase::DisplayName,
        None,
        help(kit, Phrase::AccountNameHint.text(lang)),
        None,
    );
    let names = pair(commands, kit, username, Some(display));
    let password = field(
        commands,
        state,
        kit,
        Field::Password,
        Phrase::Password,
        None,
        None,
        Some("eye"),
    );
    let again = field(
        commands,
        state,
        kit,
        Field::PasswordAgain,
        Phrase::PasswordAgain,
        None,
        None,
        Some("eye-again"),
    );
    let passwords = pair(commands, kit, password, Some(again));
    commands.entity(card).add_children(&[names, passwords]);
    if lobby.invite_key_offered() {
        let key = field(
            commands,
            state,
            kit,
            Field::InviteKey,
            Phrase::InviteKey,
            Some(Phrase::InviteKeyShape.text(lang)),
            help(kit, Phrase::InviteKeyHint.text(lang)),
            None,
        );
        if kit.m.frame == Frame::Phone {
            // A phone's form ends above the keyboard: the key and the
            // primary share the last row (§2.7).
            let go = submit(commands, state, kit, Phrase::CreateAccount.text(lang));
            let last = pair(commands, kit, key, Some(go));
            commands.entity(card).add_child(last);
        } else {
            commands.entity(card).add_child(key);
            let go = submit(commands, state, kit, Phrase::CreateAccount.text(lang));
            commands.entity(card).add_child(go);
        }
    } else {
        let go = submit(commands, state, kit, Phrase::CreateAccount.text(lang));
        commands.entity(card).add_child(go);
    }
    if let Some(said) = status(commands, state, kit) {
        commands.entity(card).add_child(said);
    }
}

/// The guest's face: a display name, the key on a closed beta, what a
/// guest is, Play as guest.
pub(super) fn guest(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let head = face_head(
        commands,
        state,
        kit,
        Phrase::FrontGuestTitle.text(lang),
        music_on,
    );
    let line = rule(commands);
    commands.entity(card).add_children(&[head, line]);
    let name = field(
        commands,
        state,
        kit,
        Field::GuestName,
        Phrase::DisplayName,
        Some(Phrase::GuestDefaultName.text(lang)),
        help(kit, Phrase::AccountNameHint.text(lang)),
        None,
    );
    let key = lobby.invite_key_offered().then(|| {
        field(
            commands,
            state,
            kit,
            Field::InviteKey,
            Phrase::InviteKey,
            Some(Phrase::InviteKeyShape.text(lang)),
            help(kit, Phrase::InviteKeyHint.text(lang)),
            None,
        )
    });
    let row = pair(commands, kit, name, key);
    let notice = surfaces::prose(commands, kit, Phrase::GuestNotice.text(lang), true);
    commands.entity(card).add_children(&[row, notice]);
    let go = submit(commands, state, kit, Phrase::FrontGuestTitle.text(lang));
    commands.entity(card).add_child(go);
    if let Some(said) = status(commands, state, kit) {
        commands.entity(card).add_child(said);
    }
}

/// The new-password face (beta.8): an admin reset the password, and the
/// player chooses their own, twice, before anything else. `‹ Back` gives
/// up and returns to the sign-in face.
pub(super) fn new_password(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let head = face_head(
        commands,
        state,
        kit,
        Phrase::ChooseNewPassword.text(lang),
        music_on,
    );
    let line = rule(commands);
    let why = surfaces::prose(commands, kit, Phrase::PasswordWasReset.text(lang), true);
    commands.entity(card).add_children(&[head, line, why]);
    let password = field(
        commands,
        state,
        kit,
        Field::Password,
        Phrase::NewPassword,
        None,
        None,
        Some("eye"),
    );
    let again = field(
        commands,
        state,
        kit,
        Field::PasswordAgain,
        Phrase::PasswordAgain,
        None,
        None,
        Some("eye-again"),
    );
    let passwords = pair(commands, kit, password, Some(again));
    commands.entity(card).add_child(passwords);
    let go = submit(commands, state, kit, Phrase::SetNewPassword.text(lang));
    commands.entity(card).add_child(go);
    if let Some(said) = status(commands, state, kit) {
        commands.entity(card).add_child(said);
    }
}

fn gateway_title_row(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    _music_on: bool,
) -> Entity {
    let lang = state.lobby.lang();
    let title =
        super::door::card_title_lockup(commands, kit, '\u{f1eb}', Phrase::ChooseGateway.text(lang));
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(12.0),
                flex_wrap: FlexWrap::Wrap,
                row_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_child(title);
    head
}

/// The gateway picker: the saved gateways, a separated "Add Gateway" section
/// with address input + Add button, and the status line.
pub(super) fn gateway(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled_to: &Scrolled,
    music_on: bool,
) {
    let lang = state.lobby.lang();
    let head = gateway_title_row(commands, state, kit, music_on);
    let explanation = if state.central_ready() {
        Phrase::GatewayHintCentral
    } else {
        Phrase::GatewayHint
    };
    let hint = surfaces::prose(commands, kit, explanation.text(lang), true);
    commands.entity(card).add_children(&[head, hint]);
    if !state.displayed_gateways().is_empty() {
        let toolbar = super::super::gateway::toolbar(commands, state, kit);
        commands.entity(card).add_child(toolbar);
    }
    let list =
        super::super::gateway::list(commands, state, kit.fonts, field_metrics(kit), scrolled_to);
    commands.entity(card).add_child(list);

    let add_box = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(12.0)),
                row_gap: kit.m.px(8.0),
                margin: UiRect::top(kit.m.px(14.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.13, 0.72)),
            BorderColor::all(tokens::BORDER.with_alpha(0.85)),
            Pickable::IGNORE,
        ))
        .id();

    let address = field(
        commands,
        state,
        kit,
        Field::Gateway,
        Phrase::GatewayAddress,
        Some("https://"),
        None,
        None,
    );
    let busy = if state.central_ready() {
        state.central_admitting()
    } else {
        state.lobby.busy()
    };
    let save = controls::button_with_icon(
        commands,
        kit,
        '\u{f067}',
        Phrase::SaveGateway.text(lang),
        Weight::Primary,
        if !busy && state.adding.is_none() {
            Live::Yes
        } else {
            Live::No("")
        },
        None,
        (
            Press::Front(FrontPress::AddGateway),
            keys::stop(state, "save"),
        ),
    );
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::FlexEnd,
                column_gap: kit.m.px(10.0),
                row_gap: kit.m.px(8.0),
                flex_direction: if kit.m.frame == Frame::Compact {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(address)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_grow = 1.0;
            node.min_width = px_fixed(0.0);
        });
    commands.entity(row).add_children(&[address, save]);
    commands.entity(add_box).add_child(row);
    if !state.central_admitting()
        && let Some(said) = status(commands, state, kit)
    {
        commands.entity(add_box).add_child(said);
    }
    commands.entity(card).add_child(add_box);
}
