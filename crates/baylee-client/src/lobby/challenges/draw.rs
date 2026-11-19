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
mod layout;
pub(in crate::lobby) mod presentation;
mod rewards;
mod tower;
use catalogue::catalogue;
use rewards::badges;

pub(in crate::lobby) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) {
    if state.echoes.opened {
        super::super::echoes::draw(commands, root, state, metrics, kit, scrolled);
        return;
    }
    let model = &state.challenges.model;
    let lang = state.lobby.lang();
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px_fixed(kit.m.body_max),
                align_self: AlignSelf::Center,
                // Centred in the scroll band's row too, where `align_self`
                // is the vertical axis: the page stands under the header's
                // middle as the Tower's does, not against the left edge.
                margin: UiRect::horizontal(Val::Auto),
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
    masthead(commands, body, kit, lang, model);
    if !state.can_enter_challenges() {
        layout::notice(
            commands,
            body,
            kit,
            crate::hud::glyph::LOCK,
            Phrase::ChallengeSignIn.text(lang),
            tokens::INK,
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

/// The page's head: where the player is, the way back, and a re-read.
///
/// The way back leads, as a breadcrumb does; the title names the page and,
/// on the calendar, one sentence says what the page is for.
fn masthead(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let head = layout::section(commands, body, kit);
    let line = row(commands, head, kit);
    commands.entity(line).entry::<Node>().and_modify(|mut n| {
        n.flex_wrap = FlexWrap::NoWrap;
    });
    let inner = model.selected.is_some() || model.tower.opened;
    if inner {
        button(
            commands,
            line,
            kit,
            Phrase::Back.text(lang),
            Action::Back,
            model,
            Weight::Secondary,
        );
    }
    let words = commands
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
    commands.entity(line).add_child(words);
    let name = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.space(1),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(words).add_child(name);
    let mark = commands
        .spawn((
            Text::new(crate::hud::glyph::CROWN.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::GOLD),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(name).add_child(mark);
    layout::title(
        commands,
        name,
        kit,
        if model.tower.opened {
            Phrase::TowerNav
        } else {
            Phrase::ChallengeNav
        }
        .text(lang),
        kit.m.h1,
    );
    if !inner && !layout::tight(kit) {
        layout::prose(
            commands,
            words,
            kit,
            Phrase::ChallengeHomeLead.text(lang),
            tokens::MUTED,
        );
    }
    button(
        commands,
        line,
        kit,
        Phrase::Refresh.text(lang),
        Action::Refresh,
        model,
        Weight::Ghost,
    );
}

fn notices(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, state: &LobbyState) {
    let model = &state.challenges.model;
    for (shown, icon, phrase, ink) in [
        (
            // A minute's re-read of a page that stands says nothing: a plate
            // coming and going would move everything under it.
            model.busy && !model.loaded,
            crate::hud::glyph::HOURGLASS,
            Phrase::ChallengeLoading,
            tokens::INK,
        ),
        (
            model.failed,
            crate::hud::glyph::INFO,
            Phrase::ChallengeFailed,
            tokens::DANGER,
        ),
        (
            state.challenges.pending_result,
            crate::hud::glyph::CLOCK,
            Phrase::ChallengePendingResult,
            tokens::ACCENT,
        ),
    ] {
        if shown {
            layout::notice(commands, body, kit, icon, phrase.text(lang), ink);
        }
    }
}

pub(in crate::lobby) fn attach_body(
    commands: &mut Commands,
    root: Entity,
    body: Entity,
    frame: Frame,
) {
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

pub(in crate::lobby) fn panel(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
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

pub(in crate::lobby) fn row(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
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

pub(in crate::lobby) fn heading(commands: &mut Commands, root: Entity, kit: Kit, text: &str) {
    let heading = crate::shellkit::surfaces::heading(commands, kit, text);
    commands.entity(root).add_child(heading);
}

pub(in crate::lobby) fn label(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    text: &str,
    color: Color,
) {
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
    let lang = kit.lang();
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
    let button = controls::button_with_icon(
        commands,
        kit,
        presentation::action_icon(action),
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

/// An expedition's card on the calendar, shaped as a current task's: the
/// landscape, the name, one sentence. Its owner adds what it offers under it.
pub(in crate::lobby) fn entry_card(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    art: crate::challenge_art::Scene,
    title: &str,
    lead: &str,
) -> Entity {
    let card = layout::section(commands, root, kit);
    layout::banner(
        commands,
        card,
        kit,
        art,
        if layout::tight(kit) { 64.0 } else { 92.0 },
    );
    layout::title(commands, card, kit, title, kit.m.head);
    layout::prose(commands, card, kit, lead, tokens::MUTED);
    card
}
