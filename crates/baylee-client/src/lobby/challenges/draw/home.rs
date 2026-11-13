//! The signed-in home: challenges first, connection details in their popover.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(in crate::lobby) fn home(
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
                min_height: px_fixed(0.0),
                row_gap: px_fixed(kit.m.gap),
                padding: UiRect::all(px_fixed(kit.m.body)),
                overflow: if metrics.frame == Frame::Compact {
                    Overflow::DEFAULT
                } else {
                    Overflow::scroll_y()
                },
                ..default()
            },
            // Wheel input must also land in the gaps between controls.
            Pickable::default(),
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
    heading(commands, head, kit, Phrase::ChallengeHome.text(lang));
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(head).add_child(spacer);
    button(
        commands,
        head,
        kit,
        Phrase::Refresh.text(lang),
        Action::Refresh,
        model,
        Weight::Ghost,
    );
    label(
        commands,
        body,
        kit,
        Phrase::ChallengeHomeLead.text(lang),
        tokens::MUTED,
    );
    if model.failed {
        label(
            commands,
            body,
            kit,
            Phrase::ChallengeFailed.text(lang),
            tokens::DANGER,
        );
    } else if !model.loaded {
        label(
            commands,
            body,
            kit,
            Phrase::ChallengeLoading.text(lang),
            tokens::MUTED,
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
    catalogue(commands, body, kit, lang, model, false);
    let foot = row(commands, body, kit);
    label(
        commands,
        foot,
        kit,
        Phrase::ChallengeTogether.text(lang),
        tokens::MUTED,
    );
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(foot).add_child(spacer);
    let multiplayer = controls::button(
        commands,
        kit,
        Phrase::ChallengeMultiplayer.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Hub(HubPress::Tab(Hub::Tables)),
    );
    crate::lobby::orders::stop(
        commands,
        multiplayer,
        &crate::lobby::orders::CHALLENGES,
        "multiplayer",
    );
    commands.entity(foot).add_child(multiplayer);
}
