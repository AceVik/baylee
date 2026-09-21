//! Destructive actions are named and confirmed in a modal, never on a list row.
#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum Destructive {
    Delete(String),
    Clear(Option<String>),
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
    let (phrase, name) = match action {
        Destructive::Delete(id) => (
            Phrase::DeleteDeckQuestion,
            state
                .lobby
                .decks()
                .iter()
                .find(|d| &d.id == id)
                .map_or("", |d| d.name.as_str()),
        ),
        Destructive::Clear(_) => (Phrase::ClearDeckQuestion, state.lobby.builder().name()),
    };
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
            Press::CancelDestructive,
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
    let title = heading(commands, fonts, metrics, &phrase.fill(lang, &[name]));
    let hint = note(commands, fonts, metrics, Phrase::DestructiveHint.text(lang));
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
