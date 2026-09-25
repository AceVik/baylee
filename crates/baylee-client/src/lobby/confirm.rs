//! Destructive actions are named and confirmed in a modal, never on a list row.
#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum Destructive {
    Delete(String),
    Clear(Option<String>),
    /// A saved gateway leaving this device's list, by address.
    ForgetGateway(String),
    /// A guest signing out, which is the end of it (#269). Carried out by
    /// the sign-out itself, not by [`accept`].
    SignOutGuest,
}

pub(super) fn accept(state: &mut LobbyState) -> Option<LobbyRequest> {
    match state.confirmation.take()? {
        Destructive::Delete(id) => {
            let at = state.lobby.decks().iter().position(|d| d.id == id)?;
            state.lobby.delete_deck(at)
        }
        Destructive::Clear(id) => {
            if state.lobby.builder().editing() == id.as_deref() {
                state.lobby.builder_mut().clear_deck();
            }
            None
        }
        Destructive::ForgetGateway(url) => {
            state.forget_gateway(&url);
            None
        }
        Destructive::SignOutGuest => None,
    }
}

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
    let chosen;
    let (phrase, name, said) = match action {
        Destructive::Delete(id) => (
            Phrase::DeleteDeckQuestion,
            state
                .lobby
                .decks()
                .iter()
                .find(|d| &d.id == id)
                .map_or("", |d| d.name.as_str()),
            Phrase::DestructiveHint,
        ),
        Destructive::Clear(_) => (
            Phrase::ClearDeckQuestion,
            state.lobby.builder().name(),
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
        Destructive::SignOutGuest => (
            Phrase::GuestSignOutQuestion,
            state
                .lobby
                .kept_guest()
                .map_or("", |kept| kept.handle.as_str()),
            Phrase::GuestSignOutHint,
        ),
    };
    let (shade, panel) = modal(commands, metrics, Press::CancelDestructive);
    let title = heading(commands, fonts, metrics, &phrase.fill(lang, &[name]));
    let hint = note(commands, fonts, metrics, said.text(lang));
    let actions = row(commands, metrics, true);
    let cancel = button(
        commands,
        fonts,
        metrics,
        Phrase::ActCancel.text(lang),
        Press::CancelDestructive,
        palette::PANEL_LIT,
        true,
    );
    let yes = button(
        commands,
        fonts,
        metrics,
        Phrase::ConfirmOk.text(lang),
        Press::ConfirmDestructive,
        palette::DANGER,
        !state.lobby.busy(),
    );
    commands.entity(actions).add_children(&[cancel, yes]);
    commands.entity(panel).add_children(&[title, hint, actions]);
    commands.entity(shade).add_child(panel);
    commands.entity(root).add_child(shade);
}

/// A confirmation's shade over the whole screen, which cancels when it is
/// tapped, and the panel on it.
fn modal(commands: &mut Commands, metrics: Metrics, cancel: Press) -> (Entity, Entity) {
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
            BackgroundColor(Color::BLACK.with_alpha(0.8)),
            GlobalZIndex(800),
            cancel,
        ))
        .id();
    let panel = super::ui::surface(commands, metrics);
    commands
        .entity(panel)
        .entry::<Node>()
        .and_modify(|mut n| n.max_width = px(520));
    commands.entity(panel).insert((
        Press::PickerNothing,
        BackgroundColor(palette::PANEL.with_alpha(0.98)),
    ));
    (shade, panel)
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
    let (shade, panel) = modal(commands, metrics, Press::CancelAccountDeletion);
    let question = Phrase::DeleteAccountQuestion.fill(lang, &[lobby.account_name()]);
    let title = heading(commands, fonts, metrics, &question);
    let said = if lobby.guest() {
        Phrase::DeleteGuestHint
    } else {
        Phrase::DeleteAccountHint
    };
    let hint = note(commands, fonts, metrics, said.text(lang));
    commands.entity(panel).add_children(&[title, hint]);
    if !lobby.guest() {
        let field = Field::AccountPassword;
        let box_ = text_field(
            commands,
            fonts,
            metrics,
            Phrase::Password.text(lang),
            &FieldLook {
                buffer: lobby.buffer(field),
                focused: lobby.focus() == field,
                mask: Some(super::ui::Masked {
                    field,
                    shown: lobby.showing(field),
                }),
                press: Press::Focus(field),
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
    let actions = row(commands, metrics, true);
    let cancel = button(
        commands,
        fonts,
        metrics,
        Phrase::ActCancel.text(lang),
        Press::CancelAccountDeletion,
        palette::PANEL_LIT,
        true,
    );
    let yes = button(
        commands,
        fonts,
        metrics,
        Phrase::DeleteAccountConfirm.text(lang),
        Press::ConfirmAccountDeletion,
        palette::DANGER,
        !lobby.busy(),
    );
    commands.entity(actions).add_children(&[cancel, yes]);
    commands.entity(panel).add_child(actions);
    commands.entity(shade).add_child(panel);
    commands.entity(root).add_child(shade);
}
