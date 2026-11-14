//! Responsive challenge panels using the lobby's existing controls and typography.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::shellkit::{
    controls::{self, Kit, Live, Weight},
    px_fixed, tokens,
};
use baylee_client_core::challenges::{Confirmation, api};

mod catalogue;
mod detail;
mod home;
mod rewards;
mod tower;
use catalogue::catalogue;
pub(in crate::lobby) use home::home;
use rewards::badges;

pub(in crate::lobby) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let model = &state.challenges.model;
    let lang = state.lobby.lang();
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px_fixed(kit.m.body_max),
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_height: px(0),
                row_gap: px_fixed(kit.m.gap),
                padding: UiRect::all(px_fixed(kit.m.body)),
                overflow: if metrics.frame == Frame::Compact {
                    Overflow::DEFAULT
                } else {
                    Overflow::scroll_y()
                },
                ..default()
            },
            Pickable::default(),
        ))
        .id();
    if metrics.frame != Frame::Compact {
        commands.entity(body).insert((
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Table))),
        ));
    }
    attach_body(commands, root, body, metrics.frame);
    let masthead = panel(commands, body, kit);
    let head = row(commands, masthead, kit);
    heading(
        commands,
        head,
        kit,
        if model.tower.opened {
            Phrase::TowerNav
        } else {
            Phrase::ChallengeNav
        }
        .text(lang),
    );
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(head).add_child(spacer);
    if model.selected.is_some() || model.tower.opened {
        button(
            commands,
            head,
            kit,
            Phrase::Back.text(lang),
            Action::Back,
            model,
            Weight::Secondary,
        );
    }
    button(
        commands,
        head,
        kit,
        Phrase::Refresh.text(lang),
        Action::Refresh,
        model,
        Weight::Ghost,
    );
    if !state.central_ready() {
        label(
            commands,
            body,
            kit,
            Phrase::ChallengeSignIn.text(lang),
            tokens::MUTED,
        );
        return;
    }
    notices(commands, body, kit, lang, state);
    if model.tower.opened {
        tower::draw(commands, body, kit, lang, model);
    } else if model.selected.is_some() {
        if let Some(task) = &model.detail {
            detail::draw(commands, body, kit, lang, model, task);
        }
    } else {
        catalogue(commands, body, kit, lang, model, true);
    }
}

fn notices(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, state: &LobbyState) {
    let model = &state.challenges.model;
    if model.busy {
        label(
            commands,
            body,
            kit,
            Phrase::ChallengeLoading.text(lang),
            tokens::MUTED,
        );
    }
    if model.failed {
        label(
            commands,
            body,
            kit,
            Phrase::ChallengeFailed.text(lang),
            tokens::DANGER,
        );
    }
    if state.challenges.pending_result {
        label(
            commands,
            body,
            kit,
            Phrase::ChallengePendingResult.text(lang),
            tokens::ACCENT,
        );
    }
}

fn attach_body(commands: &mut Commands, root: Entity, body: Entity, frame: Frame) {
    if frame == Frame::Compact {
        commands.entity(root).add_child(body);
    } else {
        let band = crate::lobby::scrollbars::band_when_needed(commands, body, false);
        commands
            .entity(band)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.width = percent(100);
                node.flex_grow = 1.0;
            });
        commands.entity(root).add_child(band);
    }
}

fn words(words: &api::Words, lang: Lang) -> &str {
    if lang == Lang::De {
        &words.de
    } else {
        &words.en
    }
}

fn status(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) {
    let phrase = if task.withdrawn {
        Phrase::ChallengeWithdrawn
    } else if model.now < task.description.period.ends_at {
        Phrase::ChallengeLive
    } else {
        Phrase::ChallengeArchive
    };
    label(
        commands,
        root,
        kit,
        phrase.text(lang),
        if task.withdrawn {
            tokens::DANGER
        } else {
            tokens::MUTED
        },
    );
    label(
        commands,
        root,
        kit,
        &Phrase::ChallengeSchedule.fill(
            lang,
            &[
                &date(task.description.period.starts_at),
                &date(task.description.period.ends_at),
            ],
        ),
        tokens::MUTED,
    );
}

fn date(seconds: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(seconds).map_or_else(
        |_| "—".into(),
        |at| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                at.year(),
                u8::from(at.month()),
                at.day(),
                at.hour(),
                at.minute()
            )
        },
    )
}

fn panel(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let panel = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                padding: UiRect::all(px_fixed(kit.m.pad)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(panel);
    panel
}

fn row(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(row);
    row
}

fn heading(commands: &mut Commands, root: Entity, kit: Kit, text: &str) {
    let heading = crate::shellkit::surfaces::heading(commands, kit, text);
    commands.entity(root).add_child(heading);
}

fn label(commands: &mut Commands, root: Entity, kit: Kit, text: &str, color: Color) {
    let label = commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, kit.m.small),
            TextColor(color),
            Node {
                max_width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(label);
}

fn button(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    text: &str,
    action: Action,
    model: &State,
    weight: Weight,
) -> Entity {
    let lang = if kit.german { Lang::De } else { Lang::En };
    let live = if model.busy {
        Live::No(Phrase::ChallengeLoading.text(lang))
    } else if model.tower.uncertain()
        && matches!(
            action,
            Action::Tower(
                baylee_client_core::tower::Action::Upgrade(_)
                    | baylee_client_core::tower::Action::Cancel
                    | baylee_client_core::tower::Action::Abandon
            )
        )
    {
        Live::No(Phrase::ChallengeFailed.text(lang))
    } else if (model.confirmation.is_some() || model.tower.abandoning)
        && !matches!(
            action,
            Action::Confirm
                | Action::Cancel
                | Action::Back
                | Action::Tower(
                    baylee_client_core::tower::Action::Confirm
                        | baylee_client_core::tower::Action::Cancel
                )
        )
    {
        Live::No(Phrase::ChallengeConfirm.text(lang))
    } else {
        Live::Yes
    };
    let button = controls::button(
        commands,
        kit,
        text,
        weight,
        live,
        None,
        Press::Challenges(action),
    );
    commands.entity(button).insert((
        super::keys::stop(action, model),
        crate::shellkit::focus::Current(
            matches!(action, Action::Board(board) if board == model.board)
                || matches!(action, Action::Badge(kind) if model.selected_badge == Some(kind))
                || matches!(action, Action::Tower(baylee_client_core::tower::Action::Board(board)) if board == model.tower.board)
                || matches!(action, Action::Tower(baylee_client_core::tower::Action::Upgrade(track)) if model.tower.upgrade_preview() == Some(track))
                || matches!(action, Action::Tower(baylee_client_core::tower::Action::Badge(kind)) if model.tower.badge == Some(kind)),
        ),
    ));
    commands.entity(root).add_child(button);
    button
}
