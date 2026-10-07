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

/// The lobby's own field metrics, read off the kit's (the text step and
/// the size class): the fields are the lobby's editor, sized by the shell.
pub(super) fn field_metrics(kit: Kit) -> Metrics {
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

/// Whether the face sets its fields two to a row: every class but the
/// smallest desktop window (§2.7: a phone's are a two-column grid that
/// ends above the keyboard line).
fn two_columns(kit: Kit) -> bool {
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
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(10.0),
                flex_direction: if two_columns(kit) {
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
        typed: which,
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

/// The status line (a refusal, verbatim; a note), only while it says
/// something: no reserved hole under the button (A12).
fn status(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Option<Entity> {
    let said = state.lobby.status();
    if said.is_empty() {
        return None;
    }
    let ink = match state.lobby.tone() {
        Tone::Refusal => tokens::DANGER,
        Tone::Note => tokens::MUTED,
    };
    Some(
        commands
            .spawn((
                Text::new(said),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id(),
    )
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

#[allow(clippy::too_many_lines)] // one head: the way back, the name over its address, the version
/// The sign-in face's head: `‹` back to the gateways, the reach dot, the
/// gateway's name (and the closed-beta badge), its address under it; on the
/// right its version with the verdict, and this client's (A2, A3: the name
/// at row size, the version whole and right-aligned).
fn gateway_head(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let words =
        super::super::gateway::row_words(&state.gateway, state.probes.get(&state.gateway), lang);
    let phone = kit.m.frame == Frame::Phone;
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(10.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
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
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                max_width: Val::Percent(45.0),
                row_gap: kit.m.px(2.0),
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
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(words.ink),
            TextLayout::justify(Justify::Right),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(right).add_child(version);
    if !phone {
        let ours = controls::label(
            commands,
            kit,
            &Phrase::FrontThisClient.fill(lang, &[baylee_build::VERSION]),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(right).add_child(ours);
    }
    commands.entity(head).add_children(&[back, names, right]);
    head
}

/// A second face's head: `‹ Back`, the face's title, and the gateway as a
/// chip on the right.
fn face_head(commands: &mut Commands, state: &LobbyState, kit: Kit, title: &str) -> Entity {
    let lang = state.lobby.lang();
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(12.0),
                align_items: AlignItems::Center,
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
    let words = commands
        .spawn((
            Text::new(title),
            crate::hud::tf(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Node {
                flex_grow: 1.0,
                min_width: px_fixed(0.0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let gateway =
        super::super::gateway::row_words(&state.gateway, state.probes.get(&state.gateway), lang);
    let chip = commands
        .spawn((
            Node {
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let reach = dot(commands, kit, gateway.reach);
    let name = controls::label(commands, kit, &gateway.title, kit.m.small, tokens::INK);
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
    let live = if lobby.busy() || !state.gateway_selected {
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

/// The sign-in face: the gateway, username and password, Sign in; Create
/// account and Play as guest under a rule, the key hint under them.
pub(super) fn sign_in(commands: &mut Commands, card: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let head = gateway_head(commands, state, kit);
    let line = rule(commands);
    commands.entity(card).add_children(&[head, line]);
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
    let guest = lobby.guest_offered().then(|| {
        let text = lobby.kept_guest().map_or_else(
            || Phrase::FrontGuestTitle.text(lang).to_string(),
            |kept| Phrase::ContinueAsGuest.fill(lang, &[&kept.handle]),
        );
        controls::wide_button(
            commands,
            kit,
            &text,
            Weight::Secondary,
            if lobby.busy() || !state.gateway_selected {
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
    let doors: Vec<Entity> = create.into_iter().chain(guest).collect();
    if !doors.is_empty() {
        let line = rule(commands);
        let row = pair(commands, kit, doors[0], doors.get(1).copied());
        commands.entity(card).add_children(&[line, row]);
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
}

/// The create-account face: username and display name, the password twice,
/// the key on a closed beta, Create account.
pub(super) fn create(commands: &mut Commands, card: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let head = face_head(commands, state, kit, Phrase::CreateAccount.text(lang));
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
pub(super) fn guest(commands: &mut Commands, card: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let head = face_head(commands, state, kit, Phrase::FrontGuestTitle.text(lang));
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

/// The gateway picker: the saved gateways, the address with Check / Save
/// beside it, and the status line.
pub(super) fn gateway(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled_to: &Scrolled,
) {
    let lang = state.lobby.lang();
    let title = commands
        .spawn((
            Text::new(Phrase::ChooseGateway.text(lang)),
            crate::hud::tf(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let hint = surfaces::prose(commands, kit, Phrase::GatewayHint.text(lang), true);
    commands.entity(card).add_children(&[title, hint]);
    if !state.gateways.is_empty() {
        let list = super::super::gateway::list(
            commands,
            state,
            kit.fonts,
            field_metrics(kit),
            scrolled_to,
        );
        commands.entity(card).add_child(list);
    }
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
    let save = controls::button(
        commands,
        kit,
        Phrase::SaveGateway.text(lang),
        Weight::Primary,
        if !state.lobby.busy() && state.adding.is_none() {
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
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(10.0),
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
    commands.entity(card).add_child(row);
    if let Some(said) = status(commands, state, kit) {
        commands.entity(card).add_child(said);
    }
}
