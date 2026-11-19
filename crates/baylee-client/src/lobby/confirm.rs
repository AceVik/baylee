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
    // Constructed by `lobby::links`; `baylee:` deep links are native-only.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
    let panel = modal(
        commands,
        root,
        metrics,
        Press::Shared(SharedPress::CancelDestructive),
        DialogKind::Destructive,
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
        DialogKind::Destructive,
    );
    let hint = dialog_body(commands, fonts, metrics, said.text(lang));
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
    let can_confirm = !state.lobby.busy();
    let yes = split_button(
        commands,
        fonts,
        metrics,
        yes_phrase.text(lang),
        Press::Shared(SharedPress::ConfirmDestructive),
        SplitTone::Danger,
        can_confirm,
    );
    if !can_confirm {
        commands.entity(yes).insert((
            crate::shellkit::controls::Disabled,
            super::hint::HoverHint(Phrase::VeilTalking.text(lang).to_string()),
        ));
    }
    let actions = dialog_actions(commands, &[cancel, yes]);
    commands.entity(panel).add_children(&[head, hint, actions]);
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
    let panel = modal(
        commands,
        root,
        metrics,
        Press::Shared(SharedPress::CancelDestructive),
        DialogKind::Proposal,
    );
    let head = dialog_header(
        commands,
        fonts,
        metrics,
        &Phrase::LinkGatewayQuestion.fill(lang, &[url]),
        None,
        DialogKind::Proposal,
    );
    let hint = dialog_body(commands, fonts, metrics, Phrase::LinkGatewayHint.text(lang));
    let joins = if state.central_ready() {
        Phrase::LinkGatewayJoinsAccount
    } else {
        Phrase::LinkGatewayJoinsDevice
    };
    let list = note(commands, fonts, metrics, joins.text(lang));
    let not_now = split_button(
        commands,
        fonts,
        metrics,
        Phrase::LinkNotNow.text(lang),
        Press::Shared(SharedPress::CancelDestructive),
        SplitTone::Neutral,
        true,
    );
    let can_add = !state.lobby.busy() && state.adding.is_none();
    let add = split_button(
        commands,
        fonts,
        metrics,
        Phrase::LinkGatewayAdd.text(lang),
        Press::Shared(SharedPress::ConfirmDestructive),
        SplitTone::Accent,
        can_add,
    );
    if !can_add {
        commands.entity(add).insert((
            crate::shellkit::controls::Disabled,
            super::hint::HoverHint(Phrase::VeilTalking.text(lang).to_string()),
        ));
    }
    let actions = dialog_actions(commands, &[not_now, add]);
    commands
        .entity(panel)
        .add_children(&[head, hint, list, actions]);
}

/// The sign-in from baylee.app (`central::pairing`), over Central's sign-in:
/// while the page is asked, the code it shows beside its Approve; then whom
/// the session signs in as, with Cancel and Sign in. Cancel is what `Esc`
/// and a tap beside the panel do, at every stage; no key says yes. Nothing
/// in it comes from the link: the code is the client's own, the name
/// Central's.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn draw_pairing(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    use super::central::pairing::Stage;
    let Some(pairing) = state.pairing() else {
        return;
    };
    let lang = state.lobby.lang();
    let cancel_press = Press::Shared(SharedPress::PairCancel);
    let panel = modal(commands, root, metrics, cancel_press, DialogKind::Proposal);
    let (title, body, code) = match &pairing.stage {
        Stage::Claiming => (
            Phrase::PairTitle.fill(lang, &[]),
            Phrase::PairClaiming,
            None,
        ),
        Stage::Finishing => (
            Phrase::PairTitle.fill(lang, &[]),
            Phrase::PairFinishing,
            None,
        ),
        Stage::Waiting { code } => (
            Phrase::PairWaitingTitle.fill(lang, &[]),
            Phrase::PairWaitingHint,
            Some(code.as_str()),
        ),
        Stage::Asking { who } => (
            Phrase::PairQuestion.fill(lang, &[who.as_str()]),
            Phrase::PairQuestionHint,
            None,
        ),
    };
    let mut kids = vec![
        dialog_header(commands, fonts, metrics, &title, None, DialogKind::Proposal),
        dialog_body(commands, fonts, metrics, body.text(lang)),
    ];
    if let Some(code) = code {
        kids.push(code_line(commands, fonts, metrics, code));
    }
    let mut buttons = vec![split_button(
        commands,
        fonts,
        metrics,
        Phrase::ActCancel.text(lang),
        cancel_press,
        SplitTone::Neutral,
        true,
    )];
    if matches!(pairing.stage, Stage::Asking { .. }) {
        buttons.push(split_button(
            commands,
            fonts,
            metrics,
            Phrase::SignIn.text(lang),
            Press::Shared(SharedPress::PairSignIn),
            SplitTone::Accent,
            true,
        ));
    }
    kids.push(dialog_actions(commands, &buttons));
    commands.entity(panel).add_children(&kids);
}

/// The pairing's comparison code, large and centred: the one thing the
/// player compares with the page.
#[cfg(not(target_arch = "wasm32"))]
fn code_line(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, code: &str) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(14), px(10)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(code),
            tf_bold(fonts, metrics.head * 1.6),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id()
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
    let panel = modal(
        commands,
        root,
        metrics,
        Press::Shared(SharedPress::CloseRefusal),
        DialogKind::Warning,
    );
    let name = baylee_client_core::deckcheck::format_name(&refusal.list, lang);
    let title = dialog_header(
        commands,
        fonts,
        metrics,
        &Phrase::RefusedTitle.fill(lang, &[name]),
        None,
        DialogKind::Warning,
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
                padding: UiRect::axes(px(14), px(12)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            super::Scrollable(super::List::Check),
            ScrollPosition::default(),
        ))
        .id();
    for line in state.lobby.refusal_lines() {
        let said = note(commands, fonts, metrics, &line);
        commands.entity(reasons).add_child(said);
    }
    commands.entity(reasons).insert(Pickable::default());
    let reasons_band = super::scrollbars::band_when_needed(
        commands,
        reasons,
        metrics.frame == super::Frame::Compact,
    );
    let close = split_button(
        commands,
        fonts,
        metrics,
        Phrase::ShellClose.text(lang),
        Press::Shared(SharedPress::CloseRefusal),
        SplitTone::Accent,
        true,
    );
    let actions = dialog_actions(commands, &[close]);
    commands
        .entity(panel)
        .add_children(&[title, reasons_band, actions]);
}

#[derive(Clone, Copy)]
enum DialogKind {
    Destructive,
    Proposal,
    Warning,
}

#[derive(Clone, Copy)]
enum SplitTone {
    Neutral,
    Danger,
    Accent,
}

/// A confirmation's shade over the whole screen (`surfaces::sheet`), which
/// cancels when tapped, and the opaque panel on it.
fn modal(
    commands: &mut Commands,
    root: Entity,
    metrics: Metrics,
    cancel: Press,
    kind: DialogKind,
) -> Entity {
    use bevy::ui::{ColorStop, InterpolationColorSpace};
    let top_rim = match kind {
        DialogKind::Destructive => tokens::DANGER.with_alpha(0.54),
        DialogKind::Proposal => Color::srgba(0.86, 0.78, 0.62, 0.48),
        DialogKind::Warning => tokens::GOLD.with_alpha(0.54),
    };
    let side_rim = match kind {
        DialogKind::Destructive => tokens::DANGER.with_alpha(0.34),
        DialogKind::Proposal => Color::srgba(0.58, 0.66, 0.76, 0.34),
        DialogKind::Warning => tokens::GOLD.with_alpha(0.34),
    };
    let panel = commands
        .spawn((
            Role::Opaque,
            Node {
                width: percent(100),
                max_width: px(460),
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                padding: UiRect::all(px(22)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BackgroundGradient::from(
                LinearGradient::to_bottom(vec![
                    ColorStop::percent(Color::srgba(0.080, 0.130, 0.200, 0.64), 0.0),
                    ColorStop::percent(Color::srgba(0.035, 0.068, 0.115, 0.58), 48.0),
                    ColorStop::percent(Color::srgba(0.025, 0.050, 0.088, 0.68), 100.0),
                ])
                .in_color_space(InterpolationColorSpace::Srgba),
            ),
            BorderColor {
                top: top_rim,
                right: side_rim,
                bottom: side_rim,
                left: side_rim,
            },
            crate::shellkit::surfaces::elevation_card(),
            Press::Shared(SharedPress::PickerNothing),
        ))
        .id();
    let shade = crate::shellkit::surfaces::sheet(commands, panel);
    commands
        .entity(shade)
        .insert((GlobalZIndex(tokens::z::SHEET + 2), cancel))
        .entry::<Node>()
        .and_modify(move |mut node| node.padding = UiRect::all(px(metrics.pad)));
    commands.entity(root).add_child(shade);
    panel
}

fn dialog_header(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    title_text: &str,
    subtitle: Option<&str>,
    kind: DialogKind,
) -> Entity {
    let (badge_bg, badge_border, badge_fg, glyph) = match kind {
        DialogKind::Destructive => (
            tokens::DANGER.with_alpha(0.16),
            tokens::DANGER.with_alpha(0.44),
            tokens::DANGER,
            "\u{f2ed}",
        ),
        DialogKind::Proposal => (
            tokens::PRIMARY.with_alpha(0.55),
            tokens::PRIMARY_EDGE.with_alpha(0.65),
            tokens::ACCENT,
            "\u{f0c1}",
        ),
        DialogKind::Warning => (
            tokens::GOLD.with_alpha(0.16),
            tokens::GOLD.with_alpha(0.46),
            tokens::GOLD,
            "\u{f071}",
        ),
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
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
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
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(col).add_child(title);
    if let Some(sub) = subtitle {
        let sub_node = commands
            .spawn((
                Text::new(sub),
                tf(fonts, (metrics.small - 1.0).max(11.0)),
                TextColor(tokens::MUTED),
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
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.018, 0.038, 0.068, 0.46)),
            BorderColor::all(Color::srgba(0.64, 0.70, 0.78, 0.26)),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(text),
            tf(fonts, metrics.text),
            TextColor(tokens::INK.with_alpha(0.90)),
            Pickable::IGNORE,
        ))
        .id()
}

fn dialog_actions(commands: &mut Commands, buttons: &[Entity]) -> Entity {
    let row = commands
        .spawn((
            crate::shellkit::surfaces::ModalFoot,
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
        .id();
    commands.entity(row).add_children(buttons);
    row
}

fn split_tone_colors(tone: SplitTone, enabled: bool) -> (Color, Color, Color, Color, Color) {
    let fg = if enabled {
        tokens::INK
    } else {
        tokens::DISABLED
    };
    match tone {
        SplitTone::Neutral => (
            tokens::CONTROL,
            tokens::SELECTED,
            tokens::BORDER,
            tokens::ACCENT,
            fg,
        ),
        SplitTone::Danger => (
            if enabled {
                tokens::DANGER
            } else {
                tokens::CONTROL
            },
            Color::srgba(0.92, 0.28, 0.36, 0.98),
            if enabled {
                tokens::DANGER.with_alpha(0.75)
            } else {
                tokens::BORDER
            },
            tokens::INK,
            fg,
        ),
        SplitTone::Accent => (
            if enabled {
                tokens::PRIMARY
            } else {
                tokens::CONTROL
            },
            tokens::SELECTED,
            if enabled {
                tokens::PRIMARY_EDGE
            } else {
                tokens::BORDER
            },
            tokens::ACCENT,
            fg,
        ),
    }
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
    let (bg, hover_bg, border, hover_border, fg) = split_tone_colors(tone, enabled);
    let mut node = commands.spawn((
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(100),
            height: px(metrics.tap.max(40.0)),
            padding: UiRect::axes(px(14), px(8)),
            column_gap: px(8),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px_fixed(1.0)),
            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
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
            crate::shellkit::surfaces::elevation_control(),
            Feel::tinting_to(bg, hover_bg),
            FeelBorder {
                rest: border,
                hot: hover_border,
                press: hover_border,
            },
            crate::ambience::FeelShadow,
            Sheen,
        ));
    } else {
        node.insert((
            Pickable::default(),
            bevy::picking::hover::PickingInteraction::default(),
            crate::shellkit::surfaces::elevation_control_disabled(),
            crate::shellkit::controls::Disabled,
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
    let panel = modal(
        commands,
        root,
        metrics,
        Press::Settings(SettingsPress::CancelAccountDeletion),
        DialogKind::Destructive,
    );
    let question = Phrase::DeleteAccountQuestion.fill(lang, &[lobby.account_name()]);
    let title = dialog_header(
        commands,
        fonts,
        metrics,
        &question,
        None,
        DialogKind::Destructive,
    );
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
                lead: Some(super::field::lead_glyph::LOCK),
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
                TextColor(tokens::DANGER),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(panel).add_child(refusal);
    }
    let cancel = split_button(
        commands,
        fonts,
        metrics,
        Phrase::ActCancel.text(lang),
        Press::Settings(SettingsPress::CancelAccountDeletion),
        SplitTone::Neutral,
        true,
    );
    let can_delete = !lobby.busy();
    let yes = split_button(
        commands,
        fonts,
        metrics,
        Phrase::DeleteAccountConfirm.text(lang),
        Press::Settings(SettingsPress::ConfirmAccountDeletion),
        SplitTone::Danger,
        can_delete,
    );
    if !can_delete {
        commands.entity(yes).insert((
            crate::shellkit::controls::Disabled,
            super::hint::HoverHint(Phrase::VeilTalking.text(lang).to_string()),
        ));
    }
    let actions = dialog_actions(commands, &[cancel, yes]);
    commands.entity(panel).add_child(actions);
}
