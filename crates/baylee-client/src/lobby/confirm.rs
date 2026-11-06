//! Destructive actions are named and confirmed in a modal, never on a list
//! row; so is what a `baylee:` link proposes, which changes the player's list
//! on a stranger's word (`links`).
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::ambience::{Feel, FeelBorder};
use crate::hud::tf_bold;
use crate::shellkit::{Role, px_fixed, sheen::Sheen, tokens};

#[derive(Clone, Debug)]
pub(crate) enum Destructive {
    Clear(Option<String>),
    /// Delete the deck currently open in the deckbuilder.
    DeleteBuildDeck(Option<String>),
    /// A saved gateway leaving this device's list, by address.
    ForgetGateway(String),
    /// A guest signing out, which is the end of it (#269). Carried out by
    /// the sign-out itself, not by [`accept`]. `to_gateways` is the same end
    /// by way of Switch gateway: the list follows, and Central stays signed
    /// in (`clicks::to_the_gateways`).
    SignOutGuest {
        to_gateways: bool,
    },
    /// The host leaving their table, which passes it on or closes it
    /// (DESIGN-v5 §5 "Leaving"), by game id.
    LeaveHosting(String),
    /// A gateway a `baylee:` link proposes, by its origin: not destructive,
    /// but asked the same way, with "Not now" as what `Esc` and a tap beside
    /// the panel do. Carried out by the press itself, which has the mailbox
    /// the address is asked through.
    AddGateway(String),
}

impl Destructive {
    /// What kind of question this is, without what it names (dev-control's
    /// `/state`).
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Destructive::Clear(_) => "clear-deck",
            Destructive::DeleteBuildDeck(_) => "delete-deck",
            Destructive::ForgetGateway(_) => "forget-gateway",
            Destructive::SignOutGuest { .. } => "sign-out-guest",
            Destructive::LeaveHosting(_) => "leave-hosting",
            Destructive::AddGateway(_) => "add-gateway",
        }
    }
}

pub(super) fn accept(state: &mut LobbyState) -> Option<LobbyRequest> {
    match state.confirmation.take()? {
        Destructive::Clear(id) => {
            if state.lobby.builder().editing() == id.as_deref() {
                state.lobby.builder_mut().clear_deck();
            }
            None
        }
        Destructive::DeleteBuildDeck(id) => {
            // A deck whose first save landed under the question has an id now.
            let deck = state.lobby.builder();
            let visit = deck.visit();
            let id = id.or_else(|| deck.editing().map(str::to_owned));
            state.deck_saves.throw_away(visit);
            state.drafts.forget(&[id.as_deref()]);
            state.lobby.builder_mut().discard_edits();
            state.confirm_leave = false;
            state.hub = Hub::Decks;
            let refresh = state.lobby.close_builder();
            if let Some(id) = id {
                if let Some(index) = state.lobby.decks().iter().position(|d| d.id == id) {
                    let _ = state.lobby.stage_delete(index);
                    state.undo = Some(super::decks::Undo {
                        kind: super::decks::UndoKind::Delete,
                        left: super::decks::UNDO_SECS,
                    });
                    refresh
                } else {
                    Some(LobbyRequest::DeleteDeck { deck_id: id })
                }
            } else {
                refresh
            }
        }
        Destructive::ForgetGateway(url) => {
            state.forget_gateway(&url);
            None
        }
        Destructive::SignOutGuest { .. } | Destructive::AddGateway(_) => None,
        Destructive::LeaveHosting(game) => state.lobby.leave_table(&game),
    }
}

/// The handle of the guest a question is about.
fn guest_handle(state: &LobbyState) -> &str {
    state
        .lobby
        .kept_guest()
        .map_or("", |kept| kept.handle.as_str())
}

#[allow(clippy::too_many_lines)]
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    let Some(action) = &state.confirmation else {
        return;
    };
    let lang = state.lobby.lang();
    if let Destructive::AddGateway(url) = action {
        draw_proposal(commands, root, state, fonts, metrics, url);
        return;
    }
    let mut chosen = String::new();
    let deck_title = {
        let trimmed = state.lobby.builder().name().trim();
        if trimmed.is_empty() {
            Phrase::BuildUntitled.text(lang)
        } else {
            trimmed
        }
    };
    let (phrase, name, said) = match action {
        Destructive::Clear(_) => (
            Phrase::ClearDeckQuestion,
            deck_title,
            Phrase::DestructiveHint,
        ),
        Destructive::DeleteBuildDeck(_) => (
            Phrase::DeleteDeckQuestion,
            deck_title,
            Phrase::DestructiveHint,
        ),
        Destructive::ForgetGateway(url) => {
            chosen = super::gateway::title_of(state, url);
            (
                Phrase::ForgetGatewayQuestion,
                chosen.as_str(),
                Phrase::ForgetGatewayHint,
            )
        }
        Destructive::SignOutGuest { to_gateways: false } => (
            Phrase::GuestSignOutQuestion,
            guest_handle(state),
            Phrase::GuestSignOutHint,
        ),
        Destructive::SignOutGuest { to_gateways: true } => (
            Phrase::GuestSwitchGatewayQuestion,
            "",
            Phrase::GuestSwitchGatewayHint,
        ),
        Destructive::LeaveHosting(game) => (
            Phrase::LeaveHostingQuestion,
            state
                .lobby
                .games()
                .iter()
                .find(|g| &g.id == game)
                .map_or("", |g| g.name.as_str()),
            Phrase::LeaveHostingHint,
        ),
        Destructive::AddGateway(_) => unreachable!("drawn by `draw_proposal`"),
    };
    let (shade, panel) = modal(
        commands,
        metrics,
        Press::Shared(SharedPress::CancelDestructive),
        true,
    );
    let subtitle = match action {
        Destructive::ForgetGateway(url) if chosen != *url => Some(url.as_str()),
        Destructive::SignOutGuest { to_gateways: true } => Some(guest_handle(state)),
        _ => None,
    };
    let head = dialog_header(
        commands,
        fonts,
        metrics,
        &phrase.fill(lang, &[name]),
        subtitle,
        true,
    );
    let hint = dialog_body(commands, fonts, metrics, said.text(lang));
    let actions = action_row(commands);
    let cancel = split_button(
        commands,
        fonts,
        metrics,
        Phrase::ActCancel.text(lang),
        Press::Shared(SharedPress::CancelDestructive),
        SplitTone::Neutral,
        true,
    );
    let yes_phrase = match action {
        Destructive::ForgetGateway(_) => Phrase::SeatRemove,
        Destructive::DeleteBuildDeck(_) => Phrase::Delete,
        Destructive::SignOutGuest { to_gateways: true } => Phrase::ShellSwitchGateway,
        _ => Phrase::ConfirmOk,
    };
    let yes = split_button(
        commands,
        fonts,
        metrics,
        yes_phrase.text(lang),
        Press::Shared(SharedPress::ConfirmDestructive),
        SplitTone::Danger,
        !state.lobby.busy(),
    );
    commands.entity(actions).add_children(&[cancel, yes]);
    commands.entity(panel).add_children(&[head, hint, actions]);
    commands.entity(shade).add_child(panel);
    commands.entity(root).add_child(shade);
}

/// A link's gateway, asked about: the address in full as it would be saved,
/// what a gateway is and is not given, which list it joins; "Not now" and
/// "Add gateway". Nothing in it comes from the link but the address.
fn draw_proposal(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    url: &str,
) {
    let lang = state.lobby.lang();
    let (shade, panel) = modal(
        commands,
        metrics,
        Press::Shared(SharedPress::CancelDestructive),
        false,
    );
    let head = dialog_header(
        commands,
        fonts,
        metrics,
        &Phrase::LinkGatewayQuestion.fill(lang, &[url]),
        None,
        false,
    );
    let hint = dialog_body(commands, fonts, metrics, Phrase::LinkGatewayHint.text(lang));
    let joins = if state.central_ready() {
        Phrase::LinkGatewayJoinsAccount
    } else {
        Phrase::LinkGatewayJoinsDevice
    };
    let list = note(commands, fonts, metrics, joins.text(lang));
    let actions = action_row(commands);
    let not_now = split_button(
        commands,
        fonts,
        metrics,
        Phrase::LinkNotNow.text(lang),
        Press::Shared(SharedPress::CancelDestructive),
        SplitTone::Neutral,
        true,
    );
    let add = split_button(
        commands,
        fonts,
        metrics,
        Phrase::LinkGatewayAdd.text(lang),
        Press::Shared(SharedPress::ConfirmDestructive),
        SplitTone::Accent,
        !state.lobby.busy() && state.adding.is_none(),
    );
    commands.entity(actions).add_children(&[not_now, add]);
    commands
        .entity(panel)
        .add_children(&[head, hint, list, actions]);
    commands.entity(shade).add_child(panel);
    commands.entity(root).add_child(shade);
}

/// Why a table's list turned a deck away, a line a reason. It stands over
/// whatever screen the deck was brought from, because the door may be a
/// room's, the Play screen's or an invite's.
pub(super) fn draw_refusal(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    let Some(refusal) = state.lobby.refusal() else {
        return;
    };
    let lang = state.lobby.lang();
    let (shade, panel) = modal(
        commands,
        metrics,
        Press::Shared(SharedPress::CloseRefusal),
        false,
    );
    let name = baylee_client_core::deckcheck::format_name(&refusal.list, lang);
    let title = heading(
        commands,
        fonts,
        metrics,
        &Phrase::RefusedTitle.fill(lang, &[name]),
    );
    // A hundred-card deck can bring a hundred reasons: they scroll between
    // the heading and the way out.
    let reasons = commands
        .spawn((
            Node {
                width: percent(100),
                max_height: Val::Vh(62.0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            super::Scrollable(super::List::Check),
            ScrollPosition::default(),
        ))
        .id();
    for line in state.lobby.refusal_lines() {
        let said = note(commands, fonts, metrics, &line);
        commands.entity(reasons).add_child(said);
    }
    let mut kids = vec![title, reasons];
    let actions = row(commands, metrics, true);
    let close = button(
        commands,
        fonts,
        metrics,
        Phrase::ShellClose.text(lang),
        Press::Shared(SharedPress::CloseRefusal),
        palette::ACCENT,
        true,
    );
    commands.entity(actions).add_child(close);
    kids.push(actions);
    commands.entity(panel).add_children(&kids);
    commands.entity(shade).add_child(panel);
    commands.entity(root).add_child(shade);
}

#[derive(Clone, Copy)]
enum SplitTone {
    Neutral,
    Danger,
    Accent,
}

/// A confirmation's shade over the whole screen, which cancels when it is
/// tapped, and the panel on it.
fn modal(
    commands: &mut Commands,
    metrics: Metrics,
    cancel: Press,
    destructive: bool,
) -> (Entity, Entity) {
    let shade = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(metrics.pad)),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.78)),
            GlobalZIndex(800),
            cancel,
        ))
        .id();
    let border_col = if destructive {
        Color::srgba(0.82, 0.30, 0.38, 0.42)
    } else {
        Color::srgba(0.36, 0.56, 0.88, 0.42)
    };
    let panel = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px(460),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                padding: UiRect::all(px(22)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px(16)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.07, 0.11, 0.98)),
            BorderColor::all(border_col),
            BoxShadow::new(
                Color::BLACK.with_alpha(0.68),
                Val::ZERO,
                px_fixed(16.0),
                px_fixed(2.0),
                px_fixed(36.0),
            ),
            Press::Shared(SharedPress::PickerNothing),
        ))
        .id();
    (shade, panel)
}

fn dialog_header(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    title_text: &str,
    subtitle: Option<&str>,
    destructive: bool,
) -> Entity {
    let (badge_bg, badge_border, badge_fg, glyph) = if destructive {
        (
            Color::srgba(0.86, 0.22, 0.30, 0.16),
            Color::srgba(0.94, 0.34, 0.42, 0.38),
            Color::srgb(0.96, 0.42, 0.48),
            "\u{f2ed}",
        )
    } else {
        (
            Color::srgba(0.24, 0.52, 0.92, 0.18),
            Color::srgba(0.38, 0.66, 0.98, 0.40),
            Color::srgb(0.56, 0.80, 0.99),
            "\u{f0c1}",
        )
    };
    let head = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let badge = commands
        .spawn((
            Node {
                width: px(38),
                height: px(38),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(badge_bg),
            BorderColor::all(badge_border),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(glyph),
            crate::hud::icon_tf(fonts, (metrics.head - 1.0).max(14.0)),
            TextColor(badge_fg),
            Pickable::IGNORE,
        ))
        .id();
    let col = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(title_text),
            tf_bold(fonts, metrics.head),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(col).add_child(title);
    if let Some(sub) = subtitle {
        let sub_node = commands
            .spawn((
                Text::new(sub),
                tf(fonts, (metrics.small - 1.0).max(11.0)),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(col).add_child(sub_node);
    }
    commands.entity(head).add_children(&[badge, col]);
    head
}

fn dialog_body(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, text: &str) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                padding: UiRect::axes(px(14), px(12)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.09, 0.11, 0.16, 0.72)),
            BorderColor::all(Color::srgba(0.32, 0.38, 0.54, 0.24)),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(text),
            tf(fonts, metrics.text),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

fn action_row(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                margin: UiRect::top(px(4)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

fn split_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
    tone: SplitTone,
    enabled: bool,
) -> Entity {
    let (bg, hover_bg, border, hover_border, fg) = match tone {
        SplitTone::Neutral => (
            Color::srgba(0.13, 0.15, 0.22, 0.94),
            Color::srgba(0.19, 0.22, 0.31, 0.98),
            Color::srgba(0.42, 0.48, 0.68, 0.36),
            Color::srgba(0.56, 0.64, 0.86, 0.62),
            palette::INK,
        ),
        SplitTone::Danger => (
            if enabled {
                palette::DANGER
            } else {
                tokens::CONTROL
            },
            Color::srgba(0.92, 0.28, 0.36, 0.98),
            if enabled {
                Color::srgba(0.98, 0.46, 0.52, 0.58)
            } else {
                tokens::BORDER
            },
            Color::srgba(1.0, 0.58, 0.64, 0.85),
            if enabled {
                palette::INK
            } else {
                palette::MUTED
            },
        ),
        SplitTone::Accent => (
            if enabled {
                palette::ACCENT
            } else {
                tokens::CONTROL
            },
            Color::srgba(0.32, 0.62, 0.96, 0.98),
            if enabled {
                Color::srgba(0.45, 0.72, 0.98, 0.58)
            } else {
                tokens::BORDER
            },
            Color::srgba(0.60, 0.82, 1.0, 0.85),
            if enabled {
                palette::INK
            } else {
                palette::MUTED
            },
        ),
    };
    let mut node = commands.spawn((
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            width: percent(50),
            height: px(40),
            padding: UiRect::axes(px(14), px(8)),
            column_gap: px(8),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px_fixed(1.0)),
            border_radius: BorderRadius::all(px(10)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border),
    ));
    if enabled {
        node.insert((
            press,
            Role::Button,
            Feel::tinting_to(bg, hover_bg),
            FeelBorder {
                rest: border,
                hot: hover_border,
                press: hover_border,
            },
            Sheen,
        ));
    }
    let id = node
        .with_child((
            Text::new(label),
            tf_bold(fonts, metrics.text),
            TextColor(fg),
            Pickable::IGNORE,
        ))
        .id();
    super::button_style::icon(commands, fonts, id, press, metrics.small);
    id
}

/// The account's deletion (#292), over the settings screen: the question,
/// what goes, the password box for an account that has one, what the
/// gateway said to the last try, and the two answers.
///
/// Drawn from the lobby's own state ([`Lobby::deleting_account`]) rather
/// than from [`Destructive`], because it types into a field and answers the
/// gateway, and both of those are the lobby's.
pub(super) fn draw_deletion(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    let lobby = &state.lobby;
    let Some(deletion) = lobby.deleting_account() else {
        return;
    };
    let lang = lobby.lang();
    let (shade, panel) = modal(
        commands,
        metrics,
        Press::Settings(SettingsPress::CancelAccountDeletion),
        true,
    );
    let question = Phrase::DeleteAccountQuestion.fill(lang, &[lobby.account_name()]);
    let title = dialog_header(commands, fonts, metrics, &question, None, true);
    let said = if lobby.guest() {
        Phrase::DeleteGuestHint
    } else if lobby.central_bound() {
        Phrase::DeleteGatewayDataHint
    } else {
        Phrase::DeleteAccountHint
    };
    let hint = dialog_body(commands, fonts, metrics, said.text(lang));
    commands.entity(panel).add_children(&[title, hint]);
    if lobby.deletion_needs_password() {
        let field = Field::AccountPassword;
        let box_ = text_field(
            commands,
            fonts,
            metrics,
            Phrase::Password.text(lang),
            &FieldLook {
                buffer: lobby.buffer(field),
                focused: lobby.focus() == field,
                mask: Some(super::Masked {
                    field: Some(field),
                    shown: lobby.showing(field),
                }),
                press: Press::Shared(SharedPress::Focus(field)),
                lead: None,
                hint: None,
                tail: None,
            },
        );
        commands.entity(panel).add_child(box_);
    }
    if let Some(refusal) = &deletion.refusal {
        let refusal = commands
            .spawn((
                Text::new(refusal.clone()),
                tf(fonts, metrics.small),
                TextColor(palette::DANGER),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(panel).add_child(refusal);
    }
    let actions = action_row(commands);
    let cancel = split_button(
        commands,
        fonts,
        metrics,
        Phrase::ActCancel.text(lang),
        Press::Settings(SettingsPress::CancelAccountDeletion),
        SplitTone::Neutral,
        true,
    );
    let yes = split_button(
        commands,
        fonts,
        metrics,
        Phrase::DeleteAccountConfirm.text(lang),
        Press::Settings(SettingsPress::ConfirmAccountDeletion),
        SplitTone::Danger,
        !lobby.busy(),
    );
    commands.entity(actions).add_children(&[cancel, yes]);
    commands.entity(panel).add_child(actions);
    commands.entity(shade).add_child(panel);
    commands.entity(root).add_child(shade);
}
