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
use catalogue::catalogue;

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
            Pickable::IGNORE,
        ))
        .id();
    if metrics.frame != Frame::Compact {
        commands.entity(body).insert((
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Table))),
        ));
    }
    commands.entity(root).add_child(body);
    let head = row(commands, body, kit);
    heading(commands, head, kit, Phrase::ChallengeNav.text(lang));
    button(
        commands,
        head,
        kit,
        Phrase::Refresh.text(lang),
        Action::Refresh,
        model,
        Weight::Secondary,
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
    if model.selected.is_some() {
        button(
            commands,
            body,
            kit,
            Phrase::Back.text(lang),
            Action::Back,
            model,
            Weight::Ghost,
        );
        if let Some(task) = &model.detail {
            detail::draw(commands, body, kit, lang, model, task);
        }
    } else {
        catalogue(commands, body, kit, lang, model);
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

fn badges(commands: &mut Commands, root: Entity, kit: Kit, lang: Lang, model: &State) {
    let panel = panel(commands, root, kit);
    heading(commands, panel, kit, Phrase::ChallengeBadges.text(lang));
    label(
        commands,
        panel,
        kit,
        Phrase::ChallengeBadgeRules.text(lang),
        tokens::MUTED,
    );
    for badge in &model.badges {
        let name = match badge.kind {
            api::BadgeKind::Daily => Phrase::ChallengeBadgeDaily,
            api::BadgeKind::Weekly => Phrase::ChallengeBadgeWeekly,
            api::BadgeKind::Tactician => Phrase::ChallengeBadgeTactician,
            api::BadgeKind::FirstTry => Phrase::ChallengeBadgeFirst,
            api::BadgeKind::Regular => Phrase::ChallengeBadgeRegular,
        };
        let tier = match badge.tier {
            None => Phrase::ChallengeNotEarned,
            Some(api::Tier::Bronze) => Phrase::ChallengeBronze,
            Some(api::Tier::Silver) => Phrase::ChallengeSilver,
            Some(api::Tier::Gold) => Phrase::ChallengeGold,
            Some(api::Tier::Platinum) => Phrase::ChallengePlatinum,
            Some(api::Tier::Obsidian) => Phrase::ChallengeObsidian,
        };
        label(
            commands,
            panel,
            kit,
            &format!("{} · {}", name.text(lang), tier.text(lang)),
            tokens::INK,
        );
        let progress = badge.next.map_or_else(
            || Phrase::ChallengeBadgeMax.fill(lang, &[&badge.count.to_string()]),
            |next| {
                Phrase::ChallengeBadgeProgress
                    .fill(lang, &[&badge.count.to_string(), &next.to_string()])
            },
        );
        label(commands, panel, kit, &progress, tokens::MUTED);
        let track = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: px(5),
                    ..default()
                },
                BackgroundColor(tokens::CONTROL),
                Pickable::IGNORE,
            ))
            .id();
        let share = badge.next.map_or(1.0, |next| {
            (f64::from(badge.count) / f64::from(next)).min(1.0)
        });
        let fill = commands
            .spawn((
                Node {
                    width: percent((100.0 * share) as f32),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(tokens::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(fill);
        commands.entity(panel).add_child(track);
    }
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
            BackgroundColor(tokens::PANEL),
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
    } else if model.confirmation.is_some()
        && !matches!(action, Action::Confirm | Action::Cancel | Action::Back)
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
            matches!(action, Action::Board(board) if board == model.board),
        ),
    ));
    commands.entity(root).add_child(button);
    button
}
